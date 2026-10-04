use super::UnixNetwork;
use futures::Stream;
use std::fmt::{Debug, Formatter};
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

/// The address of a Unix domain socket.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnixSocketAddr {
    /// A socket address stored in the filesystem.
    Pathname(PathBuf),
    /// A socket address in the abstract namespace.
    Abstract(Vec<u8>),
    /// An unnamed socket address.
    Unnamed,
}

impl UnixSocketAddr {
    /// Returns the socket pathname, if it has one.
    pub fn as_pathname(&self) -> Option<&Path> {
        match self {
            Self::Pathname(path) => Some(path),
            _ => None,
        }
    }

    /// Returns the abstract socket name, if it has one.
    pub fn as_abstract_name(&self) -> Option<&[u8]> {
        match self {
            Self::Abstract(name) => Some(name),
            _ => None,
        }
    }

    /// Returns whether this address is unnamed.
    pub fn is_unnamed(&self) -> bool {
        matches!(self, Self::Unnamed)
    }

    fn from_std(address: std::os::unix::net::SocketAddr) -> Self {
        if let Some(path) = address.as_pathname() {
            return Self::Pathname(path.to_owned());
        }
        if let Some(name) = std_abstract_name(&address) {
            return Self::Abstract(name.to_vec());
        }
        Self::Unnamed
    }

    #[cfg(feature = "tokio")]
    fn from_tokio(address: ::tokio::net::unix::SocketAddr) -> Self {
        Self::from_std(address.into())
    }

    #[cfg(feature = "compio")]
    fn from_compio(address: ::socket2::SockAddr) -> Self {
        if let Some(path) = address.as_pathname() {
            return Self::Pathname(path.to_owned());
        }
        if let Some(name) = address.as_abstract_namespace() {
            return Self::Abstract(name.to_vec());
        }
        Self::Unnamed
    }
}

impl From<std::os::unix::net::SocketAddr> for UnixSocketAddr {
    fn from(address: std::os::unix::net::SocketAddr) -> Self {
        Self::from_std(address)
    }
}

#[cfg(feature = "tokio")]
impl From<::tokio::net::unix::SocketAddr> for UnixSocketAddr {
    fn from(address: ::tokio::net::unix::SocketAddr) -> Self {
        Self::from_tokio(address)
    }
}

#[cfg(target_os = "linux")]
fn std_abstract_name(address: &std::os::unix::net::SocketAddr) -> Option<&[u8]> {
    use std::os::linux::net::SocketAddrExt;
    address.as_abstract_name()
}

#[cfg(target_os = "android")]
fn std_abstract_name(address: &std::os::unix::net::SocketAddr) -> Option<&[u8]> {
    use std::os::android::net::SocketAddrExt;
    address.as_abstract_name()
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn std_abstract_name(_: &std::os::unix::net::SocketAddr) -> Option<&[u8]> {
    None
}

/// A Unix stream using one of the built-in networking implementations.
pub struct UnixStream {
    pub(super) inner: UnixStreamInner,
}

pub(super) enum UnixStreamInner {
    #[cfg(feature = "tokio")]
    Tokio(::tokio::net::UnixStream),
    #[cfg(feature = "smol")]
    Smol(::smol::net::unix::UnixStream),
    #[cfg(feature = "compio")]
    Compio(Pin<Box<::compio::io::compat::AsyncStream<::compio::net::UnixStream>>>),
}

impl UnixStream {
    /// Connects to a Unix socket path.
    pub async fn connect(path: impl AsRef<Path>) -> io::Result<Self> {
        super::current().connect_unix(path).await
    }

    #[cfg(feature = "tokio")]
    pub(super) fn from_tokio(stream: ::tokio::net::UnixStream) -> Self {
        Self {
            inner: UnixStreamInner::Tokio(stream),
        }
    }

    #[cfg(feature = "smol")]
    pub(super) fn from_smol(stream: ::smol::net::unix::UnixStream) -> Self {
        Self {
            inner: UnixStreamInner::Smol(stream),
        }
    }

    #[cfg(feature = "compio")]
    pub(super) fn from_compio(stream: ::compio::net::UnixStream) -> Self {
        Self {
            inner: UnixStreamInner::Compio(Box::pin(::compio::io::compat::AsyncStream::new(
                stream,
            ))),
        }
    }

    /// Returns the local address of this stream.
    pub fn local_addr(&self) -> io::Result<UnixSocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UnixStreamInner::Tokio(stream) => stream.local_addr().map(UnixSocketAddr::from_tokio),
            #[cfg(feature = "smol")]
            UnixStreamInner::Smol(stream) => stream.local_addr().map(UnixSocketAddr::from_std),
            #[cfg(feature = "compio")]
            UnixStreamInner::Compio(stream) => compio_stream(stream)
                .local_addr()
                .map(UnixSocketAddr::from_compio),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns the peer address of this stream.
    pub fn peer_addr(&self) -> io::Result<UnixSocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UnixStreamInner::Tokio(stream) => stream.peer_addr().map(UnixSocketAddr::from_tokio),
            #[cfg(feature = "smol")]
            UnixStreamInner::Smol(stream) => stream.peer_addr().map(UnixSocketAddr::from_std),
            #[cfg(feature = "compio")]
            UnixStreamInner::Compio(stream) => compio_stream(stream)
                .peer_addr()
                .map(UnixSocketAddr::from_compio),
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
    stream: &Pin<Box<::compio::io::compat::AsyncStream<::compio::net::UnixStream>>>,
) -> &::compio::net::UnixStream {
    stream.as_ref().get_ref().get_ref().0
}

impl Debug for UnixStream {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UnixStream")
            .field("local_addr", &self.local_addr())
            .field("peer_addr", &self.peer_addr())
            .finish()
    }
}

/// A Unix listener using one of the built-in networking implementations.
pub struct UnixListener {
    inner: UnixListenerInner,
}

enum UnixListenerInner {
    #[cfg(feature = "tokio")]
    Tokio(::tokio::net::UnixListener),
    #[cfg(feature = "smol")]
    Smol(::smol::net::unix::UnixListener),
    #[cfg(feature = "compio")]
    Compio(::compio::net::UnixListener),
}

impl UnixListener {
    /// Binds a listener to a Unix socket path.
    pub async fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        super::current().bind_unix(path).await
    }

    #[cfg(feature = "tokio")]
    pub(super) fn from_tokio(listener: ::tokio::net::UnixListener) -> Self {
        Self {
            inner: UnixListenerInner::Tokio(listener),
        }
    }

    #[cfg(feature = "smol")]
    pub(super) fn from_smol(listener: ::smol::net::unix::UnixListener) -> Self {
        Self {
            inner: UnixListenerInner::Smol(listener),
        }
    }

    #[cfg(feature = "compio")]
    pub(super) fn from_compio(listener: ::compio::net::UnixListener) -> Self {
        Self {
            inner: UnixListenerInner::Compio(listener),
        }
    }

    /// Accepts a new connection.
    pub async fn accept(&self) -> io::Result<(UnixStream, UnixSocketAddr)> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UnixListenerInner::Tokio(listener) => {
                let (stream, address) = listener.accept().await?;
                Ok((UnixStream::from_tokio(stream), address.into()))
            }
            #[cfg(feature = "smol")]
            UnixListenerInner::Smol(listener) => {
                let (stream, address) = listener.accept().await?;
                Ok((UnixStream::from_smol(stream), address.into()))
            }
            #[cfg(feature = "compio")]
            UnixListenerInner::Compio(listener) => {
                let (stream, address) = listener.accept().await?;
                Ok((
                    UnixStream::from_compio(stream),
                    UnixSocketAddr::from_compio(address),
                ))
            }
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns a stream of incoming connections.
    pub fn incoming(&self) -> UnixIncoming<'_> {
        let inner = match &self.inner {
            #[cfg(feature = "tokio")]
            UnixListenerInner::Tokio(listener) => UnixIncomingInner::Tokio(listener),
            #[cfg(feature = "smol")]
            UnixListenerInner::Smol(listener) => UnixIncomingInner::Smol(listener.incoming()),
            #[cfg(feature = "compio")]
            UnixListenerInner::Compio(listener) => UnixIncomingInner::Compio(listener.incoming()),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        };
        UnixIncoming { inner }
    }

    /// Returns the local address of this listener.
    pub fn local_addr(&self) -> io::Result<UnixSocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UnixListenerInner::Tokio(listener) => {
                listener.local_addr().map(UnixSocketAddr::from_tokio)
            }
            #[cfg(feature = "smol")]
            UnixListenerInner::Smol(listener) => {
                listener.local_addr().map(UnixSocketAddr::from_std)
            }
            #[cfg(feature = "compio")]
            UnixListenerInner::Compio(listener) => {
                listener.local_addr().map(UnixSocketAddr::from_compio)
            }
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }
}

impl Debug for UnixListener {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UnixListener")
            .field("local_addr", &self.local_addr())
            .finish()
    }
}

/// A stream of incoming Unix connections.
pub struct UnixIncoming<'a> {
    inner: UnixIncomingInner<'a>,
}

enum UnixIncomingInner<'a> {
    #[cfg(feature = "tokio")]
    Tokio(&'a ::tokio::net::UnixListener),
    #[cfg(feature = "smol")]
    Smol(::smol::net::unix::Incoming<'a>),
    #[cfg(feature = "compio")]
    Compio(::compio::net::UnixIncoming<'a>),
}

impl Stream for UnixIncoming<'_> {
    type Item = io::Result<UnixStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.get_mut().inner {
            #[cfg(feature = "tokio")]
            UnixIncomingInner::Tokio(listener) => listener
                .poll_accept(cx)
                .map(|result| Some(result.map(|(stream, _)| UnixStream::from_tokio(stream)))),
            #[cfg(feature = "smol")]
            UnixIncomingInner::Smol(incoming) => Pin::new(incoming)
                .poll_next(cx)
                .map(|item| item.map(|result| result.map(UnixStream::from_smol))),
            #[cfg(feature = "compio")]
            UnixIncomingInner::Compio(incoming) => Pin::new(incoming)
                .poll_next(cx)
                .map(|item| item.map(|result| result.map(UnixStream::from_compio))),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }
}
