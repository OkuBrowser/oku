use super::Network;
#[cfg(unix)]
use super::UnixNetwork;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
#[cfg(unix)]
use std::path::Path;

/// Compio's networking implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompioNetwork;

impl Network for CompioNetwork {
    type TcpSocket = ::compio::net::TcpSocket;
    type TcpStream = ::compio::net::TcpStream;
    type TcpListener = ::compio::net::TcpListener;
    type UdpSocket = ::compio::net::UdpSocket;

    fn new_tcp_socket_v4(&self) -> impl Future<Output = io::Result<Self::TcpSocket>> {
        ::compio::net::TcpSocket::new_v4()
    }

    fn new_tcp_socket_v6(&self) -> impl Future<Output = io::Result<Self::TcpSocket>> {
        ::compio::net::TcpSocket::new_v6()
    }

    fn connect_tcp(
        &self,
        address: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> {
        ::compio::net::TcpStream::connect(address)
    }

    fn bind_tcp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::TcpListener>> {
        ::compio::net::TcpListener::bind(address)
    }

    fn bind_udp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::UdpSocket>> {
        ::compio::net::UdpSocket::bind(address)
    }
}

#[cfg(unix)]
impl UnixNetwork for CompioNetwork {
    type UnixStream = ::compio::net::UnixStream;
    type UnixListener = ::compio::net::UnixListener;

    fn connect_unix<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Self::UnixStream>> {
        ::compio::net::UnixStream::connect(path)
    }

    fn bind_unix<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Self::UnixListener>> {
        ::compio::net::UnixListener::bind(path)
    }
}
