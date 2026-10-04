// Copyright (c) 2019 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Websocket client [handshake].
//!
//! [handshake]: https://tools.ietf.org/html/rfc6455#section-4

use super::{
	append_extensions, configure_extensions, expect_ascii_header, with_first_header, Error, WebSocketKey, KEY,
	MAX_NUM_HEADERS, SEC_WEBSOCKET_EXTENSIONS, SEC_WEBSOCKET_PROTOCOL,
};
use crate::connection::{self, Mode};
use crate::{extension::Extension, Parsing};
use base64::Engine;
use bytes::{Buf, BytesMut};
use futures::prelude::*;
use sha1::{Digest, Sha1};
use std::{mem, str};

pub use httparse::Header;

const BLOCK_SIZE: usize = 8 * 1024;

/// Websocket client handshake.
#[derive(Debug)]
pub struct Client<'a, T> {
	/// The underlying async I/O resource.
	socket: T,
	/// The HTTP host to send the handshake to.
	host: &'a str,
	/// The HTTP host resource.
	resource: &'a str,
	/// The HTTP headers.
	headers: &'a [Header<'a>],
	/// A buffer holding the base-64 encoded request nonce.
	nonce: WebSocketKey,
	/// The protocols to include in the handshake.
	protocols: Vec<&'a str>,
	/// The extensions the client wishes to include in the request.
	extensions: Vec<Box<dyn Extension + Send>>,
	/// Encoding/decoding buffer.
	buffer: BytesMut,
}

impl<'a, T: AsyncRead + AsyncWrite + Unpin> Client<'a, T> {
	/// Create a new client handshake for some host and resource.
	pub fn new(socket: T, host: &'a str, resource: &'a str) -> Self {
		Client {
			socket,
			host,
			resource,
			headers: &[],
			nonce: [0; 24],
			protocols: Vec::new(),
			extensions: Vec::new(),
			buffer: BytesMut::new(),
		}
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

	/// Set connection headers to a slice. These headers are not checked for validity,
	/// the caller of this method is responsible for verification as well as avoiding
	/// conflicts with internally set headers.
	pub fn set_headers(&mut self, h: &'a [Header]) -> &mut Self {
		self.headers = h;
		self
	}

	/// Add a protocol to be included in the handshake.
	pub fn add_protocol(&mut self, p: &'a str) -> &mut Self {
		self.protocols.push(p);
		self
	}

	/// Add an extension to be included in the handshake.
	pub fn add_extension(&mut self, e: Box<dyn Extension + Send>) -> &mut Self {
		self.extensions.push(e);
		self
	}

	/// Get back all extensions.
	pub fn drain_extensions(&mut self) -> impl Iterator<Item = Box<dyn Extension + Send>> + '_ {
		self.extensions.drain(..)
	}

	/// Initiate client handshake request to server and get back the response.
	pub async fn handshake(&mut self) -> Result<ServerResponse, Error> {
		self.buffer.clear();
		self.encode_request();
		self.socket.write_all(&self.buffer).await?;
		self.socket.flush().await?;
		self.buffer.clear();

		loop {
			crate::read(&mut self.socket, &mut self.buffer, BLOCK_SIZE).await?;
			if let Parsing::Done { value, offset } = self.decode_response()? {
				self.buffer.advance(offset);
				return Ok(value);
			}
		}
	}

	/// Turn this handshake into a [`connection::Builder`].
	pub fn into_builder(mut self) -> connection::Builder<T> {
		let mut builder = connection::Builder::new(self.socket, Mode::Client);
		builder.set_buffer(self.buffer);
		builder.add_extensions(self.extensions.drain(..));
		builder
	}

	/// Get out the inner socket of the client.
	pub fn into_inner(self) -> T {
		self.socket
	}

	/// Encode the client handshake as a request, ready to be sent to the server.
	fn encode_request(&mut self) {
		let nonce: [u8; 16] = rand::random();
		base64::engine::general_purpose::STANDARD
			.encode_slice(nonce, &mut self.nonce)
			.expect("encoding to base64 is exactly 16 bytes; qed");
		self.buffer.extend_from_slice(b"GET ");
		self.buffer.extend_from_slice(self.resource.as_bytes());
		self.buffer.extend_from_slice(b" HTTP/1.1");
		self.buffer.extend_from_slice(b"\r\nHost: ");
		self.buffer.extend_from_slice(self.host.as_bytes());
		self.buffer.extend_from_slice(b"\r\nUpgrade: websocket\r\nConnection: Upgrade");
		self.buffer.extend_from_slice(b"\r\nSec-WebSocket-Key: ");
		self.buffer.extend_from_slice(&self.nonce);
		self.headers.iter().for_each(|h| {
			self.buffer.extend_from_slice(b"\r\n");
			self.buffer.extend_from_slice(h.name.as_bytes());
			self.buffer.extend_from_slice(b": ");
			self.buffer.extend_from_slice(h.value);
		});
		if let Some((last, prefix)) = self.protocols.split_last() {
			self.buffer.extend_from_slice(b"\r\nSec-WebSocket-Protocol: ");
			for p in prefix {
				self.buffer.extend_from_slice(p.as_bytes());
				self.buffer.extend_from_slice(b",")
			}
			self.buffer.extend_from_slice(last.as_bytes())
		}
		append_extensions(&self.extensions, &mut self.buffer);
		self.buffer.extend_from_slice(b"\r\nSec-WebSocket-Version: 13\r\n\r\n")
	}

	/// Decode the server response to this client request.
	fn decode_response(&mut self) -> Result<Parsing<ServerResponse>, Error> {
		let mut header_buf = [httparse::EMPTY_HEADER; MAX_NUM_HEADERS];
		let mut response = httparse::Response::new(&mut header_buf);

		let offset = match response.parse(self.buffer.as_ref()) {
			Ok(httparse::Status::Complete(off)) => off,
			Ok(httparse::Status::Partial) => return Ok(Parsing::NeedMore(())),
			Err(e) => return Err(Error::Http(Box::new(e))),
		};

		if response.version != Some(1) {
			return Err(Error::UnsupportedHttpVersion);
		}

		match response.code {
			Some(101) => (),
			Some(code @ (301..=303)) | Some(code @ 307) | Some(code @ 308) => {
				// redirect response
				let location =
					with_first_header(response.headers, "Location", |loc| Ok(String::from(std::str::from_utf8(loc)?)))?;
				let response = ServerResponse::Redirect { status_code: code, location };
				return Ok(Parsing::Done { value: response, offset });
			}
			other => {
				let response = ServerResponse::Rejected { status_code: other.unwrap_or(0) };
				return Ok(Parsing::Done { value: response, offset });
			}
		}

		expect_ascii_header(response.headers, "Upgrade", "websocket")?;
		expect_ascii_header(response.headers, "Connection", "upgrade")?;

		with_first_header(&response.headers, "Sec-WebSocket-Accept", |theirs| {
			let mut digest = Sha1::new();
			digest.update(&self.nonce);
			digest.update(KEY);
			let ours = base64::engine::general_purpose::STANDARD.encode(digest.finalize());
			if ours.as_bytes() != theirs {
				return Err(Error::InvalidSecWebSocketAccept);
			}
			Ok(())
		})?;

		// Parse `Sec-WebSocket-Extensions` headers.

		for h in response.headers.iter().filter(|h| h.name.eq_ignore_ascii_case(SEC_WEBSOCKET_EXTENSIONS)) {
			configure_extensions(&mut self.extensions, std::str::from_utf8(h.value)?)?
		}

		// Match `Sec-WebSocket-Protocol` header.

		let mut selected_proto = None;
		if let Some(tp) = response.headers.iter().find(|h| h.name.eq_ignore_ascii_case(SEC_WEBSOCKET_PROTOCOL)) {
			if let Some(&p) = self.protocols.iter().find(|x| x.as_bytes() == tp.value) {
				selected_proto = Some(String::from(p))
			} else {
				return Err(Error::UnsolicitedProtocol);
			}
		}

		let response = ServerResponse::Accepted { protocol: selected_proto };
		Ok(Parsing::Done { value: response, offset })
	}
}

/// Handshake response received from the server.
#[derive(Debug)]
pub enum ServerResponse {
	/// The server has accepted our request.
	Accepted {
		/// The protocol (if any) the server has selected.
		protocol: Option<String>,
	},
	/// The server is redirecting us to some other location.
	Redirect {
		/// The HTTP response status code.
		status_code: u16,
		/// The location URL we should go to.
		location: String,
	},
	/// The server rejected our request.
	Rejected {
		/// HTTP response status code.
		status_code: u16,
	},
}
