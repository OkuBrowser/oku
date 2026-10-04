use crate::{IfEvent, IpNet, Ipv4Net, Ipv6Net};
use fnv::FnvHashSet;
use futures::ready;
use futures::stream::{FusedStream, Stream, TryStreamExt};
use futures::StreamExt;
use netlink_packet_core::NetlinkPayload;
use netlink_packet_route::address::{AddressAttribute, AddressMessage};
use netlink_packet_route::RouteNetlinkMessage;
use netlink_proto::Connection;
use netlink_sys::{AsyncSocket, SocketAddr};
use rtnetlink::constants::{RTMGRP_IPV4_IFADDR, RTMGRP_IPV6_IFADDR};
use std::collections::VecDeque;
use std::future::Future;
use std::io::{ErrorKind, Result};
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(feature = "tokio")]
pub mod tokio {
    //! An interface watcher that uses `netlink`'s [`TokioSocket`](netlink_sys::TokioSocket)
    use netlink_sys::TokioSocket;

    /// Watches for interface changes.
    pub type IfWatcher = super::IfWatcher<TokioSocket>;
}

#[cfg(feature = "smol")]
pub mod smol {
    //! An interface watcher that uses `netlink`'s [`SmolSocket`](netlink_sys::SmolSocket)
    use netlink_sys::SmolSocket;

    /// Watches for interface changes.
    pub type IfWatcher = super::IfWatcher<SmolSocket>;
}

pub struct IfWatcher<T> {
    conn: Connection<RouteNetlinkMessage, T>,
    messages: Pin<Box<dyn Stream<Item = Result<RouteNetlinkMessage>> + Send>>,
    addrs: FnvHashSet<IpNet>,
    queue: VecDeque<IfEvent>,
}

impl<T> std::fmt::Debug for IfWatcher<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("IfWatcher")
            .field("addrs", &self.addrs)
            .finish_non_exhaustive()
    }
}

impl<T> IfWatcher<T>
where
    T: AsyncSocket + Unpin,
{
    /// Create a watcher.
    pub fn new() -> Result<Self> {
        let (mut conn, handle, messages) = rtnetlink::new_connection_with_socket::<T>()?;
        let groups = RTMGRP_IPV4_IFADDR | RTMGRP_IPV6_IFADDR;
        let addr = SocketAddr::new(0, groups);
        conn.socket_mut().socket_mut().bind(&addr)?;
        let get_addrs_stream = handle
            .address()
            .get()
            .execute()
            .map_ok(RouteNetlinkMessage::NewAddress)
            .map_err(std::io::Error::other);
        let msg_stream = messages.filter_map(|(msg, _)| async {
            match msg.payload {
                NetlinkPayload::Error(err) => Some(Err(err.to_io())),
                NetlinkPayload::InnerMessage(msg) => Some(Ok(msg)),
                _ => None,
            }
        });
        let messages = get_addrs_stream.chain(msg_stream).boxed();
        let addrs = FnvHashSet::default();
        let queue = VecDeque::default();
        Ok(Self {
            conn,
            messages,
            addrs,
            queue,
        })
    }

    /// Iterate over current networks.
    pub fn iter(&self) -> impl Iterator<Item = &IpNet> {
        self.addrs.iter()
    }

    fn add_address(&mut self, msg: AddressMessage) {
        for net in iter_nets(msg) {
            if self.addrs.insert(net) {
                self.queue.push_back(IfEvent::Up(net));
            }
        }
    }

    fn rem_address(&mut self, msg: AddressMessage) {
        for net in iter_nets(msg) {
            if self.addrs.remove(&net) {
                self.queue.push_back(IfEvent::Down(net));
            }
        }
    }

    /// Poll for an address change event.
    pub fn poll_if_event(&mut self, cx: &mut Context) -> Poll<Result<IfEvent>> {
        loop {
            if let Some(event) = self.queue.pop_front() {
                return Poll::Ready(Ok(event));
            }
            if Pin::new(&mut self.conn).poll(cx).is_ready() {
                return Poll::Ready(Err(socket_err(
                    "rtnetlink socket closed. Connection has been terminated.",
                )));
            }
            let message = match ready!(self.messages.poll_next_unpin(cx)) {
                Some(Ok(message)) => message,
                Some(Err(error)) => {
                    return Poll::Ready(Err(socket_err(&format!(
                        "rtnetlink socket closed. {error}"
                    ))));
                }
                None => {
                    return Poll::Ready(Err(socket_err(
                        "rtnetlink socket closed. Empty message has been returned.",
                    )));
                }
            };
            match message {
                RouteNetlinkMessage::NewAddress(msg) => self.add_address(msg),
                RouteNetlinkMessage::DelAddress(msg) => self.rem_address(msg),
                _ => {}
            }
        }
    }
}

fn socket_err(error: &str) -> std::io::Error {
    std::io::Error::new(ErrorKind::BrokenPipe, error)
}

fn iter_nets(msg: AddressMessage) -> impl Iterator<Item = IpNet> {
    let prefix = msg.header.prefix_len;
    msg.attributes.into_iter().filter_map(move |attr| {
        if let AddressAttribute::Address(addr) = attr {
            match addr {
                std::net::IpAddr::V4(ipv4) => Some(IpNet::V4(Ipv4Net::new(ipv4, prefix).unwrap())),
                std::net::IpAddr::V6(ipv6) => Some(IpNet::V6(Ipv6Net::new(ipv6, prefix).unwrap())),
            }
        } else {
            None
        }
    })
}

impl<T> Stream for IfWatcher<T>
where
    T: AsyncSocket + Unpin,
{
    type Item = Result<IfEvent>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::into_inner(self).poll_if_event(cx).map(Some)
    }
}

impl<T> FusedStream for IfWatcher<T>
where
    T: AsyncSocket + AsyncSocket + Unpin,
{
    fn is_terminated(&self) -> bool {
        false
    }
}
