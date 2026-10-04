// Copyright (c) 2019 Parity Technologies (UK) Ltd.
// Copyright (c) 2016 twist developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! An implementation of the [RFC 6455][rfc6455] websocket protocol.
//!
//! To begin a websocket connection one first needs to perform a [handshake],
//! either as [client] or [server], in order to upgrade from HTTP.
//! Once successful, the client or server can transition to a connection,
//! i.e. a [Sender]/[Receiver] pair and send and receive textual or
//! binary data.
//!
//! **Note**: While it is possible to only receive websocket messages it is
//! not possible to only send websocket messages. Receiving data is required
//! in order to react to control frames such as PING or CLOSE. While those will be
//! answered transparently they have to be received in the first place, so
//! calling [`connection::Receiver::receive`] is imperative.
//!
//! **Note**: None of the `async` methods are safe to cancel so their `Future`s
//! must not be dropped unless they return `Poll::Ready`.
//!
//! # Client example
//!
//! ```no_run
//! # use tokio_util::compat::TokioAsyncReadCompatExt;
//! # async fn doc() -> Result<(), soketto::BoxedError> {
//! use soketto::handshake::{Client, ServerResponse};
//!
//! // First, we need to establish a TCP connection.
//! let socket = tokio::net::TcpStream::connect("...").await?;
//!
//! // Then we configure the client handshake.
//! let mut client = Client::new(socket.compat(), "...", "/");
//!
//! // And finally we perform the handshake and handle the result.
//! let (mut sender, mut receiver) = match client.handshake().await? {
//!     ServerResponse::Accepted { .. } => client.into_builder().finish(),
//!     ServerResponse::Redirect { status_code, location } => unimplemented!("follow location URL"),
//!     ServerResponse::Rejected { status_code } => unimplemented!("handle failure")
//! };
//!
//! // Over the established websocket connection we can send
//! sender.send_text("some text").await?;
//! sender.send_text("some more text").await?;
//! sender.flush().await?;
//!
//! // ... and receive data.
//! let mut data = Vec::new();
//! receiver.receive_data(&mut data).await?;
//!
//! # Ok(())
//! # }
//!
//! ```
//!
//! # Server example
//!
//! ```no_run
//! # use tokio_util::compat::TokioAsyncReadCompatExt;
//! # use tokio_stream::{wrappers::TcpListenerStream, StreamExt};
//! # async fn doc() -> Result<(), soketto::BoxedError> {
//! use soketto::{handshake::{Server, ClientRequest, server::Response}};
//!
//! // First, we listen for incoming connections.
//! let listener = tokio::net::TcpListener::bind("...").await?;
//! let mut incoming = TcpListenerStream::new(listener);
//!
//! while let Some(socket) = incoming.next().await {
//!     // For each incoming connection we perform a handshake.
//!     let mut server = Server::new(socket?.compat());
//!
//!     let websocket_key = {
//!         let req = server.receive_request().await?;
//!         req.key()
//!     };
//!
//!     // Here we accept the client unconditionally.
//!     let accept = Response::Accept { key: websocket_key, protocol: None };
//!     server.send_response(&accept).await?;
//!
//!     // And we can finally transition to a websocket connection.
//!     let (mut sender, mut receiver) = server.into_builder().finish();
//!
//!     let mut data = Vec::new();
//!     let data_type = receiver.receive_data(&mut data).await?;
//!
//!     if data_type.is_text() {
//!         sender.send_text(std::str::from_utf8(&data)?).await?
//!     } else {
//!         sender.send_binary(&data).await?
//!     }
//!
//!     sender.close().await?
//! }
//!
//! # Ok(())
//! # }
//!
//! ```
//!
//! See `examples/hyper_server.rs` from this crate's repository for an example of
//! starting up a WebSocket server alongside an Hyper HTTP server.
//!
//! [client]: handshake::Client
//! [server]: handshake::Server
//! [Sender]: connection::Sender
//! [Receiver]: connection::Receiver
//! [rfc6455]: https://tools.ietf.org/html/rfc6455
//! [handshake]: https://tools.ietf.org/html/rfc6455#section-4

#![forbid(unsafe_code)]

pub mod base;
pub mod connection;
pub mod data;
pub mod extension;
pub mod handshake;

use bytes::BytesMut;
use futures::io::{AsyncRead, AsyncReadExt};
use std::io;

pub use connection::{Mode, Receiver, Sender};
pub use data::{Data, Incoming};

pub type BoxedError = Box<dyn std::error::Error + Send + Sync>;

/// A parsing result.
#[derive(Debug, Clone)]
pub enum Parsing<T, N = ()> {
	/// Parsing completed.
	Done {
		/// The parsed value.
		value: T,
		/// The offset into the byte slice that has been consumed.
		offset: usize,
	},
	/// Parsing is incomplete and needs more data.
	NeedMore(N),
}

/// A buffer type used for implementing `Extension`s.
#[derive(Debug)]
pub enum Storage<'a> {
	/// A read-only shared byte slice.
	Shared(&'a [u8]),
	/// A mutable byte slice.
	Unique(&'a mut [u8]),
	/// An owned byte buffer.
	Owned(Vec<u8>),
}

impl AsRef<[u8]> for Storage<'_> {
	fn as_ref(&self) -> &[u8] {
		match self {
			Storage::Shared(d) => d,
			Storage::Unique(d) => d,
			Storage::Owned(b) => b.as_ref(),
		}
	}
}

/// Helper function to allow casts from `usize` to `u64` only on platforms
/// where the sizes are guaranteed to fit.
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
const fn as_u64(a: usize) -> u64 {
	a as u64
}

/// Fill the buffer from the given `AsyncRead` impl with up to `max` bytes.
async fn read<R>(reader: &mut R, dest: &mut BytesMut, max: usize) -> io::Result<()>
where
	R: AsyncRead + Unpin,
{
	let i = dest.len();
	dest.resize(i + max, 0u8);
	let n = reader.read(&mut dest[i..]).await?;
	dest.truncate(i + n);
	if n == 0 {
		return Err(io::ErrorKind::UnexpectedEof.into());
	}
	log::trace!("read {} bytes", n);
	Ok(())
}
