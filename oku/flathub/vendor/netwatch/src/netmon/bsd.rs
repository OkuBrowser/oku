use std::ops::ControlFlow;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use libc::{RTAX_DST, RTAX_IFP};
use n0_error::stack_error;
use n0_future::{
    task::AbortOnDropHandle,
    time::{self, Duration},
};
use tokio::sync::mpsc;
use tracing::{trace, warn};

use super::actor::NetworkMessage;
#[cfg(any(target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
use crate::interfaces::bsd::{RTAX_DST, RTAX_IFP};
use crate::{
    interfaces::{
        bsd::{WireMessage, parse_rib},
        is_interesting_interface,
    },
    ip::is_link_local,
};

#[derive(Debug)]
pub(super) struct RouteMonitor {
    _handle: AbortOnDropHandle<()>,
}

#[stack_error(derive, add_meta, from_sources, std_sources)]
#[non_exhaustive]
pub enum Error {
    #[error("IO")]
    Io { source: std::io::Error },
}

/// Maximum backoff between socket recreation attempts.
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Initial backoff, grown exponentially toward [`MAX_BACKOFF`] on repeated errors.
const INITIAL_BACKOFF: Duration = Duration::from_millis(50);

fn create_socket() -> std::io::Result<tokio::net::UnixStream> {
    let socket = socket2::Socket::new(libc::AF_ROUTE.into(), socket2::Type::RAW, None)?;
    socket.set_nonblocking(true)?;
    let socket_std: std::os::unix::net::UnixStream = socket.into();
    let socket: tokio::net::UnixStream = socket_std.try_into()?;

    trace!("AF_ROUTE socket bound");

    Ok(socket)
}

impl RouteMonitor {
    pub(super) fn new(sender: mpsc::Sender<NetworkMessage>) -> Result<Self, Error> {
        let socket = create_socket()?;
        let handle = tokio::task::spawn(run(socket, sender));

        Ok(RouteMonitor {
            _handle: AbortOnDropHandle::new(handle),
        })
    }
}

/// Reads routing messages and forwards interesting changes.
///
/// Recreates the socket with backoff on error. Returns when the receiver is
/// gone.
async fn run(mut socket: tokio::net::UnixStream, sender: mpsc::Sender<NetworkMessage>) {
    trace!("AF_ROUTE monitor started");

    let mut buffer = vec![0u8; 2048];
    let mut backoff = INITIAL_BACKOFF;

    loop {
        if let Err(err) = socket.readable().await {
            warn!("AF_ROUTE: error awaiting readable: {:?}", err);
            socket = recreate_socket(&mut backoff).await;
            continue;
        }

        match read_available(&socket, &mut buffer, &sender).await {
            ControlFlow::Break(()) => break,
            ControlFlow::Continue(Ok(_read)) => backoff = INITIAL_BACKOFF,
            ControlFlow::Continue(Err(err)) => {
                warn!("AF_ROUTE: error reading: {:?}", err);
                socket = recreate_socket(&mut backoff).await;
            }
        }
    }
}

/// Drains all currently queued routing messages.
///
/// Sends a [`NetworkMessage::Change`] for each batch with an interesting message
/// and returns the number of messages read, or [`ControlFlow::Break`] once the
/// receiver is gone.
///
/// Drains with `try_read` until `WouldBlock`. Do not read via `AsyncRead::read`
/// one message per await: the fd is registered edge-triggered, so leaving data
/// queued can lose the next readiness notification and permanently stall the
/// monitor. See mio's `Poll` docs on draining edge-triggered readiness.
async fn read_available(
    socket: &tokio::net::UnixStream,
    buffer: &mut Vec<u8>,
    sender: &mpsc::Sender<NetworkMessage>,
) -> ControlFlow<(), std::io::Result<usize>> {
    let mut read_count = 0;
    loop {
        match socket.try_read(buffer) {
            Ok(0) => return ControlFlow::Continue(Ok(read_count)),
            Ok(read) => {
                read_count += 1;
                // Grow buffer if the read filled it, up to 64KiB.
                if read == buffer.len() && buffer.len() < 65536 {
                    buffer.resize(buffer.len() * 2, 0);
                }
                trace!("AF_ROUTE: read {} bytes", read);
                match parse_rib(libc::NET_RT_DUMP, &buffer[..read]) {
                    Ok(msgs) => {
                        if contains_interesting_message(&msgs)
                            && sender.send(NetworkMessage::Change).await.is_err()
                        {
                            return ControlFlow::Break(());
                        }
                    }
                    Err(err) => {
                        warn!("AF_ROUTE: failed to parse rib: {:?}", err);
                    }
                }
            }
            // Fully drained; readiness is re-armed for the next message.
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                return ControlFlow::Continue(Ok(read_count));
            }
            Err(err) => return ControlFlow::Continue(Err(err)),
        }
    }
}

/// Recreates the socket, retrying with exponential backoff until it succeeds.
async fn recreate_socket(backoff: &mut Duration) -> tokio::net::UnixStream {
    loop {
        time::sleep(*backoff).await;
        match create_socket() {
            Ok(socket) => {
                *backoff = INITIAL_BACKOFF;
                return socket;
            }
            Err(err) => {
                warn!("AF_ROUTE: unable to recreate socket: {:?}", err);
                *backoff = (*backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

fn contains_interesting_message(msgs: &[WireMessage]) -> bool {
    msgs.iter().any(is_interesting_message)
}

pub(super) fn is_interesting_message(msg: &WireMessage) -> bool {
    match msg {
        WireMessage::InterfaceMulticastAddr(_) => true,
        WireMessage::Interface(_) => false,
        WireMessage::InterfaceAddr(msg) => {
            if let Some(addr) = msg.addrs.get(RTAX_IFP as usize)
                && let Some(name) = addr.name()
                && !is_interesting_interface(name)
            {
                return false;
            }
            true
        }
        WireMessage::Route(msg) => {
            // RTM_MISS is not a route change and can be emitted often on
            // dual-stack hosts with no IPv6 connectivity.
            if msg.r#type as libc::c_int == libc::RTM_MISS {
                return false;
            }

            // Ignore local unicast
            if let Some(addr) = msg.addrs.get(RTAX_DST as usize)
                && let Some(ip) = addr.ip()
                && is_link_local(ip)
            {
                return false;
            }

            true
        }
        WireMessage::InterfaceAnnounce(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A message [`parse_rib`] skips (wrong `rtm_version`), so it is read but
    /// yields no `Change`.
    fn skipped_message() -> Vec<u8> {
        let mut m = vec![0u8; 8];
        m[..2].copy_from_slice(&8u16.to_ne_bytes()); // rtm_msglen
        m[2] = 0xff; // rtm_version: deliberately not the real one
        m
    }

    /// [`read_available`] must drain every queued message in one readiness
    /// episode, not just one per wakeup. Uses an `AF_UNIX` datagram socketpair
    /// so it is deterministic and independent of the routing subsystem.
    #[tokio::test]
    async fn read_available_drains_all_queued_messages() {
        let (writer, reader) =
            socket2::Socket::pair(socket2::Domain::UNIX, socket2::Type::DGRAM, None)
                .expect("socketpair");
        reader.set_nonblocking(true).expect("nonblocking");

        let msg = skipped_message();
        let mut sent = 0;
        for _ in 0..64 {
            if writer.send(&msg).is_ok() {
                sent += 1;
            } else {
                break;
            }
        }
        assert!(
            sent > 1,
            "expected to queue several datagrams, queued {sent}"
        );

        let reader_std: std::os::unix::net::UnixStream = reader.into();
        let reader = tokio::net::UnixStream::from_std(reader_std).expect("unixstream");
        let (tx, mut rx) = mpsc::channel(8);
        let mut buffer = vec![0u8; 2048];

        reader.readable().await.expect("readable");
        match read_available(&reader, &mut buffer, &tx).await {
            ControlFlow::Continue(Ok(read)) => assert_eq!(
                read, sent,
                "read_available must drain all {sent} queued datagrams in one episode, drained {read}"
            ),
            other => panic!("unexpected control flow: {other:?}"),
        }
        // Skipped messages are not interesting, so no Change is sent.
        assert!(
            rx.try_recv().is_err(),
            "no Change expected for skipped messages"
        );
    }
}
