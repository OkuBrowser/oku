use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
};

use libc::{
    RTNLGRP_IPV4_IFADDR, RTNLGRP_IPV4_ROUTE, RTNLGRP_IPV4_RULE, RTNLGRP_IPV6_IFADDR,
    RTNLGRP_IPV6_ROUTE, RTNLGRP_IPV6_RULE,
};
use n0_error::stack_error;
use n0_future::{
    Stream, StreamExt,
    task::AbortOnDropHandle,
    time::{self, Duration},
};
use netlink_packet_core::{NetlinkMessage, NetlinkPayload};
use netlink_packet_route::{RouteNetlinkMessage, address, route};
use netlink_sys::{AsyncSocket, SocketAddr};
use tokio::sync::mpsc;
use tracing::{trace, warn};

use super::actor::NetworkMessage;
use crate::ip::is_link_local;

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

const fn nl_mgrp(group: u32) -> u32 {
    if group > 31 {
        panic!("use netlink_sys::Socket::add_membership() for this group");
    }
    if group == 0 { 0 } else { 1 << (group - 1) }
}
macro_rules! get_nla {
    ($msg:expr, $nla:path) => {
        $msg.attributes.iter().find_map(|nla| match nla {
            $nla(n) => Some(n),
            _ => None,
        })
    };
}

#[allow(clippy::type_complexity)]
fn setup_netlink() -> std::io::Result<(
    AbortOnDropHandle<()>,
    impl Stream<Item = (NetlinkMessage<RouteNetlinkMessage>, SocketAddr)>,
)> {
    use netlink_sys::protocols::NETLINK_ROUTE;

    let (mut conn, _handle, messages) =
        netlink_proto::new_connection::<RouteNetlinkMessage>(NETLINK_ROUTE)?;

    let groups = nl_mgrp(RTNLGRP_IPV4_IFADDR)
        | nl_mgrp(RTNLGRP_IPV6_IFADDR)
        | nl_mgrp(RTNLGRP_IPV4_ROUTE)
        | nl_mgrp(RTNLGRP_IPV6_ROUTE)
        | nl_mgrp(RTNLGRP_IPV4_RULE)
        | nl_mgrp(RTNLGRP_IPV6_RULE);

    let addr = SocketAddr::new(0, groups);
    conn.socket_mut().socket_mut().bind(&addr)?;

    let conn_handle = AbortOnDropHandle::new(tokio::task::spawn(conn));

    Ok((conn_handle, messages))
}

/// Returns `true` if the connection was lost (should reconnect),
/// `false` if the sender is gone (should shut down).
async fn process_messages(
    sender: &mpsc::Sender<NetworkMessage>,
    messages: &mut (impl Stream<Item = (NetlinkMessage<RouteNetlinkMessage>, SocketAddr)> + Unpin),
) -> bool {
    let mut addr_cache: HashMap<u32, HashSet<IpAddr>> = HashMap::new();

    while let Some((message, _)) = messages.next().await {
        match message.payload {
            NetlinkPayload::Error(err) => {
                warn!("error reading netlink payload: {:?}", err);
            }
            NetlinkPayload::Done(_) => {
                trace!("done received, reconnecting");
                return true;
            }
            NetlinkPayload::InnerMessage(msg) => match msg {
                RouteNetlinkMessage::NewAddress(msg) => {
                    trace!("NEWADDR: {:?}", msg);
                    let addrs = addr_cache.entry(msg.header.index).or_default();
                    if let Some(addr) = get_nla!(msg, address::AddressAttribute::Address) {
                        if addrs.contains(addr) {
                            continue;
                        } else {
                            addrs.insert(*addr);
                            if sender.send(NetworkMessage::Change).await.is_err() {
                                return false;
                            }
                        }
                    }
                }
                RouteNetlinkMessage::DelAddress(msg) => {
                    trace!("DELADDR: {:?}", msg);
                    let addrs = addr_cache.entry(msg.header.index).or_default();
                    if let Some(addr) = get_nla!(msg, address::AddressAttribute::Address) {
                        addrs.remove(addr);
                    }
                    if sender.send(NetworkMessage::Change).await.is_err() {
                        return false;
                    }
                }
                RouteNetlinkMessage::NewRoute(msg) | RouteNetlinkMessage::DelRoute(msg) => {
                    trace!("ROUTE:: {:?}", msg);

                    let table = get_nla!(msg, route::RouteAttribute::Table)
                        .copied()
                        .unwrap_or_default();
                    if let Some(dst) = get_nla!(msg, route::RouteAttribute::Destination) {
                        match dst {
                            route::RouteAddress::Inet(addr)
                                if (table == 255 || table == 254)
                                    && (addr.is_multicast()
                                        || is_link_local(IpAddr::V4(*addr))) =>
                            {
                                continue;
                            }
                            route::RouteAddress::Inet6(addr)
                                if (table == 255 || table == 254)
                                    && (addr.is_multicast()
                                        || is_link_local(IpAddr::V6(*addr))) =>
                            {
                                continue;
                            }
                            _ => {}
                        }
                    }
                    if sender.send(NetworkMessage::Change).await.is_err() {
                        return false;
                    }
                }
                RouteNetlinkMessage::NewRule(msg) => {
                    trace!("NEWRULE: {:?}", msg);
                    if sender.send(NetworkMessage::Change).await.is_err() {
                        return false;
                    }
                }
                RouteNetlinkMessage::DelRule(msg) => {
                    trace!("DELRULE: {:?}", msg);
                    if sender.send(NetworkMessage::Change).await.is_err() {
                        return false;
                    }
                }
                RouteNetlinkMessage::NewLink(msg) => {
                    trace!("NEWLINK: {:?}", msg);
                }
                RouteNetlinkMessage::DelLink(msg) => {
                    trace!("DELLINK: {:?}", msg);
                }
                msg => {
                    trace!("unhandled: {:?}", msg);
                }
            },
            _ => {}
        }
    }

    // Stream ended — connection lost
    true
}

impl RouteMonitor {
    pub(super) fn new(sender: mpsc::Sender<NetworkMessage>) -> Result<Self, Error> {
        let handle = tokio::task::spawn(async move {
            let mut backoff = Duration::from_secs(1);
            const MAX_BACKOFF: Duration = Duration::from_secs(30);

            loop {
                match setup_netlink() {
                    Ok((_conn_handle, mut messages)) => {
                        backoff = Duration::from_secs(1);
                        let should_reconnect = process_messages(&sender, &mut messages).await;
                        // _conn_handle dropped here, aborting the connection task
                        if !should_reconnect {
                            break;
                        }
                        warn!("netlink connection lost, reconnecting");
                    }
                    Err(err) => {
                        warn!("failed to setup netlink: {:?}", err);
                    }
                }
                time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        });

        Ok(RouteMonitor {
            _handle: AbortOnDropHandle::new(handle),
        })
    }
}
