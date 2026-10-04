// Copyright (c) 2019 Parity Technologies (UK) Ltd.
// Copyright (c) 2016 twist developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Websocket extensions as per [RFC 6455][rfc6455].
//!
//! [rfc6455]: https://tools.ietf.org/html/rfc6455#section-9

#[cfg(feature = "deflate")]
pub mod deflate;

use crate::{base::Header, BoxedError, Storage};
use std::{borrow::Cow, fmt};

/// A websocket extension as per RFC 6455, section 9.
///
/// Extensions are invoked during handshake and subsequently during base
/// frame encoding and decoding. The invocation during handshake differs
/// on client and server side.
///
/// # Server
///
/// 1. All extensions should consider themselves as disabled but available.
/// 2. When receiving a handshake request from a client, for each extension
/// with a matching name, [`Extension::configure`] will be applied to the
/// request parameters. The extension may internally enable itself.
/// 3. When sending back the response, for each extension whose
/// [`Extension::is_enabled`] returns true, the extension name and its
/// parameters (as returned by [`Extension::params`]) will be included in the
/// response.
///
/// # Client
///
/// 1. All extensions should consider themselves as disabled but available.
/// 2. When creating the handshake request, all extensions and its parameters
/// (as returned by [`Extension::params`]) will be included in the request.
/// 3. When receiving the response from the server, for every extension with
/// a matching name in the response, [`Extension::configure`] will be applied
/// to the response parameters. The extension may internally enable itself.
///
/// After this handshake phase, extensions have been configured and are
/// potentially enabled. Enabled extensions can then be used for further base
/// frame processing.
pub trait Extension: std::fmt::Debug {
	/// Is this extension enabled?
	fn is_enabled(&self) -> bool;

	/// The name of this extension.
	fn name(&self) -> &str;

	/// The parameters this extension wants to send for negotiation.
	fn params(&self) -> &[Param];

	/// Configure this extension with the parameters received from negotiation.
	fn configure(&mut self, params: &[Param]) -> Result<(), BoxedError>;

	/// Encode a frame, given as frame header and payload data.
	fn encode(&mut self, header: &mut Header, data: &mut Storage) -> Result<(), BoxedError>;

	/// Decode a frame.
	///
	/// The frame header is given, as well as the accumulated payload data, i.e.
	/// the concatenated payload data of all message fragments.
	fn decode(&mut self, header: &mut Header, data: &mut Vec<u8>) -> Result<(), BoxedError>;

	/// The reserved bits this extension uses.
	fn reserved_bits(&self) -> (bool, bool, bool) {
		(false, false, false)
	}
}

impl<E: Extension + ?Sized> Extension for Box<E> {
	fn is_enabled(&self) -> bool {
		(**self).is_enabled()
	}

	fn name(&self) -> &str {
		(**self).name()
	}

	fn params(&self) -> &[Param] {
		(**self).params()
	}

	fn configure(&mut self, params: &[Param]) -> Result<(), BoxedError> {
		(**self).configure(params)
	}

	fn encode(&mut self, header: &mut Header, data: &mut Storage) -> Result<(), BoxedError> {
		(**self).encode(header, data)
	}

	fn decode(&mut self, header: &mut Header, data: &mut Vec<u8>) -> Result<(), BoxedError> {
		(**self).decode(header, data)
	}

	fn reserved_bits(&self) -> (bool, bool, bool) {
		(**self).reserved_bits()
	}
}

/// Extension parameter (used for negotiation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param<'a> {
	name: Cow<'a, str>,
	value: Option<Cow<'a, str>>,
}

impl<'a> fmt::Display for Param<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if let Some(v) = &self.value {
			write!(f, "{} = {}", self.name, v)
		} else {
			write!(f, "{}", self.name)
		}
	}
}

impl<'a> Param<'a> {
	/// Create a new parameter with the given name.
	pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
		Param { name: name.into(), value: None }
	}

	/// Access the parameter name.
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Access the optional parameter value.
	pub fn value(&self) -> Option<&str> {
		self.value.as_ref().map(|v| v.as_ref())
	}

	/// Set the parameter to the given value.
	pub fn set_value(&mut self, value: Option<impl Into<Cow<'a, str>>>) -> &mut Self {
		self.value = value.map(Into::into);
		self
	}

	/// Turn this parameter into one that owns its values.
	pub fn acquire(self) -> Param<'static> {
		Param { name: Cow::Owned(self.name.into_owned()), value: self.value.map(|v| Cow::Owned(v.into_owned())) }
	}
}
