use std::{
    future::Future,
    io,
    ops::{Deref, DerefMut},
};

use bytes::Bytes;
use iroh::endpoint::{ReadExactError, VarInt};
use iroh_io::AsyncStreamReader;
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// An abstract `iroh::endpoint::SendStream`.
pub trait SendStream: Send {
    /// Send bytes to the stream. This takes a `Bytes` because iroh can directly use them.
    ///
    /// This method is not cancellation safe. Even if this does not resolve, some bytes may have been written when previously polled.
    fn send_bytes(&mut self, bytes: Bytes) -> impl Future<Output = io::Result<()>> + Send;
    /// Send that sends a fixed sized buffer.
    fn send(&mut self, buf: &[u8]) -> impl Future<Output = io::Result<()>> + Send;
    /// Sync the stream. Not needed for iroh, but needed for intermediate buffered streams such as compression.
    fn sync(&mut self) -> impl Future<Output = io::Result<()>> + Send;
    /// Reset the stream with the given error code.
    fn reset(&mut self, code: VarInt) -> io::Result<()>;
    /// Wait for the stream to be stopped, returning the error code if it was.
    fn stopped(&mut self) -> impl Future<Output = io::Result<Option<VarInt>>> + Send;
    /// Get the stream id.
    fn id(&self) -> u64;
}

/// An abstract `iroh::endpoint::RecvStream`.
pub trait RecvStream: Send {
    /// Receive up to `len` bytes from the stream, directly into a `Bytes`.
    fn recv_bytes(&mut self, len: usize) -> impl Future<Output = io::Result<Bytes>> + Send;
    /// Receive exactly `len` bytes from the stream, directly into a `Bytes`.
    ///
    /// This will return an error if the stream ends before `len` bytes are read.
    ///
    /// Note that this is different from `recv_bytes`, which will return fewer bytes if the stream ends.
    fn recv_bytes_exact(&mut self, len: usize) -> impl Future<Output = io::Result<Bytes>> + Send;
    /// Receive exactly `target.len()` bytes from the stream.
    fn recv_exact(&mut self, target: &mut [u8]) -> impl Future<Output = io::Result<()>> + Send;
    /// Stop the stream with the given error code.
    fn stop(&mut self, code: VarInt) -> io::Result<()>;
    /// Get the stream id.
    fn id(&self) -> u64;
}

impl SendStream for iroh::endpoint::SendStream {
    async fn send_bytes(&mut self, bytes: Bytes) -> io::Result<()> {
        Ok(self.write_chunk(bytes).await?)
    }

    async fn send(&mut self, buf: &[u8]) -> io::Result<()> {
        Ok(self.write_all(buf).await?)
    }

    async fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn reset(&mut self, code: VarInt) -> io::Result<()> {
        Ok(self.reset(code)?)
    }

    async fn stopped(&mut self) -> io::Result<Option<VarInt>> {
        Ok(iroh::endpoint::SendStream::stopped(self).await?)
    }

    fn id(&self) -> u64 {
        self.id().index()
    }
}

impl RecvStream for iroh::endpoint::RecvStream {
    async fn recv_bytes(&mut self, len: usize) -> io::Result<Bytes> {
        let mut buf = vec![0; len];
        match self.read_exact(&mut buf).await {
            Err(ReadExactError::FinishedEarly(n)) => {
                buf.truncate(n);
            }
            Err(ReadExactError::ReadError(e)) => {
                return Err(e.into());
            }
            Ok(()) => {}
        };
        Ok(buf.into())
    }

    async fn recv_bytes_exact(&mut self, len: usize) -> io::Result<Bytes> {
        let mut buf = vec![0; len];
        self.read_exact(&mut buf).await.map_err(|e| match e {
            ReadExactError::FinishedEarly(0) => io::Error::new(io::ErrorKind::UnexpectedEof, ""),
            ReadExactError::FinishedEarly(_) => io::Error::new(io::ErrorKind::InvalidData, ""),
            ReadExactError::ReadError(e) => e.into(),
        })?;
        Ok(buf.into())
    }

    async fn recv_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.read_exact(buf).await.map_err(|e| match e {
            ReadExactError::FinishedEarly(0) => io::Error::new(io::ErrorKind::UnexpectedEof, ""),
            ReadExactError::FinishedEarly(_) => io::Error::new(io::ErrorKind::InvalidData, ""),
            ReadExactError::ReadError(e) => e.into(),
        })
    }

    fn stop(&mut self, code: VarInt) -> io::Result<()> {
        Ok(self.stop(code)?)
    }

    fn id(&self) -> u64 {
        self.id().index()
    }
}

impl<R: RecvStream> RecvStream for &mut R {
    async fn recv_bytes(&mut self, len: usize) -> io::Result<Bytes> {
        self.deref_mut().recv_bytes(len).await
    }

    async fn recv_bytes_exact(&mut self, len: usize) -> io::Result<Bytes> {
        self.deref_mut().recv_bytes_exact(len).await
    }

    async fn recv_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.deref_mut().recv_exact(buf).await
    }

    fn stop(&mut self, code: VarInt) -> io::Result<()> {
        self.deref_mut().stop(code)
    }

    fn id(&self) -> u64 {
        self.deref().id()
    }
}

impl<W: SendStream> SendStream for &mut W {
    async fn send_bytes(&mut self, bytes: Bytes) -> io::Result<()> {
        self.deref_mut().send_bytes(bytes).await
    }

    async fn send(&mut self, buf: &[u8]) -> io::Result<()> {
        self.deref_mut().send(buf).await
    }

    async fn sync(&mut self) -> io::Result<()> {
        self.deref_mut().sync().await
    }

    fn reset(&mut self, code: VarInt) -> io::Result<()> {
        self.deref_mut().reset(code)
    }

    async fn stopped(&mut self) -> io::Result<Option<VarInt>> {
        self.deref_mut().stopped().await
    }

    fn id(&self) -> u64 {
        self.deref().id()
    }
}

#[derive(Debug)]
pub struct AsyncReadRecvStream<R>(R);

/// This is a helper trait to work with [`AsyncReadRecvStream`]. If you have an
/// `AsyncRead + Unpin + Send`, you can implement these additional methods and wrap the result
/// in an `AsyncReadRecvStream` to get a `RecvStream` that reads from the underlying `AsyncRead`.
pub trait AsyncReadRecvStreamExtra: Send {
    /// Get a mutable reference to the inner `AsyncRead`.
    ///
    /// Getting a reference is easier than implementing all methods on `AsyncWrite` with forwarders to the inner instance.
    fn inner(&mut self) -> &mut (impl AsyncRead + Unpin + Send);
    /// Stop the stream with the given error code.
    fn stop(&mut self, code: VarInt) -> io::Result<()>;
    /// A local unique identifier for the stream.
    ///
    /// This allows distinguishing between streams, but once the stream is closed, the id may be reused.
    fn id(&self) -> u64;
}

impl<R> AsyncReadRecvStream<R> {
    pub fn new(inner: R) -> Self {
        Self(inner)
    }
}

impl<R: AsyncReadRecvStreamExtra> RecvStream for AsyncReadRecvStream<R> {
    async fn recv_bytes(&mut self, len: usize) -> io::Result<Bytes> {
        let mut res = vec![0; len];
        let mut n = 0;
        loop {
            let read = self.0.inner().read(&mut res[n..]).await?;
            if read == 0 {
                res.truncate(n);
                break;
            }
            n += read;
            if n == len {
                break;
            }
        }
        Ok(res.into())
    }

    async fn recv_bytes_exact(&mut self, len: usize) -> io::Result<Bytes> {
        let mut res = vec![0; len];
        self.0.inner().read_exact(&mut res).await?;
        Ok(res.into())
    }

    async fn recv_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.0.inner().read_exact(buf).await?;
        Ok(())
    }

    fn stop(&mut self, code: VarInt) -> io::Result<()> {
        self.0.stop(code)
    }

    fn id(&self) -> u64 {
        self.0.id()
    }
}

impl RecvStream for Bytes {
    async fn recv_bytes(&mut self, len: usize) -> io::Result<Bytes> {
        let n = len.min(self.len());
        let res = self.slice(..n);
        *self = self.slice(n..);
        Ok(res)
    }

    async fn recv_bytes_exact(&mut self, len: usize) -> io::Result<Bytes> {
        if self.len() < len {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let res = self.slice(..len);
        *self = self.slice(len..);
        Ok(res)
    }

    async fn recv_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if self.len() < buf.len() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        buf.copy_from_slice(&self[..buf.len()]);
        *self = self.slice(buf.len()..);
        Ok(())
    }

    fn stop(&mut self, _code: VarInt) -> io::Result<()> {
        Ok(())
    }

    fn id(&self) -> u64 {
        0
    }
}

/// Utility to convert a [tokio::io::AsyncWrite] into an [SendStream].
#[derive(Debug, Clone)]
pub struct AsyncWriteSendStream<W>(W);

/// This is a helper trait to work with [`AsyncWriteSendStream`].
///
/// If you have an `AsyncWrite + Unpin + Send`, you can implement these additional
/// methods and wrap the result in an `AsyncWriteSendStream` to get a `SendStream`
/// that writes to the underlying `AsyncWrite`.
pub trait AsyncWriteSendStreamExtra: Send {
    /// Get a mutable reference to the inner `AsyncWrite`.
    ///
    /// Getting a reference is easier than implementing all methods on `AsyncWrite` with forwarders to the inner instance.
    fn inner(&mut self) -> &mut (impl AsyncWrite + Unpin + Send);
    /// Reset the stream with the given error code.
    fn reset(&mut self, code: VarInt) -> io::Result<()>;
    /// Wait for the stream to be stopped, returning the optional error code if it was.
    fn stopped(&mut self) -> impl Future<Output = io::Result<Option<VarInt>>> + Send;
    /// A local unique identifier for the stream.
    ///
    /// This allows distinguishing between streams, but once the stream is closed, the id may be reused.
    fn id(&self) -> u64;
}

impl<W: AsyncWriteSendStreamExtra> AsyncWriteSendStream<W> {
    pub fn new(inner: W) -> Self {
        Self(inner)
    }
}

impl<W: AsyncWriteSendStreamExtra> AsyncWriteSendStream<W> {
    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W: AsyncWriteSendStreamExtra> SendStream for AsyncWriteSendStream<W> {
    async fn send_bytes(&mut self, bytes: Bytes) -> io::Result<()> {
        self.0.inner().write_all(&bytes).await
    }

    async fn send(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.inner().write_all(buf).await
    }

    async fn sync(&mut self) -> io::Result<()> {
        self.0.inner().flush().await
    }

    fn reset(&mut self, code: VarInt) -> io::Result<()> {
        self.0.reset(code)?;
        Ok(())
    }

    async fn stopped(&mut self) -> io::Result<Option<VarInt>> {
        let res = self.0.stopped().await?;
        Ok(res)
    }

    fn id(&self) -> u64 {
        self.0.id()
    }
}

#[derive(Debug)]
pub struct RecvStreamAsyncStreamReader<R>(R);

impl<R: RecvStream> RecvStreamAsyncStreamReader<R> {
    pub fn new(inner: R) -> Self {
        Self(inner)
    }

    pub fn into_inner(self) -> R {
        self.0
    }
}

impl<R: RecvStream> AsyncStreamReader for RecvStreamAsyncStreamReader<R> {
    async fn read_bytes(&mut self, len: usize) -> io::Result<Bytes> {
        self.0.recv_bytes_exact(len).await
    }

    async fn read<const L: usize>(&mut self) -> io::Result<[u8; L]> {
        let mut buf = [0; L];
        self.0.recv_exact(&mut buf).await?;
        Ok(buf)
    }
}

pub(crate) trait RecvStreamExt: RecvStream {
    async fn expect_eof(&mut self) -> io::Result<()> {
        match self.read_u8().await {
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected data",
            )),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        self.recv_exact(&mut buf).await?;
        Ok(buf[0])
    }

    async fn read_to_end_as<T: DeserializeOwned>(
        &mut self,
        max_size: usize,
    ) -> io::Result<(T, usize)> {
        let data = self.recv_bytes(max_size).await?;
        self.expect_eof().await?;
        let value = postcard::from_bytes(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok((value, data.len()))
    }

    async fn read_length_prefixed<T: DeserializeOwned>(
        &mut self,
        max_size: usize,
    ) -> io::Result<T> {
        let Some(n) = self.read_varint_u64().await? else {
            return Err(io::ErrorKind::UnexpectedEof.into());
        };
        if n > max_size as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "length prefix too large",
            ));
        }
        let n = n as usize;
        let data = self.recv_bytes(n).await?;
        let value = postcard::from_bytes(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(value)
    }

    /// Reads a u64 varint from an AsyncRead source, using the Postcard/LEB128 format.
    ///
    /// In Postcard's varint format (LEB128):
    /// - Each byte uses 7 bits for the value
    /// - The MSB (most significant bit) of each byte indicates if there are more bytes (1) or not (0)
    /// - Values are stored in little-endian order (least significant group first)
    ///
    /// Returns the decoded u64 value.
    async fn read_varint_u64(&mut self) -> io::Result<Option<u64>> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;

        loop {
            // We can only shift up to 63 bits (for a u64)
            if shift >= 64 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Varint is too large for u64",
                ));
            }

            // Read a single byte
            let res = self.read_u8().await;
            if shift == 0 {
                if let Err(cause) = res {
                    if cause.kind() == io::ErrorKind::UnexpectedEof {
                        return Ok(None);
                    } else {
                        return Err(cause);
                    }
                }
            }

            let byte = res?;

            // Extract the 7 value bits (bits 0-6, excluding the MSB which is the continuation bit)
            let value = (byte & 0x7F) as u64;

            // Add the bits to our result at the current shift position
            result |= value << shift;

            // If the high bit is not set (0), this is the last byte
            if byte & 0x80 == 0 {
                break;
            }

            // Move to the next 7 bits
            shift += 7;
        }

        Ok(Some(result))
    }
}

impl<R: RecvStream> RecvStreamExt for R {}

pub(crate) trait SendStreamExt: SendStream {
    async fn write_length_prefixed<T: Serialize>(&mut self, value: T) -> io::Result<usize> {
        let size = postcard::experimental::serialized_size(&value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut buf = Vec::with_capacity(size + 9);
        irpc::util::WriteVarintExt::write_length_prefixed(&mut buf, value)?;
        let n = buf.len();
        self.send_bytes(buf.into()).await?;
        Ok(n)
    }
}

impl<W: SendStream> SendStreamExt for W {}
