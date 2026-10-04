use super::Network;
#[cfg(unix)]
use super::UnixNetwork;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
#[cfg(unix)]
use std::path::Path;
use std::sync::Arc;

/// Smol's networking implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct SmolNetwork;

impl Network for SmolNetwork {
    type TcpSocket = SmolTcpSocket;
    type TcpStream = ::smol::net::TcpStream;
    type TcpListener = ::smol::net::TcpListener;
    type UdpSocket = ::smol::net::UdpSocket;

    async fn new_tcp_socket_v4(&self) -> io::Result<Self::TcpSocket> {
        SmolTcpSocket::new(::socket2::Domain::IPV4)
    }

    async fn new_tcp_socket_v6(&self) -> io::Result<Self::TcpSocket> {
        SmolTcpSocket::new(::socket2::Domain::IPV6)
    }

    fn connect_tcp(
        &self,
        address: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> {
        ::smol::net::TcpStream::connect(address)
    }

    fn bind_tcp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::TcpListener>> {
        ::smol::net::TcpListener::bind(address)
    }

    fn bind_udp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::UdpSocket>> {
        ::smol::net::UdpSocket::bind(address)
    }
}

#[cfg(unix)]
impl UnixNetwork for SmolNetwork {
    type UnixStream = ::smol::net::unix::UnixStream;
    type UnixListener = ::smol::net::unix::UnixListener;

    async fn connect_unix<P: AsRef<Path>>(&self, path: P) -> io::Result<Self::UnixStream> {
        ::smol::net::unix::UnixStream::connect(path).await
    }

    async fn bind_unix<P: AsRef<Path>>(&self, path: P) -> io::Result<Self::UnixListener> {
        ::smol::net::unix::UnixListener::bind(path)
    }
}

/// A configurable TCP socket for Smol.
#[derive(Debug)]
pub struct SmolTcpSocket {
    inner: ::socket2::Socket,
}

impl SmolTcpSocket {
    fn new(domain: ::socket2::Domain) -> io::Result<Self> {
        let inner = ::socket2::Socket::new(
            domain,
            ::socket2::Type::STREAM,
            Some(::socket2::Protocol::TCP),
        )?;
        inner.set_nonblocking(true)?;
        Ok(Self { inner })
    }

    pub(super) fn bind(&self, address: SocketAddr) -> io::Result<()> {
        self.inner.bind(&address.into())
    }

    pub(super) async fn connect(self, address: SocketAddr) -> io::Result<::smol::net::TcpStream> {
        match self.inner.connect(&address.into()) {
            Ok(()) => {}
            Err(error) if connect_pending(&error) => {}
            Err(error) => return Err(error),
        }

        let stream = ::smol::net::TcpStream::try_from(std::net::TcpStream::from(self.inner))?;
        let inner: Arc<::smol::Async<std::net::TcpStream>> = stream.clone().into();
        inner.writable().await?;
        if let Some(error) = inner.get_ref().take_error()? {
            return Err(error);
        }
        Ok(stream)
    }

    pub(super) fn listen(self, backlog: u32) -> io::Result<::smol::net::TcpListener> {
        let backlog = i32::try_from(backlog)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "backlog is too large"))?;
        self.inner.listen(backlog)?;
        ::smol::net::TcpListener::try_from(std::net::TcpListener::from(self.inner))
    }

    pub(super) fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner
            .local_addr()?
            .as_socket()
            .ok_or_else(|| io::Error::other("socket does not have an IP address"))
    }

    pub(super) fn keepalive(&self) -> io::Result<bool> {
        self.inner.keepalive()
    }

    pub(super) fn set_keepalive(&self, keepalive: bool) -> io::Result<()> {
        self.inner.set_keepalive(keepalive)
    }

    pub(super) fn reuseaddr(&self) -> io::Result<bool> {
        self.inner.reuse_address()
    }

    pub(super) fn set_reuseaddr(&self, reuseaddr: bool) -> io::Result<()> {
        self.inner.set_reuse_address(reuseaddr)
    }

    pub(super) fn nodelay(&self) -> io::Result<bool> {
        self.inner.tcp_nodelay()
    }

    pub(super) fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.inner.set_tcp_nodelay(nodelay)
    }
}

fn connect_pending(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::WouldBlock {
        return true;
    }

    #[cfg(unix)]
    {
        error.raw_os_error() == Some(::libc::EINPROGRESS)
    }

    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(10035 | 10036))
    }

    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}
