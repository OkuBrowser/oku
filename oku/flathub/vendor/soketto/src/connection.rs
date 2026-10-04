// Copyright (c) 2019 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! A persistent websocket connection after the handshake phase, represented
//! as a [`Sender`] and [`Receiver`] pair.

use crate::data::{ByteSlice125, Data, Incoming};
use crate::{
	base::{self, Header, OpCode, MAX_HEADER_SIZE},
	extension::Extension,
	Parsing, Storage,
};
use bytes::{Buf, BytesMut};
use futures::{
	io::{ReadHalf, WriteHalf},
	lock::BiLock,
	prelude::*,
};
use std::{fmt, io, str};

/// Accumulated max. size of a complete message.
const MAX_MESSAGE_SIZE: usize = 256 * 1024 * 1024;

/// Max. size of a single message frame.
const MAX_FRAME_SIZE: usize = MAX_MESSAGE_SIZE;

/// Is the connection used by a client or server?
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
	/// Client-side of a connection (implies masking of payload data).
	Client,
	/// Server-side of a connection.
	Server,
}

impl Mode {
	pub fn is_client(self) -> bool {
		if let Mode::Client = self {
			true
		} else {
			false
		}
	}

	pub fn is_server(self) -> bool {
		!self.is_client()
	}
}

/// Connection ID.
#[derive(Clone, Copy, Debug)]
struct Id(u32);

impl fmt::Display for Id {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:08x}", self.0)
	}
}

/// The sending half of a connection.
#[derive(Debug)]
pub struct Sender<T> {
	id: Id,
	mode: Mode,
	codec: base::Codec,
	writer: BiLock<WriteHalf<T>>,
	mask_buffer: Vec<u8>,
	extensions: BiLock<Vec<Box<dyn Extension + Send>>>,
	has_extensions: bool,
}

/// The receiving half of a connection.
#[derive(Debug)]
pub struct Receiver<T> {
	id: Id,
	mode: Mode,
	codec: base::Codec,
	reader: ReadHalf<T>,
	writer: BiLock<WriteHalf<T>>,
	extensions: BiLock<Vec<Box<dyn Extension + Send>>>,
	has_extensions: bool,
	buffer: BytesMut,
	ctrl_buffer: BytesMut,
	max_message_size: usize,
	is_closed: bool,
}

/// A connection builder.
///
/// Allows configuring certain parameters and extensions before
/// creating the [`Sender`]/[`Receiver`] pair that represents the
/// connection.
#[derive(Debug)]
pub struct Builder<T> {
	id: Id,
	mode: Mode,
	socket: T,
	codec: base::Codec,
	extensions: Vec<Box<dyn Extension + Send>>,
	buffer: BytesMut,
	max_message_size: usize,
}

impl<T: AsyncRead + AsyncWrite + Unpin> Builder<T> {
	/// Create a new `Builder` from the given async I/O resource and mode.
	///
	/// **Note**: Use this type only after a successful [handshake][0].
	/// You can either use this crate's [handshake functionality][1]
	/// or perform the handshake by some other means.
	///
	/// [0]: https://tools.ietf.org/html/rfc6455#section-4
	/// [1]: crate::handshake
	pub fn new(socket: T, mode: Mode) -> Self {
		let mut codec = base::Codec::default();
		codec.set_max_data_size(MAX_FRAME_SIZE);
		Builder {
			id: Id(rand::random()),
			mode,
			socket,
			codec,
			extensions: Vec::new(),
			buffer: BytesMut::new(),
			max_message_size: MAX_MESSAGE_SIZE,
		}
	}

	/// Set a custom buffer to use.
	pub fn set_buffer(&mut self, b: BytesMut) {
		self.buffer = b
	}

	/// Add extensions to use with this connection.
	///
	/// Only enabled extensions will be considered.
	pub fn add_extensions<I>(&mut self, extensions: I)
	where
		I: IntoIterator<Item = Box<dyn Extension + Send>>,
	{
		for e in extensions.into_iter().filter(|e| e.is_enabled()) {
			log::debug!("{}: using extension: {}", self.id, e.name());
			self.codec.add_reserved_bits(e.reserved_bits());
			self.extensions.push(e)
		}
	}

	/// Set the maximum size of a complete message.
	///
	/// Message fragments will be buffered and concatenated up to this value,
	/// i.e. the sum of all message frames payload lengths will not be greater
	/// than this maximum. However, extensions may increase the total message
	/// size further, e.g. by decompressing the payload data.
	pub fn set_max_message_size(&mut self, max: usize) {
		self.max_message_size = max
	}

	/// Set the maximum size of a single websocket frame payload.
	pub fn set_max_frame_size(&mut self, max: usize) {
		self.codec.set_max_data_size(max);
	}

	/// Create a configured [`Sender`]/[`Receiver`] pair.
	pub fn finish(self) -> (Sender<T>, Receiver<T>) {
		let (rhlf, whlf) = self.socket.split();
		let (wrt1, wrt2) = BiLock::new(whlf);
		let has_extensions = !self.extensions.is_empty();
		let (ext1, ext2) = BiLock::new(self.extensions);

		let recv = Receiver {
			id: self.id,
			mode: self.mode,
			reader: rhlf,
			writer: wrt1,
			codec: self.codec.clone(),
			extensions: ext1,
			has_extensions,
			buffer: self.buffer,
			ctrl_buffer: BytesMut::new(),
			max_message_size: self.max_message_size,
			is_closed: false,
		};

		let send = Sender {
			id: self.id,
			mode: self.mode,
			writer: wrt2,
			mask_buffer: Vec::new(),
			codec: self.codec,
			extensions: ext2,
			has_extensions,
		};

		(send, recv)
	}
}

impl<T: AsyncRead + AsyncWrite + Unpin> Receiver<T> {
	/// Receive the next websocket message.
	///
	/// The received frames forming the complete message will be appended to
	/// the given `message` argument. The returned [`Incoming`] value describes
	/// the type of data that was received, e.g. binary or textual data.
	///
	/// Interleaved PONG frames are returned immediately as `Data::Pong`
	/// values. If PONGs are not expected or uninteresting,
	/// [`Receiver::receive_data`] may be used instead which skips over PONGs
	/// and considers only application payload data.
	pub async fn receive(&mut self, message: &mut Vec<u8>) -> Result<Incoming<'_>, Error> {
		let mut first_fragment_opcode = None;
		let mut length: usize = 0;
		let message_len = message.len();
		loop {
			if self.is_closed {
				log::debug!("{}: cannot receive, connection is closed", self.id);
				return Err(Error::Closed);
			}

			self.ctrl_buffer.clear();
			let mut header = self.receive_header().await?;
			log::trace!("{}: recv: {}", self.id, header);

			// Handle control frames: PING, PONG and CLOSE.
			if header.opcode().is_control() {
				self.read_buffer(&header).await?;
				self.ctrl_buffer = self.buffer.split_to(header.payload_len());
				base::Codec::apply_mask(&header, &mut self.ctrl_buffer);
				if header.opcode() == OpCode::Pong {
					return Ok(Incoming::Pong(&self.ctrl_buffer[..]));
				}
				if let Some(close_reason) = self.on_control(&header).await? {
					log::trace!("{}: recv, incoming CLOSE: {:?}", self.id, close_reason);
					return Ok(Incoming::Closed(close_reason));
				}
				continue;
			}

			length = length.saturating_add(header.payload_len());

			// Check if total message does not exceed maximum.
			if length > self.max_message_size {
				log::warn!("{}: accumulated message length exceeds maximum", self.id);

				// Discard bytes that were too large to fit in the buffer.
				discard_bytes(length as u64, &mut self.reader).await?;
				return Err(Error::MessageTooLarge { current: length, maximum: self.max_message_size });
			}

			// Get the frame's payload data bytes from buffer or socket.
			{
				let old_msg_len = message.len();

				let bytes_to_read = {
					let required = header.payload_len();
					let buffered = self.buffer.len();

					if buffered == 0 {
						required
					} else if required > buffered {
						message.extend_from_slice(&self.buffer);
						self.buffer.clear();
						required - buffered
					} else {
						message.extend_from_slice(&self.buffer.split_to(required));
						0
					}
				};

				if bytes_to_read > 0 {
					let n = message.len();
					message.resize(n + bytes_to_read, 0u8);
					self.reader.read_exact(&mut message[n..]).await?
				}

				debug_assert_eq!(header.payload_len(), message.len() - old_msg_len);

				base::Codec::apply_mask(&header, &mut message[old_msg_len..]);
			}

			match (header.is_fin(), header.opcode()) {
				(false, OpCode::Continue) => {
					// Intermediate message fragment.
					if first_fragment_opcode.is_none() {
						log::debug!("{}: continue frame while not processing message fragments", self.id);
						return Err(Error::UnexpectedOpCode(OpCode::Continue));
					}
					continue;
				}
				(false, oc) => {
					// Initial message fragment.
					if first_fragment_opcode.is_some() {
						log::debug!("{}: initial fragment while processing a fragmented message", self.id);
						return Err(Error::UnexpectedOpCode(oc));
					}
					first_fragment_opcode = Some(oc);
					self.decode_with_extensions(&mut header, message).await?;
					continue;
				}
				(true, OpCode::Continue) => {
					// Last message fragment.
					if let Some(oc) = first_fragment_opcode.take() {
						header.set_payload_len(message.len());
						log::trace!("{}: last fragment: total length = {} bytes", self.id, message.len());
						self.decode_with_extensions(&mut header, message).await?;
						header.set_opcode(oc);
					} else {
						log::debug!("{}: last continue frame while not processing message fragments", self.id);
						return Err(Error::UnexpectedOpCode(OpCode::Continue));
					}
				}
				(true, oc) => {
					// Regular non-fragmented message.
					if first_fragment_opcode.is_some() {
						log::debug!("{}: regular message while processing fragmented message", self.id);
						return Err(Error::UnexpectedOpCode(oc));
					}
					self.decode_with_extensions(&mut header, message).await?
				}
			}

			let num_bytes = message.len() - message_len;

			if header.opcode() == OpCode::Text {
				return Ok(Incoming::Data(Data::Text(num_bytes)));
			} else {
				return Ok(Incoming::Data(Data::Binary(num_bytes)));
			}
		}
	}

	/// Receive the next websocket message, skipping over control frames.
	pub async fn receive_data(&mut self, message: &mut Vec<u8>) -> Result<Data, Error> {
		loop {
			if let Incoming::Data(d) = self.receive(message).await? {
				return Ok(d);
			}
		}
	}

	/// Read the next frame header.
	async fn receive_header(&mut self) -> Result<Header, Error> {
		loop {
			match self.codec.decode_header(&self.buffer)? {
				Parsing::Done { value: header, offset } => {
					debug_assert!(offset <= MAX_HEADER_SIZE);
					self.buffer.advance(offset);
					return Ok(header);
				}
				Parsing::NeedMore(n) => crate::read(&mut self.reader, &mut self.buffer, n).await?,
			}
		}
	}

	/// Read the complete payload data into the read buffer.
	async fn read_buffer(&mut self, header: &Header) -> Result<(), Error> {
		if header.payload_len() <= self.buffer.len() {
			return Ok(());
		}
		let i = self.buffer.len();
		let d = header.payload_len() - i;
		self.buffer.resize(i + d, 0u8);
		self.reader.read_exact(&mut self.buffer[i..]).await?;
		Ok(())
	}

	/// Answer incoming control frames.
	/// `PING`: replied to immediately with a `PONG`
	/// `PONG`: no action
	/// `CLOSE`: replied to immediately with a `CLOSE`; returns the [`CloseReason`]
	/// All other [`OpCode`]s return [`Error::UnexpectedOpCode`]
	async fn on_control(&mut self, header: &Header) -> Result<Option<CloseReason>, Error> {
		match header.opcode() {
			OpCode::Ping => {
				let mut answer = Header::new(OpCode::Pong);
				let mut unused = Vec::new();
				let mut data = Storage::Unique(&mut self.ctrl_buffer);
				write(self.id, self.mode, &mut self.codec, &mut self.writer, &mut answer, &mut data, &mut unused)
					.await?;
				self.flush().await?;
				Ok(None)
			}
			OpCode::Pong => Ok(None),
			OpCode::Close => {
				log::trace!("{}: Acknowledging CLOSE to sender", self.id);
				let (mut header, reason) = close_answer(&self.ctrl_buffer)?;
				// Write back a Close frame
				let mut unused = Vec::new();
				if let Some(CloseReason { code, .. }) = reason {
					let mut data = code.to_be_bytes();
					let mut data = Storage::Unique(&mut data);
					let _ = write(
						self.id,
						self.mode,
						&mut self.codec,
						&mut self.writer,
						&mut header,
						&mut data,
						&mut unused,
					)
					.await;
				} else {
					let mut data = Storage::Unique(&mut []);
					let _ = write(
						self.id,
						self.mode,
						&mut self.codec,
						&mut self.writer,
						&mut header,
						&mut data,
						&mut unused,
					)
					.await;
				}
				self.flush().await?;
				// Close down the connection but the I/O stream could already be closed and
				// we don't want propagate such error to the user if the I/O was already closed.
				_ = self.writer.lock().await.close().await;
				self.is_closed = true;
				Ok(reason)
			}
			OpCode::Binary
			| OpCode::Text
			| OpCode::Continue
			| OpCode::Reserved3
			| OpCode::Reserved4
			| OpCode::Reserved5
			| OpCode::Reserved6
			| OpCode::Reserved7
			| OpCode::Reserved11
			| OpCode::Reserved12
			| OpCode::Reserved13
			| OpCode::Reserved14
			| OpCode::Reserved15 => Err(Error::UnexpectedOpCode(header.opcode())),
		}
	}

	/// Apply all extensions to the given header and the internal message buffer.
	async fn decode_with_extensions(&mut self, header: &mut Header, message: &mut Vec<u8>) -> Result<(), Error> {
		if !self.has_extensions {
			return Ok(());
		}
		for e in self.extensions.lock().await.iter_mut() {
			log::trace!("{}: decoding with extension: {}", self.id, e.name());
			e.decode(header, message).map_err(Error::Extension)?
		}
		Ok(())
	}

	/// Flush the socket buffer.
	async fn flush(&mut self) -> Result<(), Error> {
		log::trace!("{}: Receiver flushing connection", self.id);
		if self.is_closed {
			return Ok(());
		}
		self.writer.lock().await.flush().await.or(Err(Error::Closed))
	}
}

impl<T: AsyncRead + AsyncWrite + Unpin> Sender<T> {
	/// Send a text value over the websocket connection.
	pub async fn send_text(&mut self, data: impl AsRef<str>) -> Result<(), Error> {
		let mut header = Header::new(OpCode::Text);
		self.send_frame(&mut header, &mut Storage::Shared(data.as_ref().as_bytes())).await
	}

	/// Send a text value over the websocket connection.
	///
	/// This method performs one copy fewer than [`Sender::send_text`].
	pub async fn send_text_owned(&mut self, data: String) -> Result<(), Error> {
		let mut header = Header::new(OpCode::Text);
		self.send_frame(&mut header, &mut Storage::Owned(data.into_bytes())).await
	}

	/// Send some binary data over the websocket connection.
	pub async fn send_binary(&mut self, data: impl AsRef<[u8]>) -> Result<(), Error> {
		let mut header = Header::new(OpCode::Binary);
		self.send_frame(&mut header, &mut Storage::Shared(data.as_ref())).await
	}

	/// Send some binary data over the websocket connection.
	///
	/// This method performs one copy fewer than [`Sender::send_binary`].
	/// The `data` buffer may be modified by this method, e.g. if masking is necessary.
	pub async fn send_binary_mut(&mut self, mut data: impl AsMut<[u8]>) -> Result<(), Error> {
		let mut header = Header::new(OpCode::Binary);
		self.send_frame(&mut header, &mut Storage::Unique(data.as_mut())).await
	}

	/// Ping the remote end.
	pub async fn send_ping(&mut self, data: ByteSlice125<'_>) -> Result<(), Error> {
		let mut header = Header::new(OpCode::Ping);
		self.write(&mut header, &mut Storage::Shared(data.as_ref())).await
	}

	/// Send an unsolicited Pong to the remote.
	pub async fn send_pong(&mut self, data: ByteSlice125<'_>) -> Result<(), Error> {
		let mut header = Header::new(OpCode::Pong);
		self.write(&mut header, &mut Storage::Shared(data.as_ref())).await
	}

	/// Flush the socket buffer.
	pub async fn flush(&mut self) -> Result<(), Error> {
		log::trace!("{}: Sender flushing connection", self.id);
		self.writer.lock().await.flush().await.or(Err(Error::Closed))
	}

	/// Send a close message and close the connection.
	pub async fn close(&mut self) -> Result<(), Error> {
		log::trace!("{}: closing connection", self.id);
		let mut header = Header::new(OpCode::Close);
		let code = 1000_u16.to_be_bytes(); // 1000 = normal closure
		self.write(&mut header, &mut Storage::Shared(&code[..])).await?;
		self.flush().await?;
		self.writer.lock().await.close().await.or(Err(Error::Closed))
	}

	/// Send arbitrary websocket frames.
	///
	/// Before sending, extensions will be applied to header and payload data.
	async fn send_frame(&mut self, header: &mut Header, data: &mut Storage<'_>) -> Result<(), Error> {
		if !self.has_extensions {
			return self.write(header, data).await;
		}

		for e in self.extensions.lock().await.iter_mut() {
			log::trace!("{}: encoding with extension: {}", self.id, e.name());
			e.encode(header, data).map_err(Error::Extension)?
		}

		self.write(header, data).await
	}

	/// Write final header and payload data to socket.
	///
	/// The data will be masked if necessary.
	/// No extensions will be applied to header and payload data.
	async fn write(&mut self, header: &mut Header, data: &mut Storage<'_>) -> Result<(), Error> {
		write(self.id, self.mode, &mut self.codec, &mut self.writer, header, data, &mut self.mask_buffer).await
	}
}

/// Write header and payload data to socket.
async fn write<T: AsyncWrite + Unpin>(
	id: Id,
	mode: Mode,
	codec: &mut base::Codec,
	writer: &mut BiLock<WriteHalf<T>>,
	header: &mut Header,
	data: &mut Storage<'_>,
	mask_buffer: &mut Vec<u8>,
) -> Result<(), Error> {
	if mode.is_client() {
		header.set_masked(true);
		header.set_mask(rand::random());
	}
	header.set_payload_len(data.as_ref().len());

	log::trace!("{}: send: {}", id, header);

	let header_bytes = codec.encode_header(&header);
	let mut w = writer.lock().await;
	w.write_all(&header_bytes).await.or(Err(Error::Closed))?;

	if !header.is_masked() {
		return w.write_all(data.as_ref()).await.or(Err(Error::Closed));
	}

	match data {
		Storage::Shared(slice) => {
			mask_buffer.clear();
			mask_buffer.extend_from_slice(slice);
			base::Codec::apply_mask(header, mask_buffer);
			w.write_all(mask_buffer).await.or(Err(Error::Closed))
		}
		Storage::Unique(slice) => {
			base::Codec::apply_mask(header, slice);
			w.write_all(slice).await.or(Err(Error::Closed))
		}
		Storage::Owned(ref mut bytes) => {
			base::Codec::apply_mask(header, bytes);
			w.write_all(bytes).await.or(Err(Error::Closed))
		}
	}
}

/// Create a close frame based on the given data. The close frame is echoed back
/// to the sender.
fn close_answer(data: &[u8]) -> Result<(Header, Option<CloseReason>), Error> {
	let answer = Header::new(OpCode::Close);
	if data.len() < 2 {
		return Ok((answer, None));
	}
	// Check that the reason string is properly encoded
	let descr = std::str::from_utf8(&data[2..])?.into();
	let code = u16::from_be_bytes([data[0], data[1]]);
	let reason = CloseReason { code, descr: Some(descr) };

	// Status codes are defined in
	// https://tools.ietf.org/html/rfc6455#section-7.4.1 and
	// https://mailarchive.ietf.org/arch/msg/hybi/P_1vbD9uyHl63nbIIbFxKMfSwcM/
	match code {
        | 1000 ..= 1003
        | 1007 ..= 1011
        | 1012 // Service Restart
        | 1013 // Try Again Later
        | 1015
        | 3000 ..= 4999 => Ok((answer, Some(reason))), // acceptable codes
        _               => {
            // invalid code => protocol error (1002)
            Ok((answer, Some(CloseReason { code: 1002, descr: None})))
        }
    }
}

/// Errors which may occur when sending or receiving messages.
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
	/// An I/O error was encountered.
	Io(io::Error),
	/// The base codec errored.
	Codec(base::Error),
	/// An extension produced an error while encoding or decoding.
	Extension(crate::BoxedError),
	/// An unexpected opcode was encountered.
	UnexpectedOpCode(OpCode),
	/// A close reason was not correctly UTF-8 encoded.
	Utf8(str::Utf8Error),
	/// The total message payload data size exceeds the configured maximum.
	MessageTooLarge { current: usize, maximum: usize },
	/// The connection is closed.
	Closed,
}

/// Reason for closing the connection.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CloseReason {
	pub code: u16,
	pub descr: Option<String>,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::Io(e) => write!(f, "i/o error: {}", e),
			Error::Codec(e) => write!(f, "codec error: {}", e),
			Error::Extension(e) => write!(f, "extension error: {}", e),
			Error::UnexpectedOpCode(c) => write!(f, "unexpected opcode: {}", c),
			Error::Utf8(e) => write!(f, "utf-8 error: {}", e),
			Error::MessageTooLarge { current, maximum } => {
				write!(f, "message too large: len >= {}, maximum = {}", current, maximum)
			}
			Error::Closed => f.write_str("connection closed"),
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Error::Io(e) => Some(e),
			Error::Codec(e) => Some(e),
			Error::Extension(e) => Some(&**e),
			Error::Utf8(e) => Some(e),
			Error::UnexpectedOpCode(_) | Error::MessageTooLarge { .. } | Error::Closed => None,
		}
	}
}

impl From<io::Error> for Error {
	fn from(e: io::Error) -> Self {
		if e.kind() == io::ErrorKind::UnexpectedEof {
			Error::Closed
		} else {
			Error::Io(e)
		}
	}
}

impl From<str::Utf8Error> for Error {
	fn from(e: str::Utf8Error) -> Self {
		Error::Utf8(e)
	}
}

impl From<base::Error> for Error {
	fn from(e: base::Error) -> Self {
		Error::Codec(e)
	}
}

/// Discard `n` bytes from the underlying reader.
async fn discard_bytes<R: AsyncRead + Unpin>(n: u64, reader: R) -> Result<u64, io::Error> {
	futures::io::copy(&mut reader.take(n), &mut futures::io::sink()).await
}

#[cfg(test)]
mod tests {
	use super::discard_bytes;
	use futures::{io::Cursor, AsyncReadExt};

	#[tokio::test]
	async fn discard_bytes_works() {
		let bytes: Vec<u8> = (0..5).collect();
		let mut cursor = Cursor::new(bytes);
		discard_bytes(1_u64, &mut cursor).await.unwrap();
		let mut read = vec![0; 4];
		cursor.read_exact(&mut read).await.unwrap();
		assert_eq!(read, vec![1, 2, 3, 4]);
	}
}
