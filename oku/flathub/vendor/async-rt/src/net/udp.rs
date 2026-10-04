use super::Network;
#[cfg(feature = "compio")]
use ::compio::buf::{IntoInner, IoBuf};
use std::fmt::{Debug, Formatter};
use std::io;
use std::net::SocketAddr;

/// A UDP socket using one of the built-in networking implementations.
pub struct UdpSocket {
    inner: UdpSocketInner,
}

enum UdpSocketInner {
    #[cfg(feature = "tokio")]
    Tokio(::tokio::net::UdpSocket),
    #[cfg(feature = "smol")]
    Smol(::smol::net::UdpSocket),
    #[cfg(feature = "compio")]
    Compio(CompioUdpSocket),
}

#[cfg(feature = "compio")]
struct CompioUdpSocket {
    inner: ::compio::net::UdpSocket,
    receive_buffer: BufferSlot,
    send_buffer: BufferSlot,
}

#[cfg(feature = "compio")]
impl CompioUdpSocket {
    fn new(inner: ::compio::net::UdpSocket) -> Self {
        Self {
            inner,
            receive_buffer: BufferSlot::default(),
            send_buffer: BufferSlot::default(),
        }
    }

    async fn recv(&self, output: &mut [u8]) -> io::Result<usize> {
        let buffer = self.receive_buffer.take(output.len()).slice(..output.len());
        let ::compio::BufResult(result, buffer) = self.inner.recv(buffer).await;
        if let Ok(read) = &result {
            output[..*read].copy_from_slice(&buffer[..*read]);
        }
        self.receive_buffer.replace(buffer.into_inner());
        result
    }

    async fn recv_from(&self, output: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let buffer = self.receive_buffer.take(output.len()).slice(..output.len());
        let ::compio::BufResult(result, buffer) = self.inner.recv_from(buffer).await;
        if let Ok((read, _)) = &result {
            output[..*read].copy_from_slice(&buffer[..*read]);
        }
        self.receive_buffer.replace(buffer.into_inner());
        result
    }

    async fn send(&self, input: &[u8]) -> io::Result<usize> {
        let mut buffer = self.send_buffer.take(input.len());
        buffer.extend_from_slice(input);
        let ::compio::BufResult(result, buffer) = self.inner.send(buffer).await;
        self.send_buffer.replace(buffer);
        result
    }

    async fn send_to(&self, input: &[u8], address: SocketAddr) -> io::Result<usize> {
        let mut buffer = self.send_buffer.take(input.len());
        buffer.extend_from_slice(input);
        let ::compio::BufResult(result, buffer) = self.inner.send_to(buffer, address).await;
        self.send_buffer.replace(buffer);
        result
    }
}

#[cfg(feature = "compio")]
impl std::ops::Deref for CompioUdpSocket {
    type Target = ::compio::net::UdpSocket;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "compio")]
#[derive(Default)]
struct BufferSlot(parking_lot::Mutex<Option<Vec<u8>>>);

#[cfg(feature = "compio")]
impl BufferSlot {
    fn take(&self, capacity: usize) -> Vec<u8> {
        let mut buffer = self.0.lock().take().unwrap_or_default();
        buffer.clear();
        if buffer.capacity() < capacity {
            buffer.reserve_exact(capacity);
        }
        buffer
    }

    fn replace(&self, mut buffer: Vec<u8>) {
        buffer.clear();
        let mut current = self.0.lock();
        if current
            .as_ref()
            .is_none_or(|cached| cached.capacity() < buffer.capacity())
        {
            *current = Some(buffer);
        }
    }
}

impl UdpSocket {
    /// Binds a UDP socket to an address.
    pub async fn bind(address: SocketAddr) -> io::Result<Self> {
        super::current().bind_udp(address).await
    }

    #[cfg(feature = "tokio")]
    pub(super) fn from_tokio(socket: ::tokio::net::UdpSocket) -> Self {
        Self {
            inner: UdpSocketInner::Tokio(socket),
        }
    }

    #[cfg(feature = "smol")]
    pub(super) fn from_smol(socket: ::smol::net::UdpSocket) -> Self {
        Self {
            inner: UdpSocketInner::Smol(socket),
        }
    }

    #[cfg(feature = "compio")]
    pub(super) fn from_compio(socket: ::compio::net::UdpSocket) -> Self {
        Self {
            inner: UdpSocketInner::Compio(CompioUdpSocket::new(socket)),
        }
    }

    /// Connects this socket to a remote address.
    pub async fn connect(&self, address: SocketAddr) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.connect(address).await,
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.connect(address).await,
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.connect(address).await,
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns the local address of this socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.local_addr(),
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.local_addr(),
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.local_addr(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns the remote address of this socket.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.peer_addr(),
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.peer_addr(),
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.peer_addr(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Receives a datagram from the connected address.
    pub async fn recv(&self, buffer: &mut [u8]) -> io::Result<usize> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.recv(buffer).await,
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.recv(buffer).await,
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.recv(buffer).await,
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Receives a datagram and returns its source address.
    pub async fn recv_from(&self, buffer: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.recv_from(buffer).await,
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.recv_from(buffer).await,
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.recv_from(buffer).await,
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sends a datagram to the connected address.
    pub async fn send(&self, buffer: &[u8]) -> io::Result<usize> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.send(buffer).await,
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.send(buffer).await,
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.send(buffer).await,
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sends a datagram to an address.
    pub async fn send_to(&self, buffer: &[u8], address: SocketAddr) -> io::Result<usize> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.send_to(buffer, address).await,
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.send_to(buffer, address).await,
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.send_to(buffer, address).await,
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns whether broadcast is enabled.
    pub fn broadcast(&self) -> io::Result<bool> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.broadcast(),
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.broadcast(),
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.broadcast(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sets whether broadcast is enabled.
    pub fn set_broadcast(&self, broadcast: bool) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.set_broadcast(broadcast),
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.set_broadcast(broadcast),
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.set_broadcast(broadcast),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Returns the time-to-live value used by this socket.
    pub fn ttl(&self) -> io::Result<u32> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.ttl(),
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.ttl(),
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.ttl_v4(),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }

    /// Sets the time-to-live value used by this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        match &self.inner {
            #[cfg(feature = "tokio")]
            UdpSocketInner::Tokio(socket) => socket.set_ttl(ttl),
            #[cfg(feature = "smol")]
            UdpSocketInner::Smol(socket) => socket.set_ttl(ttl),
            #[cfg(feature = "compio")]
            UdpSocketInner::Compio(socket) => socket.set_ttl_v4(ttl),
            #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
            _ => unreachable!(),
        }
    }
}

impl Debug for UdpSocket {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UdpSocket")
            .field("local_addr", &self.local_addr())
            .field("peer_addr", &self.peer_addr())
            .finish()
    }
}

#[cfg(all(test, feature = "compio"))]
mod tests {
    use super::BufferSlot;
    use ::compio::buf::{IoBuf, IoBufMut};

    #[test]
    fn compio_buffer_slot_reuses_allocation() {
        let slot = BufferSlot::default();
        let buffer = slot.take(128);
        let pointer = buffer.as_ptr();
        slot.replace(buffer);

        let buffer = slot.take(64);
        assert_eq!(buffer.as_ptr(), pointer);

        let mut buffer = buffer.slice(..64);
        assert_eq!(buffer.buf_capacity(), 64);
    }
}
