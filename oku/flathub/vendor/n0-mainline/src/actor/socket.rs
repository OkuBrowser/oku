//! UDP socket layer managing incoming/outgoing requests and responses.
//!
//! Uses [`noq_udp`] for all socket I/O (`sendmsg`/`recvmsg`) with optimal
//! platform-specific configuration (ECN, buffer sizes, MTU discovery).
//! Async readiness is driven by [`tokio::net::UdpSocket`] via `try_io`.

use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::{self, ErrorKind, IoSliceMut};
use std::net::{SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};
use tokio::io::Interest;
use tokio::net::UdpSocket;
use tracing::{debug, trace, warn};

use crate::common::{ErrorSpecific, Message, MessageType, RequestSpecific, ResponseSpecific};
#[cfg(not(test))]
use crate::core::VERSION;

use super::config::Config;

const MTU: usize = 2048;

const DEFAULT_PORT: u16 = 6881;

const MIN_REQUEST_TIMEOUT: Duration = Duration::from_millis(500);

/// Version before supporting `announce_signed_peers`
#[cfg(test)]
const LEGACY_VERSION: [u8; 4] = [82, 83, 0, 5]; // "RS" version 05

// ---------------------------------------------------------------------------
// UdpIo – thin async wrapper around noq-udp + tokio readiness
// ---------------------------------------------------------------------------

/// Async UDP I/O backed by [`noq_udp`].
///
/// [`tokio::net::UdpSocket`] provides readiness notifications (epoll/kqueue);
/// [`noq_udp::UdpSocketState`] performs the actual `sendmsg`/`recvmsg` syscalls
/// with platform-optimal settings.
#[derive(Debug)]
struct UdpIo {
    socket: UdpSocket,
    state: noq_udp::UdpSocketState,
}

impl UdpIo {
    /// Bind to `addr`, configure via noq-udp, register with tokio reactor.
    fn bind(addr: SocketAddr) -> io::Result<Self> {
        let std_socket = std::net::UdpSocket::bind(addr)?;
        let state = noq_udp::UdpSocketState::new((&std_socket).into())?;
        let socket = UdpSocket::from_std(std_socket)?;
        Ok(Self { socket, state })
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Send `buf` to `dest`. Waits for writability, then calls
    /// noq-udp `sendmsg`.
    async fn send_to(&self, buf: &[u8], dest: SocketAddr) -> io::Result<()> {
        let transmit = noq_udp::Transmit {
            destination: dest,
            ecn: None,
            contents: buf,
            segment_size: None,
            src_ip: None,
        };
        loop {
            self.socket.writable().await?;
            match self.socket.try_io(Interest::WRITABLE, || {
                self.state.send((&self.socket).into(), &transmit)
            }) {
                Ok(()) => return Ok(()),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e),
            }
        }
    }

    /// Receive one datagram. Waits for readability, then calls
    /// noq-udp `recvmsg`.
    async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        loop {
            self.socket.readable().await?;
            match self.try_recv(buf) {
                Ok(result) => return Ok(result),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e),
            }
        }
    }

    /// Non-blocking receive attempt via `try_io` + noq-udp `recvmsg`.
    fn try_recv(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.socket.try_io(Interest::READABLE, || {
            let mut iov = [IoSliceMut::new(buf)];
            let mut meta = [noq_udp::RecvMeta::default()];
            let n = self
                .state
                .recv((&self.socket).into(), &mut iov, &mut meta)?;
            if n > 0 {
                Ok((meta[0].len, meta[0].addr))
            } else {
                Err(io::Error::new(ErrorKind::WouldBlock, "no data"))
            }
        })
    }
}

// ---------------------------------------------------------------------------
// KrpcSocket – KRPC message framing on top of UdpIo
// ---------------------------------------------------------------------------

/// A UdpSocket wrapper that formats and correlates DHT requests and responses.
///
/// Sends are queued synchronously and flushed asynchronously via [`Self::flush`].
/// Receives are fully async via [`Self::recv_from`].
#[derive(Debug)]
pub struct KrpcSocket {
    io: UdpIo,
    pub(crate) server_mode: bool,
    local_addr: SocketAddrV4,

    inflight_requests: InflightRequests,
    /// Outgoing packets queued by sync send methods.
    outbox: VecDeque<(Vec<u8>, SocketAddr)>,

    #[cfg(test)]
    version: [u8; 4],
}

impl KrpcSocket {
    pub(crate) fn new(config: &Config) -> io::Result<Self> {
        let port = config.port;

        let io = if let Some(port) = port {
            UdpIo::bind(SocketAddr::from(([0, 0, 0, 0], port)))?
        } else {
            match UdpIo::bind(SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT))) {
                Ok(io) => Ok(io),
                Err(_) => UdpIo::bind(SocketAddr::from(([0, 0, 0, 0], 0))),
            }?
        };

        let local_addr = match io.local_addr()? {
            SocketAddr::V4(addr) => addr,
            SocketAddr::V6(_) => unimplemented!("KrpcSocket does not support Ipv6"),
        };

        Ok(Self {
            io,
            server_mode: config.server_mode,
            inflight_requests: InflightRequests::new(),
            local_addr,
            outbox: VecDeque::new(),

            #[cfg(test)]
            version: if config.disable_announce_signed_peers {
                LEGACY_VERSION
            } else {
                crate::core::VERSION
            },
        })
    }

    #[cfg(test)]
    pub(crate) fn server() -> io::Result<Self> {
        Self::new(&Config {
            server_mode: true,
            ..Default::default()
        })
    }

    #[cfg(test)]
    pub(crate) fn client() -> io::Result<Self> {
        Self::new(&Config::default())
    }

    // === Getters ===

    /// Returns the address the server is listening to.
    #[inline]
    pub fn local_addr(&self) -> SocketAddrV4 {
        self.local_addr
    }

    // === Public Methods ===

    /// Returns true if this message's transaction_id is still inflight
    pub fn inflight(&self, transaction_id: &u32) -> bool {
        self.inflight_requests.get(*transaction_id).is_some()
    }

    /// Send a request to the given address and return the transaction_id.
    /// The packet is queued; call [`Self::flush`] to actually send.
    pub fn request(&mut self, address: SocketAddrV4, request: RequestSpecific) -> u32 {
        let transaction_id = self.inflight_requests.add(address);

        let message = self.request_message(transaction_id, request);
        trace!(context = "socket_message_sending", message = ?message);

        let tid = message.transaction_id;
        let _ = self.enqueue(address, message).map_err(|e| {
            debug!(?e, "Error encoding request message");
        });

        tid
    }

    /// Send a response to the given address.
    /// The packet is queued; call [`Self::flush`] to actually send.
    pub fn response(
        &mut self,
        address: SocketAddrV4,
        transaction_id: u32,
        response: ResponseSpecific,
    ) {
        let message =
            self.response_message(MessageType::Response(response), address, transaction_id);
        trace!(context = "socket_message_sending", message = ?message);
        let _ = self.enqueue(address, message).map_err(|e| {
            debug!(?e, "Error encoding response message");
        });
    }

    /// Send an error to the given address.
    /// The packet is queued; call [`Self::flush`] to actually send.
    pub fn error(&mut self, address: SocketAddrV4, transaction_id: u32, error: ErrorSpecific) {
        let message = self.response_message(MessageType::Error(error), address, transaction_id);
        let _ = self.enqueue(address, message).map_err(|e| {
            debug!(?e, "Error encoding error message");
        });
    }

    /// Flush all queued outgoing packets.
    pub async fn flush(&mut self) {
        while let Some((bytes, addr)) = self.outbox.pop_front() {
            if let Err(e) = self.io.send_to(&bytes, addr).await {
                debug!(?e, "Error sending UDP packet");
            }
        }
    }

    /// Async receive: waits for a datagram and returns a parsed KRPC message.
    pub async fn recv_from(&mut self) -> Option<(Message, SocketAddrV4)> {
        let mut buf = [0u8; MTU];

        self.inflight_requests.cleanup();

        match self.io.recv_from(&mut buf).await {
            Ok((amt, SocketAddr::V4(from))) => {
                let bytes = &buf[..amt];

                if from.port() == 0 {
                    trace!(
                        context = "socket_validation",
                        message = "Response from port 0"
                    );
                    return None;
                }

                match Message::from_bytes(bytes) {
                    Ok(message) => {
                        // Parsed correctly.
                        let should_return = match message.message_type {
                            MessageType::Request(ref _request_specific) => {
                                // simulate legacy nodes not supporting `announce_signed_peers` and `get_signed_peers`
                                #[cfg(test)]
                                let should_return =
                                    supports_request(&self.version, _request_specific);
                                #[cfg(not(test))]
                                let should_return = true;

                                trace!(
                                    context = "socket_message_receiving",
                                    ?message,
                                    ?from,
                                    "Received request message"
                                );

                                should_return
                            }
                            MessageType::Response(_) => {
                                trace!(
                                    context = "socket_message_receiving",
                                    ?message,
                                    ?from,
                                    "Received response message"
                                );

                                self.is_expected_response(&message, &from)
                            }
                            MessageType::Error(_) => {
                                trace!(
                                    context = "socket_message_receiving",
                                    ?message,
                                    ?from,
                                    "Received error message"
                                );

                                self.is_expected_response(&message, &from)
                            }
                        };

                        if should_return {
                            return Some((message, from));
                        }
                    }
                    Err(error) => {
                        trace!(context = "socket_error", ?error, ?from, message = ?String::from_utf8_lossy(bytes), "Received invalid Bencode message.");
                    }
                };
            }
            Ok((_, SocketAddr::V6(_))) => {
                // Ignore unsupported Ipv6 messages
            }
            Err(error) => {
                warn!("IO error {error}")
            }
        };

        None
    }

    // === Private Methods ===

    fn is_expected_response(&mut self, message: &Message, from: &SocketAddrV4) -> bool {
        // Positive or an error response or to an inflight request.
        match self.inflight_requests.remove(message.transaction_id) {
            Some(request) => {
                if compare_socket_addr(&request.to, from) {
                    return true;
                } else {
                    trace!(
                        context = "socket_validation",
                        message = "Response from wrong address"
                    );
                }
            }
            None => {
                trace!(
                    context = "socket_validation",
                    message = "Unexpected response id, or timedout request"
                );
            }
        }

        false
    }

    /// Set transactin_id, version and read_only
    fn request_message(&mut self, transaction_id: u32, message: RequestSpecific) -> Message {
        Message {
            transaction_id,
            message_type: MessageType::Request(message),
            version: Some(self.version()),
            read_only: !self.server_mode,
            requester_ip: None,
        }
    }

    /// Same as request_message but with request transaction_id and the requester_ip.
    fn response_message(
        &mut self,
        message: MessageType,
        requester_ip: SocketAddrV4,
        request_tid: u32,
    ) -> Message {
        Message {
            transaction_id: request_tid,
            message_type: message,
            version: Some(self.version()),
            read_only: !self.server_mode,
            // BEP_0042 Only relevant in responses.
            requester_ip: Some(requester_ip),
        }
    }

    fn version(&self) -> [u8; 4] {
        #[cfg(test)]
        return self.version;
        #[cfg(not(test))]
        return VERSION;
    }

    /// Encode a message and queue it for sending.
    fn enqueue(&mut self, address: SocketAddrV4, message: Message) -> Result<(), SendMessageError> {
        let bytes = message.to_bytes()?;
        trace!(context = "socket_message_sending", message = ?message);
        self.outbox.push_back((bytes, SocketAddr::from(address)));
        Ok(())
    }
}

#[n0_error::stack_error(derive, from_sources, std_sources)]
/// Mainline crate error enum.
enum SendMessageError {
    /// Errors related to parsing DHT messages.
    #[error(transparent)]
    BencodeError(serde_bencode::Error),

    #[error(transparent)]
    /// IO error
    IO(std::io::Error),
}

// Same as SocketAddr::eq but ignores the ip if it is unspecified for testing reasons.
fn compare_socket_addr(a: &SocketAddrV4, b: &SocketAddrV4) -> bool {
    if a.port() != b.port() {
        return false;
    }

    if a.ip().is_unspecified() {
        return true;
    }

    a.ip() == b.ip()
}

struct InflightRequest {
    tid: u32,
    to: SocketAddrV4,
    sent_at: Instant,
}

impl Debug for InflightRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InflightRequest")
            .field("tid", &self.tid)
            .field("to", &self.to)
            .field("elapsed", &self.sent_at.elapsed())
            .finish()
    }
}

/// We don't need a map, since we know the maximum size is `65536` requests.
/// Requests are also ordered by their transaction_id and thus sent_at, so lookup is fast.
struct InflightRequests {
    next_tid: u32,
    requests: Vec<InflightRequest>,
    estimated_rtt: Duration,
    deviation_rtt: Duration,
}

impl InflightRequests {
    fn new() -> Self {
        Self {
            next_tid: 0,
            requests: Vec::new(),
            estimated_rtt: MIN_REQUEST_TIMEOUT,
            deviation_rtt: Duration::from_secs(0),
        }
    }

    /// Increments self.next_tid and returns the previous value.
    fn tid(&mut self) -> u32 {
        // We don't reuse freed transaction ids, to preserve the sortablitiy
        // of both `tid`s and `sent_at`.
        let tid = self.next_tid;
        self.next_tid = self.next_tid.wrapping_add(1);
        tid
    }

    fn request_timeout(&self) -> Duration {
        self.estimated_rtt + self.deviation_rtt.mul_f64(4.0)
    }

    fn get(&self, key: u32) -> Option<&InflightRequest> {
        if let Ok(index) = self.find_by_tid(key)
            && let Some(request) = self.requests.get(index)
            && request.sent_at.elapsed() < self.request_timeout()
        {
            return Some(request);
        };

        None
    }

    /// Adds a [InflightRequest] with new transaction_id, and returns that id.
    fn add(&mut self, to: SocketAddrV4) -> u32 {
        let tid = self.tid();
        self.requests.push(InflightRequest {
            tid,
            to,
            sent_at: Instant::now(),
        });

        tid
    }

    fn remove(&mut self, key: u32) -> Option<InflightRequest> {
        match self.find_by_tid(key) {
            Ok(index) => {
                let request = self.requests.remove(index);

                self.update_rtt_estimates(request.sent_at.elapsed());
                trace!(
                    "Updated estimated round trip time {:?} and request timeout {:?}",
                    self.estimated_rtt,
                    self.request_timeout()
                );

                Some(request)
            }
            Err(_) => None,
        }
    }

    fn update_rtt_estimates(&mut self, sample_rtt: Duration) {
        if sample_rtt < MIN_REQUEST_TIMEOUT {
            return;
        }

        // Use TCP-like alpha = 1/8, beta = 1/4
        let alpha = 0.125;
        let beta = 0.25;

        let sample_rtt_secs = sample_rtt.as_secs_f64();
        let est_rtt_secs = self.estimated_rtt.as_secs_f64();
        let dev_rtt_secs = self.deviation_rtt.as_secs_f64();

        let new_est_rtt = (1.0 - alpha) * est_rtt_secs + alpha * sample_rtt_secs;
        let new_dev_rtt =
            (1.0 - beta) * dev_rtt_secs + beta * (sample_rtt_secs - new_est_rtt).abs();

        self.estimated_rtt = Duration::from_secs_f64(new_est_rtt);
        self.deviation_rtt = Duration::from_secs_f64(new_dev_rtt);
    }

    fn find_by_tid(&self, tid: u32) -> Result<usize, usize> {
        self.requests
            .binary_search_by(|request| request.tid.cmp(&tid))
    }

    /// Removes timeedout requests if necessary to save memory
    fn cleanup(&mut self) {
        if self.requests.len() < self.requests.capacity() {
            return;
        }

        let index = match self
            .requests
            .binary_search_by(|request| self.request_timeout().cmp(&request.sent_at.elapsed()))
        {
            Ok(index) => index,
            Err(index) => index,
        };

        self.requests.drain(0..index);
    }
}

impl Debug for InflightRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let timeout = self.request_timeout();
        let non_expired: Vec<_> = self
            .requests
            .iter()
            .filter(|req| req.sent_at.elapsed() < timeout)
            .collect();

        f.debug_struct("InflightRequests")
            .field("next_tid", &self.next_tid)
            .field("requests", &non_expired)
            .field("estimated_rtt", &self.estimated_rtt)
            .field("deviation_rtt", &self.deviation_rtt)
            .field("request_timeout", &timeout)
            .finish()
    }
}

#[cfg(test)]
fn supports_request(version: &[u8; 4], request_specific: &crate::common::RequestSpecific) -> bool {
    use crate::{
        PutRequestSpecific,
        common::{PutRequest, RequestTypeSpecific},
    };
    match request_specific.request_type {
        RequestTypeSpecific::GetSignedPeers(_)
        | RequestTypeSpecific::Put(PutRequest {
            put_request_type: PutRequestSpecific::AnnounceSignedPeer(_),
            ..
        }) => version != &LEGACY_VERSION,
        _ => true,
    }
}

#[cfg(test)]
mod test {
    use crate::{
        common::{Id, PingResponseArguments, RequestTypeSpecific},
        core::VERSION,
    };

    use super::*;

    #[tokio::test]
    async fn tid() {
        let socket = KrpcSocket::server().unwrap();

        let mut requests = socket.inflight_requests;
        assert_eq!(requests.tid(), 0);
        assert_eq!(requests.tid(), 1);
        assert_eq!(requests.tid(), 2);

        requests.next_tid = u32::MAX;

        assert_eq!(requests.tid(), u32::MAX);
        assert_eq!(requests.tid(), 0);
    }

    #[tokio::test]
    async fn recv_request() {
        let mut server = KrpcSocket::server().unwrap();
        let server_address = SocketAddrV4::new([127, 0, 0, 1].into(), server.local_addr().port());

        let mut client = KrpcSocket::client().unwrap();
        client.inflight_requests.next_tid = 120;

        let client_address = client.local_addr();
        let request = RequestSpecific {
            requester_id: Id::random(),
            request_type: RequestTypeSpecific::Ping,
        };

        let expected_request = request.clone();

        client.request(server_address, request);
        client.flush().await;

        let (message, from) = loop {
            if let Some(result) = server.recv_from().await {
                break result;
            }
        };

        assert_eq!(from.port(), client_address.port());
        assert_eq!(message.transaction_id, 120);
        assert!(message.read_only, "Read-only should be true");
        assert_eq!(message.version, Some(VERSION), "Version should be 'RS'");
        assert_eq!(message.message_type, MessageType::Request(expected_request));
    }

    #[tokio::test]
    async fn recv_response() {
        let mut client = KrpcSocket::client().unwrap();
        let client_address = client.local_addr();

        let mut server = KrpcSocket::client().unwrap();
        let server_address = SocketAddrV4::new([127, 0, 0, 1].into(), server.local_addr().port());

        let responder_id = Id::random();
        let response = ResponseSpecific::Ping(PingResponseArguments { responder_id });

        server.inflight_requests.requests.push(InflightRequest {
            tid: 8,
            to: client_address,
            sent_at: Instant::now(),
        });

        client.response(server_address, 8, response);
        client.flush().await;

        let (message, from) = loop {
            if let Some(result) = server.recv_from().await {
                break result;
            }
        };

        assert_eq!(from.port(), client_address.port());
        assert_eq!(message.transaction_id, 8);
        assert!(message.read_only, "Read-only should be true");
        assert_eq!(message.version, Some(VERSION), "Version should be 'RS'");
        assert_eq!(
            message.message_type,
            MessageType::Response(ResponseSpecific::Ping(PingResponseArguments {
                responder_id,
            }))
        );
    }

    #[tokio::test]
    async fn ignore_response_from_wrong_address() {
        let mut server = KrpcSocket::client().unwrap();
        let server_address = server.local_addr();

        let mut client = KrpcSocket::client().unwrap();

        let client_address = client.local_addr();

        server.inflight_requests.requests.push(InflightRequest {
            tid: 8,
            to: SocketAddrV4::new([127, 0, 0, 1].into(), client_address.port() + 1),
            sent_at: Instant::now(),
        });

        let response = ResponseSpecific::Ping(PingResponseArguments {
            responder_id: Id::random(),
        });

        client.response(server_address, 8, response);
        client.flush().await;

        // Use a timeout to avoid hanging - recv should not return a valid response
        let result = tokio::time::timeout(Duration::from_millis(50), server.recv_from()).await;

        assert!(
            result.is_err() || result.unwrap().is_none(),
            "Should not receive a response from wrong address"
        );
    }

    #[tokio::test]
    async fn inflight_request_timeout() {
        let mut server = KrpcSocket::client().unwrap();

        let tid = 8;
        let sent_at = Instant::now();

        server.inflight_requests.requests.push(InflightRequest {
            tid,
            to: SocketAddrV4::new([0, 0, 0, 0].into(), 0),
            sent_at,
        });

        tokio::time::sleep(server.inflight_requests.request_timeout()).await;

        assert!(!server.inflight(&tid));
    }
}
