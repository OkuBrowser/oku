// Copyright (c) 2021 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

/*!
This module somewhat mirrors [`crate::handshake::server`], except it's focus is on working
with [`http::Request`] and [`http::Response`] types, making it easier to integrate with
external web servers such as Hyper.

See `examples/hyper_server.rs` from this crate's repository for example usage.
*/

use super::{WebSocketKey, SEC_WEBSOCKET_EXTENSIONS};
use crate::connection::{self, Mode};
use crate::extension::Extension;
use crate::handshake;
use bytes::BytesMut;
use futures::prelude::*;
use http::{header, HeaderMap, Response};
use std::mem;

/// A re-export of [`handshake::Error`].
pub type Error = handshake::Error;

/// Websocket handshake server. This is similar to [`handshake::Server`], but it is
/// focused on performing the WebSocket handshake using a provided [`http::Request`], as opposed
/// to decoding the request internally.
pub struct Server {
	// Extensions the server supports.
	extensions: Vec<Box<dyn Extension + Send>>,
	// Encoding/decoding buffer.
	buffer: BytesMut,
}

impl Server {
	/// Create a new server handshake.
	pub fn new() -> Self {
		Server { extensions: Vec::new(), buffer: BytesMut::new() }
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

	/// Add an extension the server supports.
	pub fn add_extension(&mut self, e: Box<dyn Extension + Send>) -> &mut Self {
		self.extensions.push(e);
		self
	}

	/// Get back all extensions.
	pub fn drain_extensions(&mut self) -> impl Iterator<Item = Box<dyn Extension + Send>> + '_ {
		self.extensions.drain(..)
	}

	/// Attempt to interpret the provided [`http::Request`] as a WebSocket Upgrade request. If successful, this
	/// returns an [`http::Response`] that should be returned to the client to complete the handshake.
	pub fn receive_request<B>(&mut self, req: &http::Request<B>) -> Result<http::Response<()>, Error> {
		if !is_upgrade_request(&req) {
			return Err(Error::InvalidSecWebSocketAccept);
		}

		let key = match req.headers().get("Sec-WebSocket-Key") {
			Some(key) => key,
			None => {
				return Err(Error::HeaderNotFound("Sec-WebSocket-Key".into()).into());
			}
		};

		if req.headers().get("Sec-WebSocket-Version").map(|v| v.as_bytes()) != Some(b"13") {
			return Err(Error::HeaderNotFound("Sec-WebSocket-Version".into()).into());
		}

		// Pull out the Sec-WebSocket-Key and generate the appropriate response to it.
		let key: &WebSocketKey = match key.as_bytes().try_into() {
			Ok(key) => key,
			Err(_) => return Err(Error::InvalidSecWebSocketAccept),
		};
		let accept_key = handshake::generate_accept_key(key);

		// Get extension information out of the request as we'll need this as well.
		let extension_config = req
			.headers()
			.iter()
			.filter(|&(name, _)| name.as_str().eq_ignore_ascii_case(SEC_WEBSOCKET_EXTENSIONS))
			.map(|(_, value)| Ok(std::str::from_utf8(value.as_bytes())?.to_string()))
			.collect::<Result<Vec<_>, Error>>()?;

		// Attempt to set the extension configuration params that the client requested.
		for config_str in &extension_config {
			handshake::configure_extensions(&mut self.extensions, &config_str)?;
		}

		// Build a response that should be sent back to the client to acknowledge the upgrade.
		let mut response = Response::builder()
			.status(http::StatusCode::SWITCHING_PROTOCOLS)
			.header(http::header::CONNECTION, "upgrade")
			.header(http::header::UPGRADE, "websocket")
			.header("Sec-WebSocket-Accept", &accept_key[..]);

		// Tell the client about the agreed-upon extension configuration. We reuse code to build up the
		// extension header value, but that does make this a little more clunky.
		if !self.extensions.is_empty() {
			let mut buf = bytes::BytesMut::new();
			let enabled_extensions = self.extensions.iter().filter(|e| e.is_enabled()).peekable();
			handshake::append_extension_header_value(enabled_extensions, &mut buf);
			response = response.header("Sec-WebSocket-Extensions", buf.as_ref());
		}

		let response = response.body(()).expect("bug: failed to build response");
		Ok(response)
	}

	/// Turn this handshake into a [`connection::Builder`].
	pub fn into_builder<T: AsyncRead + AsyncWrite + Unpin>(mut self, socket: T) -> connection::Builder<T> {
		let mut builder = connection::Builder::new(socket, Mode::Server);
		builder.set_buffer(self.buffer);
		builder.add_extensions(self.extensions.drain(..));
		builder
	}
}

/// Check if an [`http::Request`] looks like a valid websocket upgrade request.
pub fn is_upgrade_request<B>(request: &http::Request<B>) -> bool {
	header_contains_value(request.headers(), header::CONNECTION, b"upgrade")
		&& header_contains_value(request.headers(), header::UPGRADE, b"websocket")
}

// Check if there is a header of the given name containing the wanted value.
fn header_contains_value(headers: &HeaderMap, header: header::HeaderName, value: &[u8]) -> bool {
	pub fn trim(x: &[u8]) -> &[u8] {
		let from = match x.iter().position(|x| !x.is_ascii_whitespace()) {
			Some(i) => i,
			None => return &[],
		};
		let to = x.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
		&x[from..=to]
	}

	for header in headers.get_all(header) {
		if header.as_bytes().split(|&c| c == b',').any(|x| trim(x).eq_ignore_ascii_case(value)) {
			return true;
		}
	}
	false
}
