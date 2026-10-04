// Copyright (c) 2019 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Deflate compression extension mostly conformant with [RFC 7692][rfc7692].
//!
//! [rfc7692]: https://tools.ietf.org/html/rfc7692

use crate::{
	as_u64,
	base::{Header, OpCode},
	connection::Mode,
	extension::{Extension, Param},
	BoxedError, Storage,
};
use flate2::{write::DeflateDecoder, Compress, Compression, FlushCompress, Status};
use std::{
	convert::TryInto,
	io::{self, Write},
	mem,
};

const SERVER_NO_CONTEXT_TAKEOVER: &str = "server_no_context_takeover";
const SERVER_MAX_WINDOW_BITS: &str = "server_max_window_bits";

const CLIENT_NO_CONTEXT_TAKEOVER: &str = "client_no_context_takeover";
const CLIENT_MAX_WINDOW_BITS: &str = "client_max_window_bits";

/// The deflate extension type.
///
/// The extension does currently not support max. window bits other than the
/// default, which is 15 and will ask for no context takeover during handshake.
#[derive(Debug)]
pub struct Deflate {
	mode: Mode,
	enabled: bool,
	buffer: Vec<u8>,
	params: Vec<Param<'static>>,
	our_max_window_bits: u8,
	their_max_window_bits: u8,
	await_last_fragment: bool,
}

impl Deflate {
	/// Create a new deflate extension either on client or server side.
	pub fn new(mode: Mode) -> Self {
		let params = match mode {
			Mode::Server => Vec::new(),
			Mode::Client => {
				let mut params = Vec::new();
				params.push(Param::new(SERVER_NO_CONTEXT_TAKEOVER));
				params.push(Param::new(CLIENT_NO_CONTEXT_TAKEOVER));
				params.push(Param::new(CLIENT_MAX_WINDOW_BITS));
				params
			}
		};
		Deflate {
			mode,
			enabled: false,
			buffer: Vec::new(),
			params,
			our_max_window_bits: 15,
			their_max_window_bits: 15,
			await_last_fragment: false,
		}
	}

	/// Set the server's max. window bits.
	///
	/// The value must be within 9 ..= 15.
	/// The extension must be in client mode.
	///
	/// By including this parameter, a client limits the LZ77 sliding window
	/// size that the server will use to compress messages. A server accepts
	/// by including the "server_max_window_bits" extension parameter in the
	/// response with the same or smaller value as the offer.
	pub fn set_max_server_window_bits(&mut self, max: u8) {
		assert!(self.mode == Mode::Client, "setting max. server window bits requires client mode");
		assert!(max > 8 && max <= 15, "max. server window bits have to be within 9 ..= 15");
		self.their_max_window_bits = max; // upper bound of the server's window
		let mut p = Param::new(SERVER_MAX_WINDOW_BITS);
		p.set_value(Some(max.to_string()));
		self.params.push(p)
	}

	/// Set the client's max. window bits.
	///
	/// The value must be within 9 ..= 15.
	/// The extension must be in client mode.
	///
	/// The parameter informs the server that even if it doesn't include the
	/// "client_max_window_bits" extension parameter in the response with a
	/// value greater than the one in the negotiation offer or if it doesn't
	/// include the extension parameter at all, the client is not going to
	/// use an LZ77 sliding window size greater than one given here.
	/// The server may also respond with a smaller value which allows the client
	/// to reduce its sliding window even more.
	pub fn set_max_client_window_bits(&mut self, max: u8) {
		assert!(self.mode == Mode::Client, "setting max. client window bits requires client mode");
		assert!(max > 8 && max <= 15, "max. client window bits have to be within 9 ..= 15");
		self.our_max_window_bits = max; // upper bound of the client's window
		if let Some(p) = self.params.iter_mut().find(|p| p.name() == CLIENT_MAX_WINDOW_BITS) {
			p.set_value(Some(max.to_string()));
		} else {
			let mut p = Param::new(CLIENT_MAX_WINDOW_BITS);
			p.set_value(Some(max.to_string()));
			self.params.push(p)
		}
	}

	fn set_their_max_window_bits(&mut self, p: &Param, expected: Option<u8>) -> Result<(), ()> {
		if let Some(Ok(v)) = p.value().map(|s| s.parse::<u8>()) {
			if v < 8 || v > 15 {
				log::debug!("invalid {}: {} (expected range: 8 ..= 15)", p.name(), v);
				return Err(());
			}
			if let Some(x) = expected {
				if v > x {
					log::debug!("invalid {}: {} (expected: {} <= {})", p.name(), v, v, x);
					return Err(());
				}
			}
			self.their_max_window_bits = std::cmp::max(9, v);
		}
		Ok(())
	}
}

impl Extension for Deflate {
	fn name(&self) -> &str {
		"permessage-deflate"
	}

	fn is_enabled(&self) -> bool {
		self.enabled
	}

	fn params(&self) -> &[Param] {
		&self.params
	}

	fn configure(&mut self, params: &[Param]) -> Result<(), BoxedError> {
		match self.mode {
			Mode::Server => {
				self.params.clear();
				for p in params {
					log::trace!("configure server with: {}", p);
					match p.name() {
						CLIENT_MAX_WINDOW_BITS => {
							if self.set_their_max_window_bits(&p, None).is_err() {
								// we just accept the client's offer as is => no need to reply
								return Ok(());
							}
						}
						SERVER_MAX_WINDOW_BITS => {
							if let Some(Ok(v)) = p.value().map(|s| s.parse::<u8>()) {
								// The RFC allows 8 to 15 bits, but due to zlib limitations we
								// only support 9 to 15.
								if v < 9 || v > 15 {
									log::debug!("unacceptable server_max_window_bits: {}", v);
									return Ok(());
								}
								let mut x = Param::new(SERVER_MAX_WINDOW_BITS);
								x.set_value(Some(v.to_string()));
								self.params.push(x);
								self.our_max_window_bits = v;
							} else {
								log::debug!("invalid server_max_window_bits: {:?}", p.value());
								return Ok(());
							}
						}
						CLIENT_NO_CONTEXT_TAKEOVER => self.params.push(Param::new(CLIENT_NO_CONTEXT_TAKEOVER)),
						SERVER_NO_CONTEXT_TAKEOVER => self.params.push(Param::new(SERVER_NO_CONTEXT_TAKEOVER)),
						_ => {
							log::debug!("{}: unknown parameter: {}", self.name(), p.name());
							return Ok(());
						}
					}
				}
			}
			Mode::Client => {
				let mut server_no_context_takeover = false;
				for p in params {
					log::trace!("configure client with: {}", p);
					match p.name() {
						SERVER_NO_CONTEXT_TAKEOVER => server_no_context_takeover = true,
						CLIENT_NO_CONTEXT_TAKEOVER => {} // must be supported
						SERVER_MAX_WINDOW_BITS => {
							let expected = Some(self.their_max_window_bits);
							if self.set_their_max_window_bits(&p, expected).is_err() {
								return Ok(());
							}
						}
						CLIENT_MAX_WINDOW_BITS => {
							if let Some(Ok(v)) = p.value().map(|s| s.parse::<u8>()) {
								if v < 8 || v > 15 {
									log::debug!("unacceptable client_max_window_bits: {}", v);
									return Ok(());
								}
								use std::cmp::{max, min};
								// Due to zlib limitations we have to use 9 as a lower bound
								// here, even if the server allowed us to go down to 8 bits.
								self.our_max_window_bits = min(self.our_max_window_bits, max(9, v));
							}
						}
						_ => {
							log::debug!("{}: unknown parameter: {}", self.name(), p.name());
							return Ok(());
						}
					}
				}
				if !server_no_context_takeover {
					log::debug!("{}: server did not confirm no context takeover", self.name());
					return Ok(());
				}
			}
		}
		self.enabled = true;
		Ok(())
	}

	fn reserved_bits(&self) -> (bool, bool, bool) {
		(true, false, false)
	}

	fn decode(&mut self, header: &mut Header, data: &mut Vec<u8>) -> Result<(), BoxedError> {
		if data.is_empty() {
			return Ok(());
		}

		match header.opcode() {
			OpCode::Binary | OpCode::Text if header.is_rsv1() => {
				if !header.is_fin() {
					self.await_last_fragment = true;
					log::trace!("deflate: not decoding {}; awaiting last fragment", header);
					return Ok(());
				}
				log::trace!("deflate: decoding {}", header)
			}
			OpCode::Continue if header.is_fin() && self.await_last_fragment => {
				self.await_last_fragment = false;
				log::trace!("deflate: decoding {}", header)
			}
			_ => {
				log::trace!("deflate: not decoding {}", header);
				return Ok(());
			}
		}

		// Restore LEN and NLEN:
		data.extend_from_slice(&[0, 0, 0xFF, 0xFF]); // cf. RFC 7692, 7.2.2

		self.buffer.clear();
		let mut decoder = DeflateDecoder::new(&mut self.buffer);
		decoder.write_all(&data)?;
		decoder.finish()?;
		mem::swap(data, &mut self.buffer);

		header.set_rsv1(false);
		header.set_payload_len(data.len());

		Ok(())
	}

	fn encode(&mut self, header: &mut Header, data: &mut Storage) -> Result<(), BoxedError> {
		if data.as_ref().is_empty() {
			return Ok(());
		}

		if let OpCode::Binary | OpCode::Text = header.opcode() {
			log::trace!("deflate: encoding {}", header)
		} else {
			log::trace!("deflate: not encoding {}", header);
			return Ok(());
		}

		self.buffer.clear();
		self.buffer.reserve(data.as_ref().len());

		let mut encoder = Compress::new_with_window_bits(Compression::fast(), false, self.our_max_window_bits);

		// Compress all input bytes.
		while encoder.total_in() < as_u64(data.as_ref().len()) {
			let i: usize = encoder.total_in().try_into()?;
			match encoder.compress_vec(&data.as_ref()[i..], &mut self.buffer, FlushCompress::None)? {
				Status::BufError => self.buffer.reserve(4096),
				Status::Ok => continue,
				Status::StreamEnd => break,
			}
		}

		// We need to append an empty deflate block if not there yet (RFC 7692, 7.2.1).
		while !self.buffer.ends_with(&[0, 0, 0xFF, 0xFF]) {
			self.buffer.reserve(5); // Make sure there is room for the trailing end bytes.
			match encoder.compress_vec(&[], &mut self.buffer, FlushCompress::Sync)? {
				Status::Ok => continue,
				Status::BufError => continue, // more capacity is reserved above
				Status::StreamEnd => break,
			}
		}

		// If we still have not seen the empty deflate block appended, something is wrong.
		if !self.buffer.ends_with(&[0, 0, 0xFF, 0xFF]) {
			log::error!("missing 00 00 FF FF");
			return Err(io::Error::new(io::ErrorKind::Other, "missing 00 00 FF FF").into());
		}

		self.buffer.truncate(self.buffer.len() - 4); // Remove 00 00 FF FF; cf. RFC 7692, 7.2.1

		if let Storage::Owned(d) = data {
			mem::swap(d, &mut self.buffer)
		} else {
			*data = Storage::Owned(mem::take(&mut self.buffer))
		}
		header.set_rsv1(true);
		header.set_payload_len(data.as_ref().len());
		Ok(())
	}
}
