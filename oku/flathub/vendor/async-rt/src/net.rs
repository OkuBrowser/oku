//! Asynchronous networking.

use std::future::Future;
use std::io;
use std::net::SocketAddr;
#[cfg(unix)]
use std::path::Path;

mod builtin;
pub(crate) use builtin::BuiltinNetwork;
#[path = "net/io.rs"]
mod stream_io;
mod tcp;
pub use tcp::{Incoming, TcpListener, TcpSocket, TcpStream};
mod udp;
pub use udp::UdpSocket;
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{UnixIncoming, UnixListener, UnixSocketAddr, UnixStream};
#[cfg(unix)]
#[path = "net/unix_io.rs"]
mod unix_stream_io;

#[cfg(feature = "compio")]
pub mod compio;
#[cfg(feature = "compio")]
pub use compio::CompioNetwork;
#[cfg(feature = "smol")]
pub mod smol;
#[cfg(feature = "smol")]
pub use smol::SmolNetwork;
#[cfg(feature = "tokio")]
pub mod tokio;
#[cfg(feature = "tokio")]
pub use tokio::TokioNetwork;

/// A networking implementation.
pub trait Network: Send + Sync {
    /// The configurable TCP socket returned by this implementation.
    type TcpSocket: 'static;

    /// The TCP stream returned by this implementation.
    type TcpStream: 'static;

    /// The TCP listener returned by this implementation.
    type TcpListener: 'static;

    /// The UDP socket returned by this implementation.
    type UdpSocket: 'static;

    /// Creates a new IPv4 TCP socket.
    fn new_tcp_socket_v4(&self) -> impl Future<Output = io::Result<Self::TcpSocket>>;

    /// Creates a new IPv6 TCP socket.
    fn new_tcp_socket_v6(&self) -> impl Future<Output = io::Result<Self::TcpSocket>>;

    /// Connects a TCP stream to an address.
    fn connect_tcp(&self, address: SocketAddr)
    -> impl Future<Output = io::Result<Self::TcpStream>>;

    /// Binds a TCP listener to an address.
    fn bind_tcp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::TcpListener>>;

    /// Binds a UDP socket to an address.
    fn bind_udp(&self, address: SocketAddr) -> impl Future<Output = io::Result<Self::UdpSocket>>;
}

/// A networking implementation that supports Unix domain sockets.
#[cfg(unix)]
pub trait UnixNetwork: Network {
    /// The Unix stream returned by this implementation.
    type UnixStream: 'static;

    /// The Unix listener returned by this implementation.
    type UnixListener: 'static;

    /// Connects a Unix stream to a socket path.
    fn connect_unix<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Self::UnixStream>>;

    /// Binds a Unix listener to a socket path.
    fn bind_unix<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Self::UnixListener>>;
}

fn current() -> BuiltinNetwork {
    crate::task::executor().into()
}

fn unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "no networking backend is available for the active executor",
    )
}

#[cfg(test)]
mod tests {
    use super::{TcpListener, TcpSocket, TcpStream, UdpSocket};
    #[cfg(unix)]
    use super::{UnixListener, UnixStream};
    use crate::ExecutorBlockOn;
    use crate::global::BuiltinExecutor;
    use futures::StreamExt;
    use std::net::SocketAddr;
    #[cfg(unix)]
    use std::path::PathBuf;
    #[cfg(unix)]
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    struct SocketPath(PathBuf);

    #[cfg(unix)]
    impl SocketPath {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let name = format!("art-{}-{id}.sock", std::process::id());
            Self(std::env::temp_dir().join(name))
        }
    }

    #[cfg(unix)]
    impl Drop for SocketPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    async fn tcp_round_trip() {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let (client, accepted) =
            futures::future::try_join(TcpStream::connect(address), listener.accept())
                .await
                .unwrap();
        let (mut server, peer) = accepted;
        let mut client = client;

        assert_eq!(client.peer_addr().unwrap(), address);
        assert_eq!(server.peer_addr().unwrap(), peer);
        client.set_nodelay(true).unwrap();
        assert!(client.nodelay().unwrap());

        let client_exchange = async {
            client.write_all(b"ping").await?;
            client.flush().await?;
            let mut response = [0; 4];
            client.read_exact(&mut response).await?;
            std::io::Result::Ok(response)
        };
        let server_exchange = async {
            server.write_all(b"pong").await?;
            server.flush().await?;
            let mut request = [0; 4];
            server.read_exact(&mut request).await?;
            std::io::Result::Ok(request)
        };
        let (response, request) = futures::future::try_join(client_exchange, server_exchange)
            .await
            .unwrap();
        assert_eq!(&request, b"ping");
        assert_eq!(&response, b"pong");

        client.shutdown().await.unwrap();
        let mut eof = [0; 1];
        assert_eq!(server.read(&mut eof).await.unwrap(), 0);

        let mut incoming = listener.incoming();
        let (_, next) = futures::future::try_join(TcpStream::connect(address), async {
            incoming.next().await.unwrap()
        })
        .await
        .unwrap();
        assert_eq!(next.local_addr().unwrap(), address);
    }

    async fn udp_round_trip() {
        let first = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let second = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let first_address = first.local_addr().unwrap();
        let second_address = second.local_addr().unwrap();

        assert_eq!(first.send_to(b"ping", second_address).await.unwrap(), 4);
        let mut request = [0; 4];
        let (read, sender) = second.recv_from(&mut request).await.unwrap();
        assert_eq!(read, 4);
        assert_eq!(sender, first_address);
        assert_eq!(&request, b"ping");

        first.connect(second_address).await.unwrap();
        second.connect(first_address).await.unwrap();
        assert_eq!(second.send(b"pong").await.unwrap(), 4);
        let mut response = [0; 4];
        assert_eq!(first.recv(&mut response).await.unwrap(), 4);
        assert_eq!(&response, b"pong");
    }

    async fn tcp_socket_round_trip() {
        let socket = TcpSocket::new_v4().await.unwrap();
        socket.set_reuseaddr(true).unwrap();
        socket
            .bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let address = socket.local_addr().unwrap();
        let listener = socket.listen(32).await.unwrap();
        let client_socket = TcpSocket::new_v4().await.unwrap();
        client_socket.set_nodelay(true).unwrap();
        let (mut client, (mut server, _)) =
            futures::future::try_join(client_socket.connect(address), listener.accept())
                .await
                .unwrap();

        client.write_all(b"socket").await.unwrap();
        client.flush().await.unwrap();
        let mut received = [0; 6];
        server.read_exact(&mut received).await.unwrap();
        assert_eq!(&received, b"socket");
    }

    #[cfg(unix)]
    async fn unix_round_trip() {
        let path = SocketPath::new();
        let listener = UnixListener::bind(&path.0).await.unwrap();
        assert_eq!(
            listener.local_addr().unwrap().as_pathname(),
            Some(path.0.as_path())
        );

        let (mut client, (mut server, _)) =
            futures::future::try_join(UnixStream::connect(&path.0), listener.accept())
                .await
                .unwrap();
        client.write_all(b"unix").await.unwrap();
        client.flush().await.unwrap();
        let mut received = [0; 4];
        server.read_exact(&mut received).await.unwrap();
        assert_eq!(&received, b"unix");

        let mut incoming = listener.incoming();
        let (_, next) = futures::future::try_join(UnixStream::connect(&path.0), async {
            incoming.next().await.unwrap()
        })
        .await
        .unwrap();
        assert_eq!(
            next.local_addr().unwrap().as_pathname(),
            Some(path.0.as_path())
        );
    }

    #[cfg(feature = "tokio")]
    #[test]
    fn tokio_network() {
        let runtime = crate::rt::tokio::TokioRuntimeExecutor::with_single_thread().unwrap();
        let _guard = crate::task::set_executor(BuiltinExecutor::Tokio);
        runtime.block_on(async {
            tcp_round_trip().await;
            udp_round_trip().await;
            tcp_socket_round_trip().await;
            #[cfg(unix)]
            unix_round_trip().await;
        });
    }

    #[cfg(feature = "smol")]
    #[test]
    fn smol_network() {
        let _guard = crate::task::set_executor(BuiltinExecutor::Smol);
        crate::rt::smol::SmolExecutor.block_on(async {
            tcp_round_trip().await;
            udp_round_trip().await;
            tcp_socket_round_trip().await;
            #[cfg(unix)]
            unix_round_trip().await;
        });
    }

    #[cfg(feature = "compio")]
    #[test]
    fn compio_network() {
        let runtime = crate::rt::compio::CompioRuntimeExecutor::new().unwrap();
        let _guard = crate::task::set_executor(BuiltinExecutor::Compio);
        runtime.block_on(async {
            tcp_round_trip().await;
            udp_round_trip().await;
            tcp_socket_round_trip().await;
            #[cfg(unix)]
            unix_round_trip().await;
        });
    }

    #[cfg(feature = "tokio")]
    #[test]
    fn tcp_stream_supports_tokio_io() {
        use ::tokio::io::{AsyncReadExt, AsyncWriteExt};

        let runtime = crate::rt::tokio::TokioRuntimeExecutor::with_single_thread().unwrap();
        let _guard = crate::task::set_executor(BuiltinExecutor::Tokio);
        runtime.block_on(async {
            let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
                .await
                .unwrap();
            let address = listener.local_addr().unwrap();
            let (mut client, (mut server, _)) =
                futures::future::try_join(TcpStream::connect(address), listener.accept())
                    .await
                    .unwrap();

            AsyncWriteExt::write_all(&mut client, b"hello")
                .await
                .unwrap();
            let mut received = [0; 5];
            AsyncReadExt::read_exact(&mut server, &mut received)
                .await
                .unwrap();
            assert_eq!(&received, b"hello");
        });
    }
}
