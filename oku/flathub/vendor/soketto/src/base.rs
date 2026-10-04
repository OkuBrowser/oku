// Copyright (c) 2019 Parity Technologies (UK) Ltd.
// Copyright (c) 2016 twist developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// This file is largely based on the original twist implementation.
// See [frame/base.rs] and [codec/base.rs].
//
// [frame/base.rs]: https://github.com/rustyhorde/twist/blob/449d8b75c2/src/frame/base.rs
// [codec/base.rs]: https://github.com/rustyhorde/twist/blob/449d8b75c2/src/codec/base.rs

//! A websocket [base frame][base] codec.
//!
//! [base]: https://tools.ietf.org/html/rfc6455#section-5.2

use crate::{as_u64, Parsing};
use std::{fmt, io};

/// Max. size of a frame header.
pub(crate) const MAX_HEADER_SIZE: usize = 14;

/// Max. size of a control frame payload.
pub(crate) const MAX_CTRL_BODY_SIZE: u64 = 125;

// OpCode /////////////////////////////////////////////////////////////////////////////////////////

/// Operation codes defined in [RFC 6455](https://tools.ietf.org/html/rfc6455#section-5.2).
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum OpCode {
	/// A continuation frame of a fragmented message.
	Continue,
	/// A text data frame.
	Text,
	/// A binary data frame.
	Binary,
	/// A close control frame.
	Close,
	/// A ping control frame.
	Ping,
	/// A pong control frame.
	Pong,
	/// A reserved op code.
	Reserved3,
	/// A reserved op code.
	Reserved4,
	/// A reserved op code.
	Reserved5,
	/// A reserved op code.
	Reserved6,
	/// A reserved op code.
	Reserved7,
	/// A reserved op code.
	Reserved11,
	/// A reserved op code.
	Reserved12,
	/// A reserved op code.
	Reserved13,
	/// A reserved op code.
	Reserved14,
	/// A reserved op code.
	Reserved15,
}

impl OpCode {
	/// Is this a control opcode?
	pub fn is_control(self) -> bool {
		if let OpCode::Close | OpCode::Ping | OpCode::Pong = self {
			true
		} else {
			false
		}
	}

	/// Is this opcode reserved?
	pub fn is_reserved(self) -> bool {
		match self {
			OpCode::Reserved3
			| OpCode::Reserved4
			| OpCode::Reserved5
			| OpCode::Reserved6
			| OpCode::Reserved7
			| OpCode::Reserved11
			| OpCode::Reserved12
			| OpCode::Reserved13
			| OpCode::Reserved14
			| OpCode::Reserved15 => true,
			_ => false,
		}
	}
}

impl fmt::Display for OpCode {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			OpCode::Continue => f.write_str("Continue"),
			OpCode::Text => f.write_str("Text"),
			OpCode::Binary => f.write_str("Binary"),
			OpCode::Close => f.write_str("Close"),
			OpCode::Ping => f.write_str("Ping"),
			OpCode::Pong => f.write_str("Pong"),
			OpCode::Reserved3 => f.write_str("Reserved:3"),
			OpCode::Reserved4 => f.write_str("Reserved:4"),
			OpCode::Reserved5 => f.write_str("Reserved:5"),
			OpCode::Reserved6 => f.write_str("Reserved:6"),
			OpCode::Reserved7 => f.write_str("Reserved:7"),
			OpCode::Reserved11 => f.write_str("Reserved:11"),
			OpCode::Reserved12 => f.write_str("Reserved:12"),
			OpCode::Reserved13 => f.write_str("Reserved:13"),
			OpCode::Reserved14 => f.write_str("Reserved:14"),
			OpCode::Reserved15 => f.write_str("Reserved:15"),
		}
	}
}

/// Error returned by `OpCode::try_from` if an unknown opcode
/// number is encountered.
#[derive(Clone, Debug)]
pub struct UnknownOpCode(());

impl fmt::Display for UnknownOpCode {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str("unknown opcode")
	}
}

impl std::error::Error for UnknownOpCode {}

impl TryFrom<u8> for OpCode {
	type Error = UnknownOpCode;

	fn try_from(val: u8) -> Result<OpCode, Self::Error> {
		match val {
			0 => Ok(OpCode::Continue),
			1 => Ok(OpCode::Text),
			2 => Ok(OpCode::Binary),
			3 => Ok(OpCode::Reserved3),
			4 => Ok(OpCode::Reserved4),
			5 => Ok(OpCode::Reserved5),
			6 => Ok(OpCode::Reserved6),
			7 => Ok(OpCode::Reserved7),
			8 => Ok(OpCode::Close),
			9 => Ok(OpCode::Ping),
			10 => Ok(OpCode::Pong),
			11 => Ok(OpCode::Reserved11),
			12 => Ok(OpCode::Reserved12),
			13 => Ok(OpCode::Reserved13),
			14 => Ok(OpCode::Reserved14),
			15 => Ok(OpCode::Reserved15),
			_ => Err(UnknownOpCode(())),
		}
	}
}

impl From<OpCode> for u8 {
	fn from(opcode: OpCode) -> u8 {
		match opcode {
			OpCode::Continue => 0,
			OpCode::Text => 1,
			OpCode::Binary => 2,
			OpCode::Close => 8,
			OpCode::Ping => 9,
			OpCode::Pong => 10,
			OpCode::Reserved3 => 3,
			OpCode::Reserved4 => 4,
			OpCode::Reserved5 => 5,
			OpCode::Reserved6 => 6,
			OpCode::Reserved7 => 7,
			OpCode::Reserved11 => 11,
			OpCode::Reserved12 => 12,
			OpCode::Reserved13 => 13,
			OpCode::Reserved14 => 14,
			OpCode::Reserved15 => 15,
		}
	}
}

// Frame header ///////////////////////////////////////////////////////////////////////////////////

/// A websocket base frame header, i.e. everything but the payload.
#[derive(Debug, Clone)]
pub struct Header {
	fin: bool,
	rsv1: bool,
	rsv2: bool,
	rsv3: bool,
	masked: bool,
	opcode: OpCode,
	mask: u32,
	payload_len: usize,
}

impl fmt::Display for Header {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"({} (fin {}) (rsv {}{}{}) (mask ({} {:x})) (len {}))",
			self.opcode,
			self.fin as u8,
			self.rsv1 as u8,
			self.rsv2 as u8,
			self.rsv3 as u8,
			self.masked as u8,
			self.mask,
			self.payload_len
		)
	}
}

impl Header {
	/// Create a new frame header with a given [`OpCode`].
	pub fn new(oc: OpCode) -> Self {
		Header { fin: true, rsv1: false, rsv2: false, rsv3: false, masked: false, opcode: oc, mask: 0, payload_len: 0 }
	}

	/// Is the `fin` flag set?
	pub fn is_fin(&self) -> bool {
		self.fin
	}

	/// Set the `fin` flag.
	pub fn set_fin(&mut self, fin: bool) -> &mut Self {
		self.fin = fin;
		self
	}

	/// Is the `rsv1` flag set?
	pub fn is_rsv1(&self) -> bool {
		self.rsv1
	}

	/// Set the `rsv1` flag.
	pub fn set_rsv1(&mut self, rsv1: bool) -> &mut Self {
		self.rsv1 = rsv1;
		self
	}

	/// Is the `rsv2` flag set?
	pub fn is_rsv2(&self) -> bool {
		self.rsv2
	}

	/// Set the `rsv2` flag.
	pub fn set_rsv2(&mut self, rsv2: bool) -> &mut Self {
		self.rsv2 = rsv2;
		self
	}

	/// Is the `rsv3` flag set?
	pub fn is_rsv3(&self) -> bool {
		self.rsv3
	}

	/// Set the `rsv3` flag.
	pub fn set_rsv3(&mut self, rsv3: bool) -> &mut Self {
		self.rsv3 = rsv3;
		self
	}

	/// Is the `masked` flag set?
	pub fn is_masked(&self) -> bool {
		self.masked
	}

	/// Set the `masked` flag.
	pub fn set_masked(&mut self, masked: bool) -> &mut Self {
		self.masked = masked;
		self
	}

	/// Get the `opcode`.
	pub fn opcode(&self) -> OpCode {
		self.opcode
	}

	/// Set the `opcode`
	pub fn set_opcode(&mut self, opcode: OpCode) -> &mut Self {
		self.opcode = opcode;
		self
	}

	/// Get the `mask`.
	pub fn mask(&self) -> u32 {
		self.mask
	}

	/// Set the `mask`
	pub fn set_mask(&mut self, mask: u32) -> &mut Self {
		self.mask = mask;
		self
	}

	/// Get the payload length.
	pub fn payload_len(&self) -> usize {
		self.payload_len
	}

	/// Set the payload length.
	pub fn set_payload_len(&mut self, len: usize) -> &mut Self {
		self.payload_len = len;
		self
	}
}

// Base codec ////////////////////////////////////////////////////////////////////////////////////.

/// If the payload length byte is 126, the following two bytes represent the
/// actual payload length.
const TWO_EXT: u8 = 126;

/// If the payload length byte is 127, the following eight bytes represent
/// the actual payload length.
const EIGHT_EXT: u8 = 127;

/// Codec for encoding/decoding websocket [base] frames.
///
/// [base]: https://tools.ietf.org/html/rfc6455#section-5.2
#[derive(Debug, Clone)]
pub struct Codec {
	/// Maximum size of payload data per frame.
	max_data_size: usize,
	/// Bits reserved by an extension.
	reserved_bits: u8,
	/// Scratch buffer used during header encoding.
	header_buffer: [u8; MAX_HEADER_SIZE],
}

impl Default for Codec {
	fn default() -> Self {
		Codec { max_data_size: 256 * 1024 * 1024, reserved_bits: 0, header_buffer: [0; MAX_HEADER_SIZE] }
	}
}

impl Codec {
	/// Create a new base frame codec.
	///
	/// The codec will support decoding payload lengths up to 256 MiB
	/// (use `set_max_data_size` to change this value).
	pub fn new() -> Self {
		Codec::default()
	}

	/// Get the configured maximum payload length.
	pub fn max_data_size(&self) -> usize {
		self.max_data_size
	}

	/// Limit the maximum size of payload data to `size` bytes.
	pub fn set_max_data_size(&mut self, size: usize) -> &mut Self {
		self.max_data_size = size;
		self
	}

	/// The reserved bits currently configured.
	pub fn reserved_bits(&self) -> (bool, bool, bool) {
		let r = self.reserved_bits;
		(r & 4 == 4, r & 2 == 2, r & 1 == 1)
	}

	/// Add to the reserved bits in use.
	pub fn add_reserved_bits(&mut self, bits: (bool, bool, bool)) -> &mut Self {
		let (r1, r2, r3) = bits;
		self.reserved_bits |= (r1 as u8) << 2 | (r2 as u8) << 1 | r3 as u8;
		self
	}

	/// Reset the reserved bits.
	pub fn clear_reserved_bits(&mut self) {
		self.reserved_bits = 0
	}

	/// Decode a websocket frame header.
	pub fn decode_header(&self, bytes: &[u8]) -> Result<Parsing<Header, usize>, Error> {
		if bytes.len() < 2 {
			return Ok(Parsing::NeedMore(2 - bytes.len()));
		}

		let first = bytes[0];
		let second = bytes[1];
		let mut offset = 2;

		let fin = first & 0x80 != 0;
		let opcode = OpCode::try_from(first & 0xF)?;

		if opcode.is_reserved() {
			return Err(Error::ReservedOpCode);
		}

		if opcode.is_control() && !fin {
			return Err(Error::FragmentedControl);
		}

		let mut header = Header::new(opcode);
		header.set_fin(fin);

		let rsv1 = first & 0x40 != 0;
		if rsv1 && (self.reserved_bits & 4 == 0) {
			return Err(Error::InvalidReservedBit(1));
		}
		header.set_rsv1(rsv1);

		let rsv2 = first & 0x20 != 0;
		if rsv2 && (self.reserved_bits & 2 == 0) {
			return Err(Error::InvalidReservedBit(2));
		}
		header.set_rsv2(rsv2);

		let rsv3 = first & 0x10 != 0;
		if rsv3 && (self.reserved_bits & 1 == 0) {
			return Err(Error::InvalidReservedBit(3));
		}
		header.set_rsv3(rsv3);
		header.set_masked(second & 0x80 != 0);

		let len: u64 = match second & 0x7F {
			TWO_EXT => {
				if bytes.len() < offset + 2 {
					return Ok(Parsing::NeedMore(offset + 2 - bytes.len()));
				}
				let len = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
				offset += 2;
				u64::from(len)
			}
			EIGHT_EXT => {
				if bytes.len() < offset + 8 {
					return Ok(Parsing::NeedMore(offset + 8 - bytes.len()));
				}
				let mut b = [0; 8];
				b.copy_from_slice(&bytes[offset..offset + 8]);
				offset += 8;
				u64::from_be_bytes(b)
			}
			n => u64::from(n),
		};

		if len > MAX_CTRL_BODY_SIZE && header.opcode().is_control() {
			return Err(Error::InvalidControlFrameLen);
		}

		let len: usize = if len > as_u64(self.max_data_size) {
			return Err(Error::PayloadTooLarge { actual: len, maximum: as_u64(self.max_data_size) });
		} else {
			len as usize
		};

		header.set_payload_len(len);

		if header.is_masked() {
			if bytes.len() < offset + 4 {
				return Ok(Parsing::NeedMore(offset + 4 - bytes.len()));
			}
			let mut b = [0; 4];
			b.copy_from_slice(&bytes[offset..offset + 4]);
			offset += 4;
			header.set_mask(u32::from_be_bytes(b));
		}

		Ok(Parsing::Done { value: header, offset })
	}

	/// Encode a websocket frame header.
	pub fn encode_header(&mut self, header: &Header) -> &[u8] {
		let mut offset = 0;

		let mut first_byte = 0_u8;
		if header.is_fin() {
			first_byte |= 0x80
		}
		if header.is_rsv1() {
			first_byte |= 0x40
		}
		if header.is_rsv2() {
			first_byte |= 0x20
		}
		if header.is_rsv3() {
			first_byte |= 0x10
		}

		let opcode: u8 = header.opcode().into();
		first_byte |= opcode;

		self.header_buffer[offset] = first_byte;
		offset += 1;

		let mut second_byte = 0_u8;
		if header.is_masked() {
			second_byte |= 0x80
		}

		let len = header.payload_len();

		if len < usize::from(TWO_EXT) {
			second_byte |= len as u8;
			self.header_buffer[offset] = second_byte;
			offset += 1;
		} else if len <= usize::from(u16::max_value()) {
			second_byte |= TWO_EXT;
			self.header_buffer[offset] = second_byte;
			offset += 1;
			self.header_buffer[offset..offset + 2].copy_from_slice(&(len as u16).to_be_bytes());
			offset += 2;
		} else {
			second_byte |= EIGHT_EXT;
			self.header_buffer[offset] = second_byte;
			offset += 1;
			self.header_buffer[offset..offset + 8].copy_from_slice(&as_u64(len).to_be_bytes());
			offset += 8;
		}

		if header.is_masked() {
			self.header_buffer[offset..offset + 4].copy_from_slice(&header.mask().to_be_bytes());
			offset += 4;
		}

		&self.header_buffer[..offset]
	}

	/// Use the given header's mask and apply it to the data.
	pub fn apply_mask(header: &Header, data: &mut [u8]) {
		if header.is_masked() {
			let mask = header.mask().to_be_bytes();
			for (byte, &key) in data.iter_mut().zip(mask.iter().cycle()) {
				*byte ^= key;
			}
		}
	}
}

/// Error cases the base frame decoder may encounter.
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
	/// An I/O error has been encountered.
	Io(io::Error),
	/// Some unknown opcode number has been decoded.
	UnknownOpCode,
	/// The opcode decoded is reserved.
	ReservedOpCode,
	/// A fragmented control frame (fin bit not set) has been decoded.
	FragmentedControl,
	/// A control frame with an invalid length code has been decoded.
	InvalidControlFrameLen,
	/// The reserved bit is invalid.
	InvalidReservedBit(u8),
	/// The payload length of a frame exceeded the configured maximum.
	PayloadTooLarge { actual: u64, maximum: u64 },
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::Io(e) => write!(f, "i/o error: {}", e),
			Error::UnknownOpCode => f.write_str("unknown opcode"),
			Error::ReservedOpCode => f.write_str("reserved opcode"),
			Error::FragmentedControl => f.write_str("fragmented control frame"),
			Error::InvalidControlFrameLen => f.write_str("invalid control frame length"),
			Error::InvalidReservedBit(n) => write!(f, "invalid reserved bit: {}", n),
			Error::PayloadTooLarge { actual, maximum } => {
				write!(f, "payload too large: len = {}, maximum = {}", actual, maximum)
			}
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Error::Io(e) => Some(e),
			Error::UnknownOpCode
			| Error::ReservedOpCode
			| Error::FragmentedControl
			| Error::InvalidControlFrameLen
			| Error::InvalidReservedBit(_)
			| Error::PayloadTooLarge { .. } => None,
		}
	}
}

impl From<io::Error> for Error {
	fn from(e: io::Error) -> Self {
		Error::Io(e)
	}
}

impl From<UnknownOpCode> for Error {
	fn from(_: UnknownOpCode) -> Self {
		Error::UnknownOpCode
	}
}

// Tests //////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
	use super::{Codec, Error, OpCode};
	use crate::Parsing;
	use quickcheck::QuickCheck;

	#[test]
	fn decode_partial_header() {
		let partial_header: &[u8] = &[0x89];
		assert!(matches! {
			Codec::new().decode_header(partial_header),
			Ok(Parsing::NeedMore(1))
		})
	}

	#[test]
	fn decode_partial_len() {
		let partial_length_1: &[u8] = &[0x89, 0xFE, 0x01];
		assert!(matches! {
			Codec::new().decode_header(partial_length_1),
			Ok(Parsing::NeedMore(1))
		});
		let partial_length_2: &[u8] = &[0x89, 0xFF, 0x01, 0x02, 0x03, 0x04];
		assert!(matches! {
			Codec::new().decode_header(partial_length_2),
			Ok(Parsing::NeedMore(4))
		})
	}

	#[test]
	fn decode_partial_mask() {
		let partial_mask: &[u8] = &[0x82, 0xFE, 0x01, 0x02, 0x00, 0x00];
		assert!(matches! {
			Codec::new().decode_header(partial_mask),
			Ok(Parsing::NeedMore(2))
		})
	}

	#[test]
	fn decode_partial_payload() {
		let partial_payload: &mut [u8] = &mut [0x82, 0x85, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00];
		if let Ok(Parsing::Done { value, offset }) = Codec::new().decode_header(partial_payload) {
			assert_eq!(3, value.payload_len() - (partial_payload.len() - offset))
		} else {
			assert!(false)
		}
	}

	#[test]
	fn decode_invalid_control_payload_len() {
		// Payload on control frame must be 125 bytes or less. 2nd byte must be 0xFD or less.
		let ctrl_payload_len: &[u8] = &[0x89, 0xFE, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
		assert!(matches! {
			Codec::new().decode_header(ctrl_payload_len),
			Err(Error::InvalidControlFrameLen)
		})
	}

	/// Checking that rsv1, rsv2, and rsv3 bit set returns error.
	#[test]
	fn decode_reserved() {
		// rsv1, rsv2, and rsv3.
		let reserved = [0x90, 0xa0, 0xc0];
		for res in &reserved {
			let mut buf = [0; 2];
			buf[0] |= *res;
			assert!(matches! {
				Codec::new().decode_header(&buf),
				Err(Error::InvalidReservedBit(_))
			})
		}
	}

	/// Checking that a control frame, where fin bit is 0, returns an error.
	#[test]
	fn decode_fragmented_control() {
		let second_bytes = [8, 9, 10];
		for sb in &second_bytes {
			let mut buf = [0; 2];
			buf[0] |= *sb;
			assert!(matches! {
				Codec::new().decode_header(&buf),
				Err(Error::FragmentedControl)
			})
		}
	}

	/// Checking that reserved opcodes return an error.
	#[test]
	fn decode_reserved_opcodes() {
		let reserved = [3, 4, 5, 6, 7, 11, 12, 13, 14, 15];
		for res in &reserved {
			let mut buf = [0; 2];
			buf[0] |= 0x80 | *res;
			assert!(matches! {
				Codec::new().decode_header(&buf),
				Err(Error::ReservedOpCode)
			})
		}
	}

	#[test]
	fn decode_ping_no_data() {
		let ping_no_data: &mut [u8] = &mut [0x89, 0x80, 0x00, 0x00, 0x00, 0x01];
		let c = Codec::new();
		if let Ok(Parsing::Done { value: header, .. }) = c.decode_header(ping_no_data) {
			assert!(header.is_fin());
			assert!(!header.is_rsv1());
			assert!(!header.is_rsv2());
			assert!(!header.is_rsv3());
			assert!(header.opcode() == OpCode::Ping);
			assert!(header.payload_len() == 0)
		} else {
			assert!(false)
		}
	}

	#[test]
	fn reserved_bits() {
		fn property(bits: (bool, bool, bool)) -> bool {
			let mut c = Codec::new();
			assert_eq!((false, false, false), c.reserved_bits());
			c.add_reserved_bits(bits);
			bits == c.reserved_bits()
		}
		QuickCheck::new().quickcheck(property as fn((bool, bool, bool)) -> bool)
	}
}
