// Copyright (c) 2019 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Websocket [handshake]s.
//!
//! [handshake]: https://tools.ietf.org/html/rfc6455#section-4

pub mod client;
#[cfg(feature = "http")]
pub mod http;
pub mod server;

use crate::extension::{Extension, Param};
use base64::Engine;
use bytes::BytesMut;
use sha1::{Digest, Sha1};
use std::{fmt, io, str};

pub use client::{Client, ServerResponse};
pub use server::{ClientRequest, Server};

// Defined in RFC 6455 and used to generate the `Sec-WebSocket-Accept` header
// in the server handshake response.
const KEY: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

// How many HTTP headers do we support during parsing?
const MAX_NUM_HEADERS: usize = 32;

// Some HTTP headers we need to check during parsing.
const SEC_WEBSOCKET_EXTENSIONS: &str = "Sec-WebSocket-Extensions";
const SEC_WEBSOCKET_PROTOCOL: &str = "Sec-WebSocket-Protocol";

/// Check a set of headers contains a specific one.
fn expect_ascii_header(headers: &[httparse::Header], name: &str, ours: &str) -> Result<(), Error> {
	enum State {
		Init,  // Start state
		Name,  // Header name found
		Match, // Header value matches
	}

	headers
		.iter()
		.filter(|h| h.name.eq_ignore_ascii_case(name))
		.fold(Ok(State::Init), |result, header| {
			if let Ok(State::Match) = result {
				return result;
			}
			if str::from_utf8(header.value)?.split(',').any(|v| v.trim().eq_ignore_ascii_case(ours)) {
				return Ok(State::Match);
			}
			Ok(State::Name)
		})
		.and_then(|state| match state {
			State::Init => Err(Error::HeaderNotFound(name.into())),
			State::Name => Err(Error::UnexpectedHeader(name.into())),
			State::Match => Ok(()),
		})
}

/// Pick the first header with the given name and apply the given closure to it.
fn with_first_header<'a, F, R>(headers: &[httparse::Header<'a>], name: &str, f: F) -> Result<R, Error>
where
	F: Fn(&'a [u8]) -> Result<R, Error>,
{
	if let Some(h) = headers.iter().find(|h| h.name.eq_ignore_ascii_case(name)) {
		f(h.value)
	} else {
		Err(Error::HeaderNotFound(name.into()))
	}
}

// Configure all extensions with parsed parameters.
fn configure_extensions(extensions: &mut [Box<dyn Extension + Send>], line: &str) -> Result<(), Error> {
	for e in line.split(',') {
		let mut ext_parts = e.split(';');
		if let Some(name) = ext_parts.next() {
			let name = name.trim();
			if let Some(ext) = extensions.iter_mut().find(|x| x.name().eq_ignore_ascii_case(name)) {
				let mut params = Vec::new();
				for p in ext_parts {
					let mut key_value = p.split('=');
					if let Some(key) = key_value.next().map(str::trim) {
						let val = key_value.next().map(|v| v.trim().trim_matches('"'));
						let mut p = Param::new(key);
						p.set_value(val);
						params.push(p)
					}
				}
				ext.configure(&params).map_err(Error::Extension)?
			}
		}
	}
	Ok(())
}

// Write all extensions to the given buffer.
fn append_extensions<'a, I>(extensions: I, bytes: &mut BytesMut)
where
	I: IntoIterator<Item = &'a Box<dyn Extension + Send>>,
{
	let mut iter = extensions.into_iter().peekable();

	if iter.peek().is_some() {
		bytes.extend_from_slice(b"\r\nSec-WebSocket-Extensions: ")
	}

	append_extension_header_value(iter, bytes)
}

// Write the extension header value to the given buffer.
fn append_extension_header_value<'a, I>(mut extensions_iter: std::iter::Peekable<I>, bytes: &mut BytesMut)
where
	I: Iterator<Item = &'a Box<dyn Extension + Send>>,
{
	while let Some(e) = extensions_iter.next() {
		bytes.extend_from_slice(e.name().as_bytes());
		for p in e.params() {
			bytes.extend_from_slice(b"; ");
			bytes.extend_from_slice(p.name().as_bytes());
			if let Some(v) = p.value() {
				bytes.extend_from_slice(b"=");
				bytes.extend_from_slice(v.as_bytes())
			}
		}
		if extensions_iter.peek().is_some() {
			bytes.extend_from_slice(b", ")
		}
	}
}

// This function takes a 16 byte key (base64 encoded, and so 24 bytes of input) that is expected via
// the `Sec-WebSocket-Key` header during a websocket handshake, and writes the response that's expected
// to be handed back in the response header `Sec-WebSocket-Accept`.
//
// The response is a base64 encoding of a 160bit hash. base64 encoding uses 1 ascii character per 6 bits,
// and 160 / 6 = 26.66 characters. The output is padded with '=' to the nearest 4 characters, so we need 28
// bytes in total for all of the characters.
//
// See https://datatracker.ietf.org/doc/html/rfc6455#section-1.3 for more information on this.
fn generate_accept_key<'k>(key_base64: &WebSocketKey) -> [u8; 28] {
	let mut digest = Sha1::new();
	digest.update(key_base64);
	digest.update(KEY);
	let d = digest.finalize();

	let mut output_buf = [0; 28];
	let n = base64::engine::general_purpose::STANDARD
		.encode_slice(d, &mut output_buf)
		.expect("encoding to base64 is exactly 28 bytes; qed");
	debug_assert_eq!(n, 28, "encoding to base64 should be exactly 28 bytes");
	output_buf
}

/// Enumeration of possible handshake errors.
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
	/// An I/O error has been encountered.
	Io(io::Error),
	/// An HTTP version =/= 1.1 was encountered.
	UnsupportedHttpVersion,
	/// An incomplete HTTP request.
	IncompleteHttpRequest,
	/// The value of the `Sec-WebSocket-Key` header is of unexpected length.
	SecWebSocketKeyInvalidLength(usize),
	/// The handshake request was not a GET request.
	InvalidRequestMethod,
	/// An HTTP header has not been present.
	HeaderNotFound(String),
	/// An HTTP header value was not expected.
	UnexpectedHeader(String),
	/// The Sec-WebSocket-Accept header value did not match.
	InvalidSecWebSocketAccept,
	/// The server returned an extension we did not ask for.
	UnsolicitedExtension,
	/// The server returned a protocol we did not ask for.
	UnsolicitedProtocol,
	/// An extension produced an error while encoding or decoding.
	Extension(crate::BoxedError),
	/// The HTTP entity could not be parsed successfully.
	Http(crate::BoxedError),
	/// UTF-8 decoding failed.
	Utf8(str::Utf8Error),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::Io(e) => write!(f, "i/o error: {}", e),
			Error::UnsupportedHttpVersion => f.write_str("http version was not 1.1"),
			Error::IncompleteHttpRequest => f.write_str("http request was incomplete"),
			Error::SecWebSocketKeyInvalidLength(len) => {
				write!(f, "Sec-WebSocket-Key header was {} bytes long, expected 24", len)
			}
			Error::InvalidRequestMethod => f.write_str("handshake was not a GET request"),
			Error::HeaderNotFound(name) => write!(f, "header {} not found", name),
			Error::UnexpectedHeader(name) => write!(f, "header {} had an unexpected value", name),
			Error::InvalidSecWebSocketAccept => f.write_str("websocket key mismatch"),
			Error::UnsolicitedExtension => f.write_str("unsolicited extension returned"),
			Error::UnsolicitedProtocol => f.write_str("unsolicited protocol returned"),
			Error::Extension(e) => write!(f, "extension error: {}", e),
			Error::Http(e) => write!(f, "http parser error: {}", e),
			Error::Utf8(e) => write!(f, "utf-8 decoding error: {}", e),
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Error::Io(e) => Some(e),
			Error::Extension(e) => Some(&**e),
			Error::Http(e) => Some(&**e),
			Error::Utf8(e) => Some(e),
			Error::UnsupportedHttpVersion
			| Error::IncompleteHttpRequest
			| Error::SecWebSocketKeyInvalidLength(_)
			| Error::InvalidRequestMethod
			| Error::HeaderNotFound(_)
			| Error::UnexpectedHeader(_)
			| Error::InvalidSecWebSocketAccept
			| Error::UnsolicitedExtension
			| Error::UnsolicitedProtocol => None,
		}
	}
}

impl From<io::Error> for Error {
	fn from(e: io::Error) -> Self {
		Error::Io(e)
	}
}

impl From<str::Utf8Error> for Error {
	fn from(e: str::Utf8Error) -> Self {
		Error::Utf8(e)
	}
}

/// Owned value of the `Sec-WebSocket-Key` header.
///
/// Per [RFC 6455](https://datatracker.ietf.org/doc/html/rfc6455#section-4.1):
///
/// ```text
/// (...) The value of this header field MUST be a
/// nonce consisting of a randomly selected 16-byte value that has
/// been base64-encoded (see Section 4 of [RFC4648]). (...)
/// ```
///
/// Base64 encoding of the nonce produces 24 ASCII bytes, padding included.
pub type WebSocketKey = [u8; 24];

#[cfg(test)]
mod tests {
	use super::expect_ascii_header;

	#[test]
	fn header_match() {
		let headers = &[
			httparse::Header { name: "foo", value: b"a,b,c,d" },
			httparse::Header { name: "foo", value: b"x" },
			httparse::Header { name: "foo", value: b"y, z, a" },
			httparse::Header { name: "bar", value: b"xxx" },
			httparse::Header { name: "bar", value: b"sdfsdf 423 42 424" },
			httparse::Header { name: "baz", value: b"123" },
		];

		assert!(expect_ascii_header(headers, "foo", "a").is_ok());
		assert!(expect_ascii_header(headers, "foo", "b").is_ok());
		assert!(expect_ascii_header(headers, "foo", "c").is_ok());
		assert!(expect_ascii_header(headers, "foo", "d").is_ok());
		assert!(expect_ascii_header(headers, "foo", "x").is_ok());
		assert!(expect_ascii_header(headers, "foo", "y").is_ok());
		assert!(expect_ascii_header(headers, "foo", "z").is_ok());
		assert!(expect_ascii_header(headers, "foo", "a").is_ok());
		assert!(expect_ascii_header(headers, "bar", "xxx").is_ok());
		assert!(expect_ascii_header(headers, "bar", "sdfsdf 423 42 424").is_ok());
		assert!(expect_ascii_header(headers, "baz", "123").is_ok());
		assert!(expect_ascii_header(headers, "baz", "???").is_err());
		assert!(expect_ascii_header(headers, "???", "x").is_err());
	}
}
