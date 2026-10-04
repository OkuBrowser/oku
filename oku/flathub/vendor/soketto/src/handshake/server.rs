// Copyright (c) 2019 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Websocket server [handshake].
//!
//! [handshake]: https://tools.ietf.org/html/rfc6455#section-4

use super::{
	append_extensions, configure_extensions, expect_ascii_header, with_first_header, Error, WebSocketKey,
	MAX_NUM_HEADERS, SEC_WEBSOCKET_EXTENSIONS, SEC_WEBSOCKET_PROTOCOL,
};
use crate::connection::{self, Mode};
use crate::extension::Extension;
use bytes::BytesMut;
use futures::prelude::*;
use std::{mem, str};

// Most HTTP servers default to 8KB limit on headers
const MAX_HEADERS_SIZE: usize = 8 * 1024;
const BLOCK_SIZE: usize = 8 * 1024;

/// Websocket handshake server.
#[derive(Debug)]
pub struct Server<'a, T> {
	socket: T,
	/// Protocols the server supports.
	protocols: Vec<&'a str>,
	/// Extensions the server supports.
	extensions: Vec<Box<dyn Extension + Send>>,
	/// Encoding/decoding buffer.
	buffer: BytesMut,
}

impl<'a, T: AsyncRead + AsyncWrite + Unpin> Server<'a, T> {
	/// Create a new server handshake.
	pub fn new(socket: T) -> Self {
		Server { socket, protocols: Vec::new(), extensions: Vec::new(), buffer: BytesMut::new() }
	}

	/// Override the buffer to use for request/response handling.
	pub fn set_buffer(&mut self, b: BytesMut) -> &mut Self {
		self.buffer = b;
		self
	}

	/// Extract the buffer.
	pub fn take_buffer(&mut self) -> BytesMut {
		mem::take(&mut self.buffer)
	}

	/// Add a protocol the server supports.
	pub fn add_protocol(&mut self, p: &'a str) -> &mut Self {
		self.protocols.push(p);
		self
	}

	/// Add an extension the server supports.
	pub fn add_extension(&mut self, e: Box<dyn Extension + Send>) -> &mut Self {
		self.extensions.push(e);
		self
	}

	/// Get back all extensions.
	pub fn drain_extensions(&mut self) -> impl Iterator<Item = Box<dyn Extension + Send>> + '_ {
		self.extensions.drain(..)
	}

	/// Await an incoming client handshake request.
	pub async fn receive_request(&mut self) -> Result<ClientRequest<'_>, Error> {
		self.buffer.clear();

		let mut skip = 0;

		loop {
			crate::read(&mut self.socket, &mut self.buffer, BLOCK_SIZE).await?;

			let limit = std::cmp::min(self.buffer.len(), MAX_HEADERS_SIZE);

			// We don't expect body, so can search for the CRLF headers tail from
			// the end of the buffer.
			if self.buffer[skip..limit].windows(4).rev().any(|w| w == b"\r\n\r\n") {
				break;
			}

			// Give up if we've reached the limit. We could emit a specific error here,
			// but httparse will produce meaningful error for us regardless.
			if limit == MAX_HEADERS_SIZE {
				break;
			}

			// Skip bytes that did not contain CRLF in the next iteration.
			// If we only read a partial CRLF sequence, we would miss it if we skipped the full buffer
			// length, hence backing off the full 4 bytes.
			skip = self.buffer.len().saturating_sub(4);
		}

		self.decode_request()
	}

	/// Respond to the client.
	pub async fn send_response(&mut self, r: &Response<'_>) -> Result<(), Error> {
		self.buffer.clear();
		self.encode_response(r);
		self.socket.write_all(&self.buffer).await?;
		self.socket.flush().await?;
		self.buffer.clear();
		Ok(())
	}

	/// Turn this handshake into a [`connection::Builder`].
	pub fn into_builder(mut self) -> connection::Builder<T> {
		let mut builder = connection::Builder::new(self.socket, Mode::Server);
		builder.set_buffer(self.buffer);
		builder.add_extensions(self.extensions.drain(..));
		builder
	}

	/// Get out the inner socket of the server.
	pub fn into_inner(self) -> T {
		self.socket
	}

	// Decode client handshake request.
	fn decode_request(&mut self) -> Result<ClientRequest, Error> {
		let mut header_buf = [httparse::EMPTY_HEADER; MAX_NUM_HEADERS];
		let mut request = httparse::Request::new(&mut header_buf);

		match request.parse(self.buffer.as_ref()) {
			Ok(httparse::Status::Complete(_)) => (),
			Ok(httparse::Status::Partial) => return Err(Error::IncompleteHttpRequest),
			Err(e) => return Err(Error::Http(Box::new(e))),
		};
		if request.method != Some("GET") {
			return Err(Error::InvalidRequestMethod);
		}
		if request.version != Some(1) {
			return Err(Error::UnsupportedHttpVersion);
		}

		let host = with_first_header(&request.headers, "Host", Ok)?;

		expect_ascii_header(request.headers, "Upgrade", "websocket")?;
		expect_ascii_header(request.headers, "Connection", "upgrade")?;
		expect_ascii_header(request.headers, "Sec-WebSocket-Version", "13")?;

		let origin =
			request.headers.iter().find_map(
				|h| {
					if h.name.eq_ignore_ascii_case("Origin") {
						Some(h.value)
					} else {
						None
					}
				},
			);
		let headers = RequestHeaders { host, origin };

		let ws_key = with_first_header(&request.headers, "Sec-WebSocket-Key", |k| {
			WebSocketKey::try_from(k).map_err(|_| Error::SecWebSocketKeyInvalidLength(k.len()))
		})?;

		for h in request.headers.iter().filter(|h| h.name.eq_ignore_ascii_case(SEC_WEBSOCKET_EXTENSIONS)) {
			configure_extensions(&mut self.extensions, std::str::from_utf8(h.value)?)?
		}

		let mut protocols = Vec::new();
		for p in request.headers.iter().filter(|h| h.name.eq_ignore_ascii_case(SEC_WEBSOCKET_PROTOCOL)) {
			if let Some(&p) = self.protocols.iter().find(|x| x.as_bytes() == p.value) {
				protocols.push(p)
			}
		}

		let path = request.path.unwrap_or("/");

		Ok(ClientRequest { ws_key, protocols, path, headers })
	}

	// Encode server handshake response.
	fn encode_response(&mut self, response: &Response<'_>) {
		match response {
			Response::Accept { key, protocol } => {
				let accept_value = super::generate_accept_key(&key);
				self.buffer.extend_from_slice(
					concat![
						"HTTP/1.1 101 Switching Protocols",
						"\r\nServer: soketto-",
						env!("CARGO_PKG_VERSION"),
						"\r\nUpgrade: websocket",
						"\r\nConnection: upgrade",
						"\r\nSec-WebSocket-Accept: ",
					]
					.as_bytes(),
				);
				self.buffer.extend_from_slice(&accept_value);
				if let Some(p) = protocol {
					self.buffer.extend_from_slice(b"\r\nSec-WebSocket-Protocol: ");
					self.buffer.extend_from_slice(p.as_bytes())
				}
				append_extensions(self.extensions.iter().filter(|e| e.is_enabled()), &mut self.buffer);
				self.buffer.extend_from_slice(b"\r\n\r\n")
			}
			Response::Reject { status_code } => {
				self.buffer.extend_from_slice(b"HTTP/1.1 ");
				let (_, reason) = if let Ok(i) = STATUSCODES.binary_search_by_key(status_code, |(n, _)| *n) {
					STATUSCODES[i]
				} else {
					(500, "500 Internal Server Error")
				};
				self.buffer.extend_from_slice(reason.as_bytes());
				self.buffer.extend_from_slice(b"\r\n\r\n")
			}
		}
	}
}

/// Handshake request received from the client.
#[derive(Debug)]
pub struct ClientRequest<'a> {
	ws_key: WebSocketKey,
	protocols: Vec<&'a str>,
	path: &'a str,
	headers: RequestHeaders<'a>,
}

/// Select HTTP headers sent by the client.
#[derive(Debug, Copy, Clone)]
pub struct RequestHeaders<'a> {
	/// The [`Host`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Host) header.
	pub host: &'a [u8],
	/// The [`Origin`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Origin) header, if provided.
	pub origin: Option<&'a [u8]>,
}

impl<'a> ClientRequest<'a> {
	/// The `Sec-WebSocket-Key` header nonce value.
	pub fn key(&self) -> WebSocketKey {
		self.ws_key
	}

	/// The protocols the client is proposing.
	pub fn protocols(&self) -> impl Iterator<Item = &str> {
		self.protocols.iter().cloned()
	}

	/// The path the client is requesting.
	pub fn path(&self) -> &str {
		self.path
	}

	/// Select HTTP headers sent by the client.
	pub fn headers(&self) -> RequestHeaders {
		self.headers
	}
}

/// Handshake response the server sends back to the client.
#[derive(Debug)]
pub enum Response<'a> {
	/// The server accepts the handshake request.
	Accept { key: WebSocketKey, protocol: Option<&'a str> },
	/// The server rejects the handshake request.
	Reject { status_code: u16 },
}

/// Known status codes and their reason phrases.
const STATUSCODES: &[(u16, &str)] = &[
	(100, "100 Continue"),
	(101, "101 Switching Protocols"),
	(102, "102 Processing"),
	(200, "200 OK"),
	(201, "201 Created"),
	(202, "202 Accepted"),
	(203, "203 Non Authoritative Information"),
	(204, "204 No Content"),
	(205, "205 Reset Content"),
	(206, "206 Partial Content"),
	(207, "207 Multi-Status"),
	(208, "208 Already Reported"),
	(226, "226 IM Used"),
	(300, "300 Multiple Choices"),
	(301, "301 Moved Permanently"),
	(302, "302 Found"),
	(303, "303 See Other"),
	(304, "304 Not Modified"),
	(305, "305 Use Proxy"),
	(307, "307 Temporary Redirect"),
	(308, "308 Permanent Redirect"),
	(400, "400 Bad Request"),
	(401, "401 Unauthorized"),
	(402, "402 Payment Required"),
	(403, "403 Forbidden"),
	(404, "404 Not Found"),
	(405, "405 Method Not Allowed"),
	(406, "406 Not Acceptable"),
	(407, "407 Proxy Authentication Required"),
	(408, "408 Request Timeout"),
	(409, "409 Conflict"),
	(410, "410 Gone"),
	(411, "411 Length Required"),
	(412, "412 Precondition Failed"),
	(413, "413 Payload Too Large"),
	(414, "414 URI Too Long"),
	(415, "415 Unsupported Media Type"),
	(416, "416 Range Not Satisfiable"),
	(417, "417 Expectation Failed"),
	(418, "418 I'm a teapot"),
	(421, "421 Misdirected Request"),
	(422, "422 Unprocessable Entity"),
	(423, "423 Locked"),
	(424, "424 Failed Dependency"),
	(426, "426 Upgrade Required"),
	(428, "428 Precondition Required"),
	(429, "429 Too Many Requests"),
	(431, "431 Request Header Fields Too Large"),
	(451, "451 Unavailable For Legal Reasons"),
	(500, "500 Internal Server Error"),
	(501, "501 Not Implemented"),
	(502, "502 Bad Gateway"),
	(503, "503 Service Unavailable"),
	(504, "504 Gateway Timeout"),
	(505, "505 HTTP Version Not Supported"),
	(506, "506 Variant Also Negotiates"),
	(507, "507 Insufficient Storage"),
	(508, "508 Loop Detected"),
	(510, "510 Not Extended"),
	(511, "511 Network Authentication Required"),
];
