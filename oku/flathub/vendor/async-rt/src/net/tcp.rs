use super::Network;
use futures::Stream;
use std::fmt::{Debug, Formatter};
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A TCP socket that can be configured before connecting or listening.
pub struct TcpSocket {
    inner: TcpSocketInner,
}

enum TcpSocketInner {
    #[cfg(feature = "tokio")]
    Tokio(::tokio::net::TcpSocket),
    #[cfg(feature = "smol")]
    Smol(super::smol::SmolTcpSocket),
    #[cfg(feature = "compio")]
    Compio(::compio::net::TcpSocket),
}

impl TcpSocket {
    /// Creates a new IPv4 TCP socket.
    pub async fn new_v4() -> io::Result<Self> {
        super::current().new_tcp_socket_v4().await
    }

    /// Creates a new IPv6 TCP socket.
    pub async fn new_v6() -> io::Result<Self> {
        super::current().new_tcp_socket_v6().await
    }

    #[cfg(feature = "tokio")]
    pub(super) fn from_tokio(socket: ::tokio::net::TcpSocket) -> Self {
        Self {
            inner: TcpSocketInner::Tokio(socket),
        }
    }

    #[cfg(feature = "smol")]
    pub(super) fn from_smol(socket: super::smol::SmolTcpSocket) -> Self {
        Self {
            inner: TcpSocketInner::Smol(socket),
        }
    }

    #[cfg(feature = "compio")]
    pub(super) fn from_compio(socket: ::compio::net::TcpSocket) -> Self {
        Self {
            inner: TcpSocketInner::Compio(socket),
        }
    }

    /// Binds the socket to an address.
    pub async fn bind(&self, address: SocketAddr) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.bind(address),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.bind(address),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => socket.bind(address).await,
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Connects the socket to a remote address.
    pub async fn connect(self, address: SocketAddr) -> io::Result<TcpStream> {
        match self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => {
                socket.connect(address).await.map(TcpStream::from_tokio)
            }
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.connect(address).await.map(TcpStream::from_smol),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => {
                socket.connect(address).await.map(TcpStream::from_compio)
            }
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Converts the socket into a TCP listener.
    pub async fn listen(self, backlog: u32) -> io::Result<TcpListener> {
        match self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.listen(backlog).map(TcpListener::from_tokio),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.listen(backlog).map(TcpListener::from_smol),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => {
                let backlog = i32::try_from(backlog).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "backlog is too large")
                })?;
                socket.listen(backlog).await.map(TcpListener::from_compio)
            }
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns the local address of this socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.local_addr(),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.local_addr(),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => socket.local_addr(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns whether keepalive is enabled.
    pub fn keepalive(&self) -> io::Result<bool> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.keepalive(),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.keepalive(),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => socket.keepalive(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sets whether keepalive is enabled.
    pub fn set_keepalive(&self, keepalive: bool) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.set_keepalive(keepalive),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.set_keepalive(keepalive),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => socket.set_keepalive(keepalive),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns whether address reuse is enabled.
    pub fn reuseaddr(&self) -> io::Result<bool> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.reuseaddr(),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.reuseaddr(),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => socket.reuseaddr(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sets whether address reuse is enabled.
    pub fn set_reuseaddr(&self, reuseaddr: bool) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.set_reuseaddr(reuseaddr),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.set_reuseaddr(reuseaddr),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => socket.set_reuseaddr(reuseaddr),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns whether `TCP_NODELAY` is enabled.
    pub fn nodelay(&self) -> io::Result<bool> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.nodelay(),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.nodelay(),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => socket.nodelay(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sets the value of `TCP_NODELAY`.
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpSocketInner::Tokio(socket) => socket.set_nodelay(nodelay),
            #[cfg(feature = "smol")]
            TcpSocketInner::Smol(socket) => socket.set_nodelay(nodelay),
            #[cfg(feature = "compio")]
            TcpSocketInner::Compio(socket) => socket.set_nodelay(nodelay),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }
}

impl Debug for TcpSocket {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TcpSocket")
            .field("local_addr", &self.local_addr())
            .finish()
    }
}

/// A TCP stream using one of the built-in networking implementations.
pub struct TcpStream {
    pub(super) inner: TcpStreamInner,
}

pub(super) enum TcpStreamInner {
    #[cfg(feature = "tokio")]
    Tokio(::tokio::net::TcpStream),
    #[cfg(feature = "smol")]
    Smol(::smol::net::TcpStream),
    #[cfg(feature = "compio")]
    Compio(Pin<Box<::compio::io::compat::AsyncStream<::compio::net::TcpStream>>>),
}

impl TcpStream {
    /// Connects to a remote address.
    pub async fn connect(address: SocketAddr) -> io::Result<Self> {
        super::current().connect_tcp(address).await
    }

    #[cfg(feature = "tokio")]
    pub(super) fn from_tokio(stream: ::tokio::net::TcpStream) -> Self {
        Self {
            inner: TcpStreamInner::Tokio(stream),
        }
    }

    #[cfg(feature = "smol")]
    pub(super) fn from_smol(stream: ::smol::net::TcpStream) -> Self {
        Self {
            inner: TcpStreamInner::Smol(stream),
        }
    }

    #[cfg(feature = "compio")]
    pub(super) fn from_compio(stream: ::compio::net::TcpStream) -> Self {
        Self {
            inner: TcpStreamInner::Compio(Box::pin(::compio::io::compat::AsyncStream::new(stream))),
        }
    }

    /// Returns the local address of this stream.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpStreamInner::Tokio(stream) => stream.local_addr(),
            #[cfg(feature = "smol")]
            TcpStreamInner::Smol(stream) => stream.local_addr(),
            #[cfg(feature = "compio")]
            TcpStreamInner::Compio(stream) => compio_stream(stream).local_addr(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns the remote address of this stream.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpStreamInner::Tokio(stream) => stream.peer_addr(),
            #[cfg(feature = "smol")]
            TcpStreamInner::Smol(stream) => stream.peer_addr(),
            #[cfg(feature = "compio")]
            TcpStreamInner::Compio(stream) => compio_stream(stream).peer_addr(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns whether `TCP_NODELAY` is enabled.
    pub fn nodelay(&self) -> io::Result<bool> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpStreamInner::Tokio(stream) => stream.nodelay(),
            #[cfg(feature = "smol")]
            TcpStreamInner::Smol(stream) => stream.nodelay(),
            #[cfg(feature = "compio")]
            TcpStreamInner::Compio(stream) => compio_stream(stream).nodelay(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sets the value of `TCP_NODELAY`.
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpStreamInner::Tokio(stream) => stream.set_nodelay(nodelay),
            #[cfg(feature = "smol")]
            TcpStreamInner::Smol(stream) => stream.set_nodelay(nodelay),
            #[cfg(feature = "compio")]
            TcpStreamInner::Compio(stream) => compio_stream(stream).set_nodelay(nodelay),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns the time-to-live value used by this stream.
    pub fn ttl(&self) -> io::Result<u32> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpStreamInner::Tokio(stream) => stream.ttl(),
            #[cfg(feature = "smol")]
            TcpStreamInner::Smol(stream) => stream.ttl(),
            #[cfg(feature = "compio")]
            TcpStreamInner::Compio(stream) => compio_stream(stream).ttl_v4(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sets the time-to-live value used by this stream.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpStreamInner::Tokio(stream) => stream.set_ttl(ttl),
            #[cfg(feature = "smol")]
            TcpStreamInner::Smol(stream) => stream.set_ttl(ttl),
            #[cfg(feature = "compio")]
            TcpStreamInner::Compio(stream) => compio_stream(stream).set_ttl_v4(ttl),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Reads bytes from the stream.
    pub async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        futures::io::AsyncReadExt::read(self, buffer).await
    }

    /// Reads enough bytes to fill the buffer.
    pub async fn read_exact(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        futures::io::AsyncReadExt::read_exact(self, buffer).await
    }

    /// Writes bytes to the stream.
    pub async fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        futures::io::AsyncWriteExt::write(self, buffer).await
    }

    /// Writes an entire buffer to the stream.
    pub async fn write_all(&mut self, buffer: &[u8]) -> io::Result<()> {
        futures::io::AsyncWriteExt::write_all(self, buffer).await
    }

    /// Flushes buffered data to the stream.
    pub async fn flush(&mut self) -> io::Result<()> {
        futures::io::AsyncWriteExt::flush(self).await
    }

    /// Shuts down the write side of the stream.
    pub async fn shutdown(&mut self) -> io::Result<()> {
        futures::io::AsyncWriteExt::close(self).await
    }
}

#[cfg(feature = "compio")]
fn compio_stream(
    stream: &Pin<Box<::compio::io::compat::AsyncStream<::compio::net::TcpStream>>>,
) -> &::compio::net::TcpStream {
    stream.as_ref().get_ref().get_ref().0
}

impl Debug for TcpStream {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TcpStream")
            .field("local_addr", &self.local_addr())
            .field("peer_addr", &self.peer_addr())
            .finish()
    }
}

/// A TCP listener using one of the built-in networking implementations.
pub struct TcpListener {
    inner: TcpListenerInner,
}

enum TcpListenerInner {
    #[cfg(feature = "tokio")]
    Tokio(::tokio::net::TcpListener),
    #[cfg(feature = "smol")]
    Smol(::smol::net::TcpListener),
    #[cfg(feature = "compio")]
    Compio(::compio::net::TcpListener),
}

impl TcpListener {
    /// Binds a listener to an address.
    pub async fn bind(address: SocketAddr) -> io::Result<Self> {
        super::current().bind_tcp(address).await
    }

    #[cfg(feature = "tokio")]
    pub(super) fn from_tokio(listener: ::tokio::net::TcpListener) -> Self {
        Self {
            inner: TcpListenerInner::Tokio(listener),
        }
    }

    #[cfg(feature = "smol")]
    pub(super) fn from_smol(listener: ::smol::net::TcpListener) -> Self {
        Self {
            inner: TcpListenerInner::Smol(listener),
        }
    }

    #[cfg(feature = "compio")]
    pub(super) fn from_compio(listener: ::compio::net::TcpListener) -> Self {
        Self {
            inner: TcpListenerInner::Compio(listener),
        }
    }

    /// Accepts a new connection.
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpListenerInner::Tokio(listener) => {
                let (stream, address) = listener.accept().await?;
                Ok((TcpStream::from_tokio(stream), address))
            }
            #[cfg(feature = "smol")]
            TcpListenerInner::Smol(listener) => {
                let (stream, address) = listener.accept().await?;
                Ok((TcpStream::from_smol(stream), address))
            }
            #[cfg(feature = "compio")]
            TcpListenerInner::Compio(listener) => {
                let (stream, address) = listener.accept().await?;
                Ok((TcpStream::from_compio(stream), address))
            }
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns a stream of incoming connections.
    pub fn incoming(&self) -> Incoming<'_> {
        let inner = match &self.inner {
            #[cfg(feature = "tokio")]
            TcpListenerInner::Tokio(listener) => IncomingInner::Tokio(listener),
            #[cfg(feature = "smol")]
            TcpListenerInner::Smol(listener) => IncomingInner::Smol(listener.incoming()),
            #[cfg(feature = "compio")]
            TcpListenerInner::Compio(listener) => IncomingInner::Compio(listener.incoming()),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        };
        Incoming { inner }
    }

    /// Returns the local address of this listener.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpListenerInner::Tokio(listener) => listener.local_addr(),
            #[cfg(feature = "smol")]
            TcpListenerInner::Smol(listener) => listener.local_addr(),
            #[cfg(feature = "compio")]
            TcpListenerInner::Compio(listener) => listener.local_addr(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns the time-to-live value used by this listener.
    pub fn ttl(&self) -> io::Result<u32> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpListenerInner::Tokio(listener) => listener.ttl(),
            #[cfg(feature = "smol")]
            TcpListenerInner::Smol(listener) => listener.ttl(),
            #[cfg(feature = "compio")]
            TcpListenerInner::Compio(listener) => listener.ttl_v4(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sets the time-to-live value used by this listener.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            TcpListenerInner::Tokio(listener) => listener.set_ttl(ttl),
            #[cfg(feature = "smol")]
            TcpListenerInner::Smol(listener) => listener.set_ttl(ttl),
            #[cfg(feature = "compio")]
            TcpListenerInner::Compio(listener) => listener.set_ttl_v4(ttl),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }
}

impl Debug for TcpListener {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TcpListener")
            .field("local_addr", &self.local_addr())
            .finish()
    }
}

/// A stream of incoming TCP connections.
pub struct Incoming<'a> {
    inner: IncomingInner<'a>,
}

enum IncomingInner<'a> {
    #[cfg(feature = "tokio")]
    Tokio(&'a ::tokio::net::TcpListener),
    #[cfg(feature = "smol")]
    Smol(::smol::net::Incoming<'a>),
    #[cfg(feature = "compio")]
    Compio(::compio::net::TcpIncoming<'a>),
}

impl Stream for Incoming<'_> {
    type Item = io::Result<TcpStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "tokio")]
            IncomingInner::Tokio(listener) => listener
                .poll_accept(cx)
                .map(|result| Some(result.map(|(stream, _)| TcpStream::from_tokio(stream)))),
            #[cfg(feature = "smol")]
            IncomingInner::Smol(incoming) => Pin::new(incoming)
                .poll_next(cx)
                .map(|item| item.map(|result| result.map(TcpStream::from_smol))),
            #[cfg(feature = "compio")]
            IncomingInner::Compio(incoming) => Pin::new(incoming)
                .poll_next(cx)
                .map(|item| item.map(|result| result.map(TcpStream::from_compio))),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }
}
