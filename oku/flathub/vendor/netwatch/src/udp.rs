use std::{
    future::Future,
    io,
    net::SocketAddr,
    num::NonZeroUsize,
    pin::Pin,
    sync::{Arc, RwLock, RwLockReadGuard, TryLockError, atomic::AtomicBool},
    task::{Context, Poll},
};

use atomic_waker::AtomicWaker;
use noq_udp::Transmit;
use tokio::io::Interest;
use tracing::{debug, trace, warn};

use super::IpFamily;

/// Wrapper around a tokio UDP socket.
#[derive(Debug)]
pub struct UdpSocket {
    socket: RwLock<SocketState>,
    recv_waker: AtomicWaker,
    send_waker: AtomicWaker,
    /// Set to true, when an error occurred, that means we need to rebind the socket.
    is_broken: AtomicBool,
}

/// UDP socket read/write buffer size (7MB). The value of 7MB is chosen as it
/// is the max supported by a default configuration of macOS. Some platforms will silently clamp the value.
const SOCKET_BUFFER_SIZE: usize = 7 << 20;
impl UdpSocket {
    /// Bind only Ipv4 on any interface.
    pub fn bind_v4(port: u16) -> io::Result<Self> {
        Self::bind(IpFamily::V4, port)
    }

    /// Bind only Ipv6 on any interface.
    pub fn bind_v6(port: u16) -> io::Result<Self> {
        Self::bind(IpFamily::V6, port)
    }

    /// Bind only Ipv4 on localhost.
    pub fn bind_local_v4(port: u16) -> io::Result<Self> {
        Self::bind_local(IpFamily::V4, port)
    }

    /// Bind only Ipv6 on localhost.
    pub fn bind_local_v6(port: u16) -> io::Result<Self> {
        Self::bind_local(IpFamily::V6, port)
    }

    /// Bind to the given port only on localhost.
    pub fn bind_local(network: IpFamily, port: u16) -> io::Result<Self> {
        let addr = SocketAddr::new(network.local_addr(), port);
        Self::bind_raw(addr)
    }

    /// Bind to the given port and listen on all interfaces.
    pub fn bind(network: IpFamily, port: u16) -> io::Result<Self> {
        let addr = SocketAddr::new(network.unspecified_addr(), port);
        Self::bind_raw(addr)
    }

    /// Bind to any provided [`SocketAddr`].
    pub fn bind_full(addr: impl Into<SocketAddr>) -> io::Result<Self> {
        Self::bind_raw(addr)
    }

    /// Is the socket broken and needs a rebind?
    pub fn is_broken(&self) -> bool {
        self.is_broken.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Marks this socket as needing a rebind
    fn mark_broken(&self) {
        self.is_broken
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Rebind the underlying socket.
    pub fn rebind(&self) -> io::Result<()> {
        {
            let mut guard = self.socket.write().unwrap();
            guard.rebind()?;

            // Clear errors
            self.is_broken
                .store(false, std::sync::atomic::Ordering::Release);

            drop(guard);
        }

        // wakeup
        self.wake_all();

        Ok(())
    }

    fn bind_raw(addr: impl Into<SocketAddr>) -> io::Result<Self> {
        let socket = SocketState::bind(addr.into())?;

        Ok(UdpSocket {
            socket: RwLock::new(socket),
            recv_waker: AtomicWaker::default(),
            send_waker: AtomicWaker::default(),
            is_broken: AtomicBool::new(false),
        })
    }

    /// Receives a single datagram message on the socket from the remote address
    /// to which it is connected. On success, returns the number of bytes read.
    ///
    /// The function must be called with valid byte array `buf` of sufficient
    /// size to hold the message bytes. If a message is too long to fit in the
    /// supplied buffer, excess bytes may be discarded.
    ///
    /// The [`connect`] method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    ///
    /// [`connect`]: method@Self::connect
    pub fn recv<'a, 'b>(&'b self, buffer: &'a mut [u8]) -> RecvFut<'a, 'b> {
        RecvFut {
            socket: self,
            buffer,
        }
    }

    /// Receives a single datagram message on the socket. On success, returns
    /// the number of bytes read and the origin.
    ///
    /// The function must be called with valid byte array `buf` of sufficient
    /// size to hold the message bytes. If a message is too long to fit in the
    /// supplied buffer, excess bytes may be discarded.
    pub fn recv_from<'a, 'b>(&'b self, buffer: &'a mut [u8]) -> RecvFromFut<'a, 'b> {
        RecvFromFut {
            socket: self,
            buffer,
        }
    }

    /// Sends data on the socket to the remote address that the socket is
    /// connected to.
    ///
    /// The [`connect`] method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    ///
    /// [`connect`]: method@Self::connect
    ///
    /// # Return
    ///
    /// On success, the number of bytes sent is returned, otherwise, the
    /// encountered error is returned.
    pub fn send<'a, 'b>(&'b self, buffer: &'a [u8]) -> SendFut<'a, 'b> {
        SendFut {
            socket: self,
            buffer,
        }
    }

    /// Sends data on the socket to the given address. On success, returns the
    /// number of bytes written.
    pub fn send_to<'a, 'b>(&'b self, buffer: &'a [u8], to: SocketAddr) -> SendToFut<'a, 'b> {
        SendToFut {
            socket: self,
            buffer,
            to,
        }
    }

    /// Connects the UDP socket setting the default destination for send() and
    /// limiting packets that are read via `recv` from the address specified in
    /// `addr`.
    pub fn connect(&self, addr: SocketAddr) -> io::Result<()> {
        trace!(%addr, "connecting");
        let guard = self.socket.read().unwrap();
        let (socket_tokio, _state) = guard.try_get_connected()?;

        let sock_ref = socket2::SockRef::from(&socket_tokio);
        sock_ref.connect(&socket2::SockAddr::from(addr))?;

        Ok(())
    }

    /// Returns the local address of this socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let guard = self.socket.read().unwrap();
        let (socket, _state) = guard.try_get_connected()?;

        socket.local_addr()
    }

    /// Closes the socket, and waits for the underlying `libc::close` call to be finished.
    pub async fn close(&self) {
        let socket = self.socket.write().unwrap().close();
        self.wake_all();
        if let Some((sock, _)) = socket {
            let std_sock = sock.into_std();
            let res = tokio::runtime::Handle::current()
                .spawn_blocking(move || {
                    // Calls libc::close, which can block
                    drop(std_sock);
                })
                .await;
            if let Err(err) = res {
                warn!("failed to close socket: {:?}", err);
            }
        }
    }

    /// Check if this socket is closed.
    pub fn is_closed(&self) -> bool {
        self.socket.read().unwrap().is_closed()
    }

    /// Handle potential read errors, updating internal state.
    ///
    /// Returns `Some(error)` if the error is fatal otherwise `None.
    fn handle_read_error(&self, error: io::Error) -> Option<io::Error> {
        match error.kind() {
            io::ErrorKind::NotConnected => {
                // This indicates the underlying socket is broken, and we should attempt to rebind it
                self.mark_broken();
                None
            }
            // A transient receive error leaves the socket healthy with the next datagram still queued,
            // so we drop the error poll again rather than surface a spurious failure.
            _ if is_transient_read_error(&error) => None,
            _ => Some(error),
        }
    }

    /// Handle potential write errors, updating internal state.
    ///
    /// Returns `Some(error)` if the error is fatal otherwise `None.
    fn handle_write_error(&self, error: io::Error) -> Option<io::Error> {
        match error.kind() {
            io::ErrorKind::BrokenPipe => {
                // This indicates the underlying socket is broken, and we should attempt to rebind it
                self.mark_broken();
                None
            }
            _ => Some(error),
        }
    }

    /// Try to get a read lock for the sockets, but don't block for trying to acquire it.
    fn poll_read_socket(
        &self,
        waker: &AtomicWaker,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<RwLockReadGuard<'_, SocketState>> {
        let guard = match self.socket.try_read() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(e)) => panic!("socket lock poisoned: {e}"),
            Err(TryLockError::WouldBlock) => {
                waker.register(cx.waker());

                match self.socket.try_read() {
                    Ok(guard) => {
                        // we're actually fine, no need to cause a spurious wakeup
                        waker.take();
                        guard
                    }
                    Err(TryLockError::Poisoned(e)) => panic!("socket lock poisoned: {e}"),
                    Err(TryLockError::WouldBlock) => {
                        // Ok fine, we registered our waker, the lock is really closed,
                        // we can return pending.
                        return Poll::Pending;
                    }
                }
            }
        };
        Poll::Ready(guard)
    }

    fn wake_all(&self) {
        self.recv_waker.wake();
        self.send_waker.wake();
    }

    /// Checks if the socket needs a rebind, and if so does it.
    ///
    /// Returns an error if the rebind is needed, but failed.
    fn maybe_rebind(&self) -> io::Result<()> {
        if !self.is_broken() {
            return Ok(());
        }

        let mut guard = self.socket.write().unwrap_or_else(|e| e.into_inner());

        // Re-check after acquiring the write lock — another caller may have
        // already completed the rebind while we were waiting.
        if !self.is_broken() {
            return Ok(());
        }

        guard.rebind()?;
        self.is_broken
            .store(false, std::sync::atomic::Ordering::Release);
        drop(guard);
        self.wake_all();
        Ok(())
    }

    /// Poll for writable
    pub fn poll_writable(&self, cx: &mut std::task::Context<'_>) -> Poll<io::Result<()>> {
        loop {
            if let Err(err) = self.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard = std::task::ready!(self.poll_read_socket(&self.send_waker, cx));
            let (socket, _state) = guard.try_get_connected()?;

            match socket.poll_send_ready(cx) {
                Poll::Pending => {
                    self.send_waker.register(cx.waker());
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => return Poll::Ready(Ok(())),
                Poll::Ready(Err(err)) => {
                    if let Some(err) = self.handle_write_error(err) {
                        return Poll::Ready(Err(err));
                    }
                    continue;
                }
            }
        }
    }

    /// Send a noq based `Transmit`.
    pub fn try_send_noq(&self, transmit: &Transmit<'_>) -> io::Result<()> {
        loop {
            self.maybe_rebind()?;

            let guard = match self.socket.try_read() {
                Ok(guard) => guard,
                Err(TryLockError::Poisoned(e)) => {
                    panic!("lock poisoned: {e:?}");
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(io::Error::new(io::ErrorKind::WouldBlock, "locked"));
                }
            };
            let (socket, state) = guard.try_get_connected()?;

            let res = socket.try_io(Interest::WRITABLE, || state.send(socket.into(), transmit));

            match res {
                Ok(()) => return Ok(()),
                Err(err) => match self.handle_write_error(err) {
                    Some(err) => return Err(err),
                    None => {
                        continue;
                    }
                },
            }
        }
    }

    /// poll send a noq based `Transmit`.
    pub fn poll_send_noq(&self, cx: &mut Context, transmit: &Transmit<'_>) -> Poll<io::Result<()>> {
        loop {
            if let Err(err) = self.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard = n0_future::ready!(self.poll_read_socket(&self.send_waker, cx));
            let (socket, state) = guard.try_get_connected()?;

            match socket.poll_send_ready(cx) {
                Poll::Pending => {
                    self.send_waker.register(cx.waker());
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => {
                    let res =
                        socket.try_io(Interest::WRITABLE, || state.send(socket.into(), transmit));
                    if let Err(err) = res {
                        if err.kind() == io::ErrorKind::WouldBlock {
                            continue;
                        }

                        if let Some(err) = self.handle_write_error(err) {
                            return Poll::Ready(Err(err));
                        }
                        continue;
                    }
                    return Poll::Ready(res);
                }
                Poll::Ready(Err(err)) => {
                    if let Some(err) = self.handle_write_error(err) {
                        return Poll::Ready(Err(err));
                    }
                    continue;
                }
            }
        }
    }

    /// noq based `poll_recv`
    pub fn poll_recv_noq(
        &self,
        cx: &mut Context,
        bufs: &mut [io::IoSliceMut<'_>],
        meta: &mut [noq_udp::RecvMeta],
    ) -> Poll<io::Result<usize>> {
        loop {
            if let Err(err) = self.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard = n0_future::ready!(self.poll_read_socket(&self.recv_waker, cx));
            let (socket, state) = guard.try_get_connected()?;

            match socket.poll_recv_ready(cx) {
                Poll::Pending => {
                    self.recv_waker.register(cx.waker());
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => {
                    // We are ready to read, continue
                }
                Poll::Ready(Err(err)) => match self.handle_read_error(err) {
                    Some(err) => return Poll::Ready(Err(err)),
                    None => {
                        continue;
                    }
                },
            }

            let res = socket.try_io(Interest::READABLE, || state.recv(socket.into(), bufs, meta));
            match res {
                Ok(count) => {
                    for meta in meta.iter().take(count) {
                        trace!(
                            src = %meta.addr,
                            len = meta.len,
                            count = meta.len.checked_div(meta.stride).unwrap_or(0),
                            dst = %meta.dst_ip.map(|x| x.to_string()).unwrap_or_default(),
                            "UDP recv"
                        );
                    }
                    return Poll::Ready(Ok(count));
                }
                Err(err) => {
                    // ignore spurious wakeups
                    if err.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    match self.handle_read_error(err) {
                        Some(err) => return Poll::Ready(Err(err)),
                        None => {
                            continue;
                        }
                    }
                }
            }
        }
    }

    /// Creates a [`UdpSender`] sender.
    pub fn create_sender(self: Arc<Self>) -> UdpSender {
        UdpSender::new(self)
    }

    /// Whether transmitted datagrams might get fragmented by the IP layer
    ///
    /// Returns `false` on targets which employ e.g. the `IPV6_DONTFRAG` socket option.
    pub fn may_fragment(&self) -> bool {
        let guard = self.socket.read().unwrap();
        guard.may_fragment()
    }

    /// The maximum amount of segments which can be transmitted if a platform
    /// supports Generic Send Offload (GSO).
    ///
    /// This is 1 if the platform doesn't support GSO. Subject to change if errors are detected
    /// while using GSO.
    pub fn max_gso_segments(&self) -> NonZeroUsize {
        let guard = self.socket.read().unwrap();
        guard.max_gso_segments()
    }

    /// The number of segments to read when GRO is enabled. Used as a factor to
    /// compute the receive buffer size.
    ///
    /// Returns 1 if the platform doesn't support GRO.
    pub fn gro_segments(&self) -> NonZeroUsize {
        let guard = self.socket.read().unwrap();
        guard.gro_segments()
    }
}

/// `WSAENETRESET` (Winsock error 10052).
///
/// On a UDP socket Windows returns this from a recv when a previously sent datagram
/// could not be delivered because its TTL expired in transit, which the network
/// reports back as an ICMP Time Exceeded message. It describes the fate of that one
/// datagram, not the state of the socket: the socket stays usable and the error is
/// cleared once read, so the next recv proceeds normally. The Rust standard library
/// does not map this code to an [`io::ErrorKind`] (unlike `WSAECONNRESET`), so we match
/// it by its raw OS value.
#[cfg(windows)]
const WSAENETRESET: i32 = 10052;

/// Whether a read error is a transient condition that should be retried rather than
/// surfaced to the caller as a failure.
///
/// On Windows the stack reports the fate of a *previously sent* datagram against the
/// next recv on the same socket, so an ICMP reply surfaces as a recv error even though
/// the socket is healthy. `WSAECONNRESET` reports an ICMP Port Unreachable, meaning the
/// destination had no listener. `WSAENETRESET` reports an ICMP Time Exceeded, meaning a
/// datagram's TTL expired in transit. Both are transient for the same reason: each
/// describes a single datagram and is delivered exactly once, so reading it clears the
/// condition and the following recv returns real data.
///
/// We treat `ConnectionReset` as transient on every platform, not only Windows:
/// ECONNRESET is undefined in QUIC and can be injected by an attacker, so
/// it must never tear down the receive path.
fn is_transient_read_error(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::ConnectionReset {
        return true;
    }
    #[cfg(windows)]
    if error.raw_os_error() == Some(WSAENETRESET) {
        return true;
    }
    false
}

/// Receive future
#[derive(Debug)]
pub struct RecvFut<'a, 'b> {
    socket: &'b UdpSocket,
    buffer: &'a mut [u8],
}

impl Future for RecvFut<'_, '_> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let Self { socket, buffer } = &mut *self;

        loop {
            if let Err(err) = socket.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard = n0_future::ready!(socket.poll_read_socket(&socket.recv_waker, cx));
            let (inner_socket, _state) = guard.try_get_connected()?;

            match inner_socket.poll_recv_ready(cx) {
                Poll::Pending => {
                    self.socket.recv_waker.register(cx.waker());
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => {
                    let res = inner_socket.try_recv(buffer);
                    if let Err(err) = res {
                        if err.kind() == io::ErrorKind::WouldBlock {
                            continue;
                        }
                        if let Some(err) = socket.handle_read_error(err) {
                            return Poll::Ready(Err(err));
                        }
                        continue;
                    }
                    return Poll::Ready(res);
                }
                Poll::Ready(Err(err)) => {
                    if let Some(err) = socket.handle_read_error(err) {
                        return Poll::Ready(Err(err));
                    }
                    continue;
                }
            }
        }
    }
}

/// Receive future
#[derive(Debug)]
pub struct RecvFromFut<'a, 'b> {
    socket: &'b UdpSocket,
    buffer: &'a mut [u8],
}

impl Future for RecvFromFut<'_, '_> {
    type Output = io::Result<(usize, SocketAddr)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let Self { socket, buffer } = &mut *self;

        loop {
            if let Err(err) = socket.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard = n0_future::ready!(socket.poll_read_socket(&socket.recv_waker, cx));
            let (inner_socket, _state) = guard.try_get_connected()?;

            match inner_socket.poll_recv_ready(cx) {
                Poll::Pending => {
                    self.socket.recv_waker.register(cx.waker());
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => {
                    let res = inner_socket.try_recv_from(buffer);
                    if let Err(err) = res {
                        if err.kind() == io::ErrorKind::WouldBlock {
                            continue;
                        }
                        if let Some(err) = socket.handle_read_error(err) {
                            return Poll::Ready(Err(err));
                        }
                        continue;
                    }
                    return Poll::Ready(res);
                }
                Poll::Ready(Err(err)) => {
                    if let Some(err) = socket.handle_read_error(err) {
                        return Poll::Ready(Err(err));
                    }
                    continue;
                }
            }
        }
    }
}

/// Send future
#[derive(Debug)]
pub struct SendFut<'a, 'b> {
    socket: &'b UdpSocket,
    buffer: &'a [u8],
}

impl Future for SendFut<'_, '_> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            if let Err(err) = self.socket.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard =
                n0_future::ready!(self.socket.poll_read_socket(&self.socket.send_waker, cx));
            let (socket, _state) = guard.try_get_connected()?;

            match socket.poll_send_ready(cx) {
                Poll::Pending => {
                    self.socket.send_waker.register(cx.waker());
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => {
                    let res = socket.try_send(self.buffer);
                    if let Err(err) = res {
                        if err.kind() == io::ErrorKind::WouldBlock {
                            continue;
                        }
                        if let Some(err) = self.socket.handle_write_error(err) {
                            return Poll::Ready(Err(err));
                        }
                        continue;
                    }
                    return Poll::Ready(res);
                }
                Poll::Ready(Err(err)) => {
                    if let Some(err) = self.socket.handle_write_error(err) {
                        return Poll::Ready(Err(err));
                    }
                    continue;
                }
            }
        }
    }
}

/// Send future
#[derive(Debug)]
pub struct SendToFut<'a, 'b> {
    socket: &'b UdpSocket,
    buffer: &'a [u8],
    to: SocketAddr,
}

impl Future for SendToFut<'_, '_> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            if let Err(err) = self.socket.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard =
                n0_future::ready!(self.socket.poll_read_socket(&self.socket.send_waker, cx));
            let (socket, _state) = guard.try_get_connected()?;

            match socket.poll_send_ready(cx) {
                Poll::Pending => {
                    self.socket.send_waker.register(cx.waker());
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => {
                    let res = socket.try_send_to(self.buffer, self.to);
                    if let Err(err) = res {
                        if err.kind() == io::ErrorKind::WouldBlock {
                            continue;
                        }

                        if let Some(err) = self.socket.handle_write_error(err) {
                            return Poll::Ready(Err(err));
                        }
                        continue;
                    }
                    return Poll::Ready(res);
                }
                Poll::Ready(Err(err)) => {
                    if let Some(err) = self.socket.handle_write_error(err) {
                        return Poll::Ready(Err(err));
                    }
                    continue;
                }
            }
        }
    }
}

#[derive(Debug)]
enum SocketState {
    Connected {
        socket: tokio::net::UdpSocket,
        state: noq_udp::UdpSocketState,
        /// The addr we are binding to.
        addr: SocketAddr,
    },
    Closed {
        /// The addr to rebind to when recovering.
        addr: SocketAddr,
        last_max_gso_segments: NonZeroUsize,
        last_gro_segments: NonZeroUsize,
        last_may_fragment: bool,
    },
}

impl SocketState {
    fn try_get_connected(&self) -> io::Result<(&tokio::net::UdpSocket, &noq_udp::UdpSocketState)> {
        match self {
            Self::Connected {
                socket,
                state,
                addr: _,
            } => Ok((socket, state)),
            Self::Closed { .. } => {
                warn!("socket closed");
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "socket closed"))
            }
        }
    }

    fn bind(addr: SocketAddr) -> io::Result<Self> {
        let network = IpFamily::from(addr.ip());
        let socket = socket2::Socket::new(
            network.into(),
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;

        if let Err(err) = socket.set_recv_buffer_size(SOCKET_BUFFER_SIZE) {
            debug!(
                "failed to set recv_buffer_size to {}: {:?}",
                SOCKET_BUFFER_SIZE, err
            );
        }
        if let Err(err) = socket.set_send_buffer_size(SOCKET_BUFFER_SIZE) {
            debug!(
                "failed to set send_buffer_size to {}: {:?}",
                SOCKET_BUFFER_SIZE, err
            );
        }
        if network == IpFamily::V6 {
            // Avoid dualstack
            socket.set_only_v6(true)?;
        }

        // Binding must happen before calling noq, otherwise `local_addr`
        // is not yet available on all OSes.
        socket.bind(&addr.into())?;

        // Ensure nonblocking
        socket.set_nonblocking(true)?;

        let socket: std::net::UdpSocket = socket.into();

        // Convert into tokio UdpSocket
        let socket = tokio::net::UdpSocket::from_std(socket)?;
        let socket_ref = noq_udp::UdpSockRef::from(&socket);
        let socket_state = noq_udp::UdpSocketState::new(socket_ref)?;

        let local_addr = socket.local_addr()?;
        if addr.port() != 0 && local_addr.port() != addr.port() {
            return Err(io::Error::other(format!(
                "wrong port bound: {:?}: wanted: {} got {}",
                network,
                addr.port(),
                local_addr.port(),
            )));
        }

        Ok(Self::Connected {
            socket,
            state: socket_state,
            addr: local_addr,
        })
    }

    fn rebind(&mut self) -> io::Result<()> {
        let addr = match self {
            Self::Connected { addr, .. } => *addr,
            Self::Closed { addr, .. } => *addr,
        };
        debug!("rebinding {}", addr);

        // Transition to Closed first to drop the old socket.
        // This is needed so the port is released before we try to bind again.
        if let Self::Connected { state, .. } = self {
            *self = SocketState::Closed {
                addr,
                last_max_gso_segments: state.max_gso_segments(),
                last_gro_segments: state.gro_segments(),
                last_may_fragment: state.may_fragment(),
            };
        }

        match Self::bind(addr) {
            Ok(new_state) => {
                *self = new_state;
                Ok(())
            }
            Err(err) => {
                // Stay in Closed state but allow future rebind attempts
                debug!("rebind failed, will retry on next attempt: {}", err);
                Err(err)
            }
        }
    }

    fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    fn close(&mut self) -> Option<(tokio::net::UdpSocket, noq_udp::UdpSocketState)> {
        match self {
            Self::Connected { state, addr, .. } => {
                let s = SocketState::Closed {
                    addr: *addr,
                    last_max_gso_segments: state.max_gso_segments(),
                    last_gro_segments: state.gro_segments(),
                    last_may_fragment: state.may_fragment(),
                };
                let Self::Connected { socket, state, .. } = std::mem::replace(self, s) else {
                    unreachable!("just checked");
                };
                Some((socket, state))
            }
            Self::Closed { .. } => None,
        }
    }

    fn may_fragment(&self) -> bool {
        match self {
            Self::Connected { state, .. } => state.may_fragment(),
            Self::Closed {
                last_may_fragment, ..
            } => *last_may_fragment,
        }
    }

    fn max_gso_segments(&self) -> NonZeroUsize {
        match self {
            Self::Connected { state, .. } => state.max_gso_segments(),
            Self::Closed {
                last_max_gso_segments,
                ..
            } => *last_max_gso_segments,
        }
    }

    fn gro_segments(&self) -> NonZeroUsize {
        match self {
            Self::Connected { state, .. } => state.gro_segments(),
            Self::Closed {
                last_gro_segments, ..
            } => *last_gro_segments,
        }
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        if let Some((socket, _)) = self.socket.write().unwrap().close()
            && let Ok(handle) = tokio::runtime::Handle::try_current()
        {
            // No wakeup after dropping write lock here, since we're getting dropped.
            // this will be empty if `close` was called before
            let std_sock = socket.into_std();
            handle.spawn_blocking(move || {
                // Calls libc::close, which can block
                drop(std_sock);
            });
        }
    }
}

pin_project_lite::pin_project! {
    pub struct UdpSender {
        socket: Arc<UdpSocket>,
        #[pin]
        fut: Option<Pin<Box<dyn Future<Output = io::Result<()>> + Send + Sync + 'static>>>,
    }
}

impl Clone for UdpSender {
    fn clone(&self) -> Self {
        self.socket.clone().create_sender()
    }
}

impl std::fmt::Debug for UdpSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UdpSender")
    }
}

impl UdpSender {
    fn new(socket: Arc<UdpSocket>) -> Self {
        Self { socket, fut: None }
    }

    /// Async sending
    pub fn send<'a, 'b>(&self, transmit: &'a noq_udp::Transmit<'b>) -> SendFutNoq<'a, 'b> {
        SendFutNoq {
            socket: self.socket.clone(),
            transmit,
        }
    }

    /// Poll send
    pub fn poll_send(
        self: Pin<&mut Self>,
        transmit: &noq_udp::Transmit,
        cx: &mut Context,
    ) -> Poll<io::Result<()>> {
        let mut this = self.project();
        loop {
            if let Err(err) = this.socket.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard =
                n0_future::ready!(this.socket.poll_read_socket(&this.socket.send_waker, cx));

            if this.fut.is_none() {
                let socket = this.socket.clone();
                this.fut.set(Some(Box::pin(async move {
                    n0_future::future::poll_fn(|cx| socket.poll_writable(cx)).await
                })));
            }
            // We're forced to `unwrap` here because `Fut` may be `!Unpin`, which means we can't safely
            // obtain an `&mut Fut` after storing it in `this.fut` when `this` is already behind `Pin`,
            // and if we didn't store it then we wouldn't be able to keep it alive between
            // `poll_writable` calls.
            let result = n0_future::ready!(this.fut.as_mut().as_pin_mut().unwrap().poll(cx));

            // Polling an arbitrary `Future` after it becomes ready is a logic error, so arrange for
            // a new `Future` to be created on the next call.
            this.fut.set(None);

            // If .writable() fails, propagate the error
            result?;

            let (socket, state) = guard.try_get_connected()?;
            let result = socket.try_io(Interest::WRITABLE, || state.send(socket.into(), transmit));

            match result {
                // We thought the socket was writable, but it wasn't, then retry so that either another
                // `writable().await` call determines that the socket is indeed not writable and
                // registers us for a wakeup, or the send succeeds if this really was just a
                // transient failure.
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                // In all other cases, either propagate the error or we're Ok
                _ => return Poll::Ready(result),
            }
        }
    }

    /// Best effort sending
    pub fn try_send(&self, transmit: &noq_udp::Transmit) -> io::Result<()> {
        self.socket.maybe_rebind()?;

        match self.socket.socket.try_read() {
            Ok(guard) => {
                let (socket, state) = guard.try_get_connected()?;
                socket.try_io(Interest::WRITABLE, || state.send(socket.into(), transmit))
            }
            Err(TryLockError::Poisoned(e)) => panic!("socket lock poisoned: {e}"),
            Err(TryLockError::WouldBlock) => {
                Err(io::Error::new(io::ErrorKind::WouldBlock, "locked"))
            }
        }
    }
}

/// Send future noq
#[derive(Debug)]
pub struct SendFutNoq<'a, 'b> {
    socket: Arc<UdpSocket>,
    transmit: &'a noq_udp::Transmit<'b>,
}

impl Future for SendFutNoq<'_, '_> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            if let Err(err) = self.socket.maybe_rebind() {
                return Poll::Ready(Err(err));
            }

            let guard =
                n0_future::ready!(self.socket.poll_read_socket(&self.socket.send_waker, cx));
            let (socket, state) = guard.try_get_connected()?;

            match socket.poll_send_ready(cx) {
                Poll::Pending => {
                    self.socket.send_waker.register(cx.waker());
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => {
                    let res = socket.try_io(Interest::WRITABLE, || {
                        state.send(socket.into(), self.transmit)
                    });

                    if let Err(err) = res {
                        if err.kind() == io::ErrorKind::WouldBlock {
                            continue;
                        }
                        if let Some(err) = self.socket.handle_write_error(err) {
                            return Poll::Ready(Err(err));
                        }
                        continue;
                    }
                    return Poll::Ready(res);
                }
                Poll::Ready(Err(err)) => {
                    if let Some(err) = self.socket.handle_write_error(err) {
                        return Poll::Ready(Err(err));
                    }
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn test_reconnect() -> TestResult {
        let (s_b, mut r_b) = tokio::sync::mpsc::channel(16);
        let handle_a = tokio::task::spawn(async move {
            let socket = UdpSocket::bind_local(IpFamily::V4, 0)?;
            let addr = socket.local_addr()?;
            s_b.send(addr).await?;
            println!("socket bound to {addr:?}");

            let mut buffer = [0u8; 16];
            for i in 0..100 {
                println!("-- tick {i}");
                let read = socket.recv_from(&mut buffer).await;
                match read {
                    Ok((count, addr)) => {
                        println!("got {:?}", &buffer[..count]);
                        println!("sending {:?} to {:?}", &buffer[..count], addr);
                        socket.send_to(&buffer[..count], addr).await?;
                    }
                    Err(err) => {
                        eprintln!("error reading: {err:?}");
                    }
                }
            }
            socket.close().await;
            Ok::<_, testresult::TestError>(())
        });

        let socket = UdpSocket::bind_local(IpFamily::V4, 0)?;
        let first_addr = socket.local_addr()?;
        println!("socket2 bound to {:?}", socket.local_addr()?);
        let addr = r_b.recv().await.unwrap();

        let mut buffer = [0u8; 16];
        for i in 0u8..100 {
            println!("round one - {i}");
            socket.send_to(&[i][..], addr).await?;
            let (count, from) = socket.recv_from(&mut buffer).await?;
            assert_eq!(addr, from);
            assert_eq!(count, 1);
            assert_eq!(buffer[0], i);

            // check for errors
            assert!(!socket.is_broken());

            // rebind
            socket.rebind()?;

            // check that the socket has the same address as before
            assert_eq!(socket.local_addr()?, first_addr);
        }

        handle_a.await.ok();

        Ok(())
    }

    #[tokio::test]
    async fn test_udp_mark_broken() -> TestResult {
        let socket_a = UdpSocket::bind_local(IpFamily::V4, 0)?;
        let addr_a = socket_a.local_addr()?;
        println!("socket bound to {addr_a:?}");

        let socket_b = UdpSocket::bind_local(IpFamily::V4, 0)?;
        let addr_b = socket_b.local_addr()?;
        println!("socket bound to {addr_b:?}");

        let handle = tokio::task::spawn(async move {
            let mut buffer = [0u8; 16];
            for _ in 0..2 {
                match socket_b.recv_from(&mut buffer).await {
                    Ok((count, addr)) => {
                        println!("got {:?} from {:?}", &buffer[..count], addr);
                    }
                    Err(err) => {
                        eprintln!("error recv: {err:?}");
                    }
                }
            }
        });
        socket_a.send_to(&[0][..], addr_b).await?;
        socket_a.mark_broken();
        assert!(socket_a.is_broken());
        socket_a.send_to(&[0][..], addr_b).await?;
        assert!(!socket_a.is_broken());

        handle.await?;
        Ok(())
    }

    /// Regression test for the Windows behavior handled by [`is_transient_read_error`].
    ///
    /// A recv call must survive an ICMP error caused by an earlier send on the same socket.
    ///
    /// On Windows, sending a UDP datagram to a port with no listener draws an ICMP
    /// port-unreachable, and the OS reports it against the *next* recv on that socket as
    /// WSAECONNRESET. Before the fix our recv loop surfaced that error, so a perfectly
    /// good datagram waiting behind it was lost and the recv failed. After the fix the
    /// error is ignored and the real datagram is delivered.
    ///
    /// This only exercises the bug on Windows. Other platforms do not deliver the ICMP
    /// error to an unconnected recv, so the recv just returns the datagram and the test
    /// passes whether or not the fix is present.
    #[tokio::test]
    async fn test_recv_survives_icmp_unreachable_from_prior_send() -> TestResult {
        use std::time::Duration;

        // The socket under test, plus a legitimate peer to deliver a real datagram.
        let receiver = UdpSocket::bind_local(IpFamily::V4, 0)?;
        let receiver_addr = receiver.local_addr()?;
        let sender = UdpSocket::bind_local(IpFamily::V4, 0)?;
        let sender_addr = sender.local_addr()?;

        // A definitely-closed port: bind a socket, take its address, then close it.
        let closed_addr = {
            let tmp = UdpSocket::bind_local(IpFamily::V4, 0)?;
            let addr = tmp.local_addr()?;
            tmp.close().await;
            addr
        };

        // The receiver pokes the closed port. On Windows the ICMP port-unreachable that
        // comes back arms WSAECONNRESET against the receiver's next recv.
        receiver.send_to(b"void", closed_addr).await?;

        // Give the ICMP reply time to arrive, so the error is pending before the real
        // datagram and the recv.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // The peer sends a real datagram that the receiver must deliver.
        sender.send_to(b"hello", receiver_addr).await?;

        // Before the fix this recv returns WSAECONNRESET on Windows instead of "hello".
        let mut buf = [0u8; 16];
        let (n, from) = tokio::time::timeout(Duration::from_secs(5), receiver.recv_from(&mut buf))
            .await
            .expect("recv must not hang")?;

        assert_eq!(&buf[..n], b"hello");
        assert_eq!(from, sender_addr);

        Ok(())
    }
}
