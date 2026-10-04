use super::Network;
#[cfg(unix)]
use super::UnixNetwork;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
#[cfg(unix)]
use std::path::Path;

/// Tokio's networking implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioNetwork;

impl Network for TokioNetwork {
    type TcpSocket = ::tokio::net::TcpSocket;
    type TcpStream = ::tokio::net::TcpStream;
    type TcpListener = ::tokio::net::TcpListener;
    type UdpSocket = ::tokio::net::UdpSocket;

    async fn new_tcp_socket_v4(&self) -> io::Result<Self::TcpSocket> {
        ::tokio::net::TcpSocket::new_v4()
    }

    async fn new_tcp_socket_v6(&self) -> io::Result<Self::TcpSocket> {
        ::tokio::net::TcpSocket::new_v6()
    }

    fn connect_tcp(
        &self,
        address: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> {
        ::tokio::net::TcpStream::connect(address)
    }

    fn bind_tcp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::TcpListener>> {
        ::tokio::net::TcpListener::bind(address)
    }

    fn bind_udp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::UdpSocket>> {
        ::tokio::net::UdpSocket::bind(address)
    }
}

#[cfg(unix)]
impl UnixNetwork for TokioNetwork {
    type UnixStream = ::tokio::net::UnixStream;
    type UnixListener = ::tokio::net::UnixListener;

    async fn connect_unix<P: AsRef<Path>>(&self, path: P) -> io::Result<Self::UnixStream> {
        ::tokio::net::UnixStream::connect(path).await
    }

    async fn bind_unix<P: AsRef<Path>>(&self, path: P) -> io::Result<Self::UnixListener> {
        ::tokio::net::UnixListener::bind(path)
    }
}
