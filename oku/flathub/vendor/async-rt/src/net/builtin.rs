use super::{Network, TcpListener, TcpSocket, TcpStream, UdpSocket};
use crate::global::BuiltinExecutor;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
#[cfg(unix)]
use std::path::Path;

#[cfg(unix)]
use super::{UnixListener, UnixNetwork, UnixStream};

/// A built-in networking implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinNetwork {
    /// Tokio's networking implementation.
    #[cfg(feature = "tokio")]
    Tokio,
    /// Smol's networking implementation.
    #[cfg(feature = "smol")]
    Smol,
    /// Compio's networking implementation.
    #[cfg(feature = "compio")]
    Compio,
    /// No networking implementation is available.
    Unavailable,
}

impl Default for BuiltinNetwork {
    fn default() -> Self {
        Self::from(BuiltinExecutor::default())
    }
}

impl From<BuiltinExecutor> for BuiltinNetwork {
    fn from(executor: BuiltinExecutor) -> Self {
        match executor {
            #[cfg(feature = "tokio")]
            BuiltinExecutor::Tokio => Self::Tokio,
            #[cfg(feature = "smol")]
            BuiltinExecutor::Smol => Self::Smol,
            #[cfg(feature = "compio")]
            BuiltinExecutor::Compio => Self::Compio,
            _ => Self::Unavailable,
        }
    }
}

impl Network for BuiltinNetwork {
    type TcpSocket = TcpSocket;
    type TcpStream = TcpStream;
    type TcpListener = TcpListener;
    type UdpSocket = UdpSocket;

    fn new_tcp_socket_v4(&self) -> impl Future<Output = io::Result<Self::TcpSocket>> {
        let network = *self;
        async move {
            match network {
                #[cfg(feature = "tokio")]
                Self::Tokio => super::TokioNetwork
                    .new_tcp_socket_v4()
                    .await
                    .map(TcpSocket::from_tokio),
                #[cfg(feature = "smol")]
                Self::Smol => super::SmolNetwork
                    .new_tcp_socket_v4()
                    .await
                    .map(TcpSocket::from_smol),
                #[cfg(feature = "compio")]
                Self::Compio => super::CompioNetwork
                    .new_tcp_socket_v4()
                    .await
                    .map(TcpSocket::from_compio),
                Self::Unavailable => Err(super::unavailable()),
            }
        }
    }

    fn new_tcp_socket_v6(&self) -> impl Future<Output = io::Result<Self::TcpSocket>> {
        let network = *self;
        async move {
            match network {
                #[cfg(feature = "tokio")]
                Self::Tokio => super::TokioNetwork
                    .new_tcp_socket_v6()
                    .await
                    .map(TcpSocket::from_tokio),
                #[cfg(feature = "smol")]
                Self::Smol => super::SmolNetwork
                    .new_tcp_socket_v6()
                    .await
                    .map(TcpSocket::from_smol),
                #[cfg(feature = "compio")]
                Self::Compio => super::CompioNetwork
                    .new_tcp_socket_v6()
                    .await
                    .map(TcpSocket::from_compio),
                Self::Unavailable => Err(super::unavailable()),
            }
        }
    }

    fn connect_tcp(
        &self,
        address: SocketAddr,
    ) -> impl Future<Output = io::Result<Self::TcpStream>> {
        let network = *self;
        async move {
            match network {
                #[cfg(feature = "tokio")]
                Self::Tokio => super::TokioNetwork
                    .connect_tcp(address)
                    .await
                    .map(TcpStream::from_tokio),
                #[cfg(feature = "smol")]
                Self::Smol => super::SmolNetwork
                    .connect_tcp(address)
                    .await
                    .map(TcpStream::from_smol),
                #[cfg(feature = "compio")]
                Self::Compio => super::CompioNetwork
                    .connect_tcp(address)
                    .await
                    .map(TcpStream::from_compio),
                Self::Unavailable => Err(super::unavailable()),
            }
        }
    }

    fn bind_tcp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::TcpListener>> {
        let network = *self;
        async move {
            match network {
                #[cfg(feature = "tokio")]
                Self::Tokio => super::TokioNetwork
                    .bind_tcp(address)
                    .await
                    .map(TcpListener::from_tokio),
                #[cfg(feature = "smol")]
                Self::Smol => super::SmolNetwork
                    .bind_tcp(address)
                    .await
                    .map(TcpListener::from_smol),
                #[cfg(feature = "compio")]
                Self::Compio => super::CompioNetwork
                    .bind_tcp(address)
                    .await
                    .map(TcpListener::from_compio),
                Self::Unavailable => Err(super::unavailable()),
            }
        }
    }

    fn bind_udp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::UdpSocket>> {
        let network = *self;
        async move {
            match network {
                #[cfg(feature = "tokio")]
                Self::Tokio => super::TokioNetwork
                    .bind_udp(address)
                    .await
                    .map(UdpSocket::from_tokio),
                #[cfg(feature = "smol")]
                Self::Smol => super::SmolNetwork
                    .bind_udp(address)
                    .await
                    .map(UdpSocket::from_smol),
                #[cfg(feature = "compio")]
                Self::Compio => super::CompioNetwork
                    .bind_udp(address)
                    .await
                    .map(UdpSocket::from_compio),
                Self::Unavailable => Err(super::unavailable()),
            }
        }
    }
}

#[cfg(unix)]
impl UnixNetwork for BuiltinNetwork {
    type UnixStream = UnixStream;
    type UnixListener = UnixListener;

    fn connect_unix<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Self::UnixStream>> {
        let network = *self;
        async move {
            match network {
                #[cfg(feature = "tokio")]
                Self::Tokio => super::TokioNetwork
                    .connect_unix(path)
                    .await
                    .map(UnixStream::from_tokio),
                #[cfg(feature = "smol")]
                Self::Smol => super::SmolNetwork
                    .connect_unix(path)
                    .await
                    .map(UnixStream::from_smol),
                #[cfg(feature = "compio")]
                Self::Compio => super::CompioNetwork
                    .connect_unix(path)
                    .await
                    .map(UnixStream::from_compio),
                Self::Unavailable => Err(super::unavailable()),
            }
        }
    }

    fn bind_unix<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Self::UnixListener>> {
        let network = *self;
        async move {
            match network {
                #[cfg(feature = "tokio")]
                Self::Tokio => super::TokioNetwork
                    .bind_unix(path)
                    .await
                    .map(UnixListener::from_tokio),
                #[cfg(feature = "smol")]
                Self::Smol => super::SmolNetwork
                    .bind_unix(path)
                    .await
                    .map(UnixListener::from_smol),
                #[cfg(feature = "compio")]
                Self::Compio => super::CompioNetwork
                    .bind_unix(path)
                    .await
                    .map(UnixListener::from_compio),
                Self::Unavailable => Err(super::unavailable()),
            }
        }
    }
}
