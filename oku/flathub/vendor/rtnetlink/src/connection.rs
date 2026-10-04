// SPDX-License-Identifier: MIT

use std::io;

use futures_channel::mpsc::UnboundedReceiver;
use netlink_packet_core::NetlinkMessage;
use netlink_packet_route::RouteNetlinkMessage;
use netlink_proto::Connection;
use netlink_sys::{protocols::NETLINK_ROUTE, AsyncSocket, SocketAddr};

use crate::{Handle, MulticastGroup};

#[cfg(feature = "tokio_socket")]
#[allow(clippy::type_complexity)]
pub fn new_connection() -> io::Result<(
    Connection<RouteNetlinkMessage>,
    Handle,
    UnboundedReceiver<(NetlinkMessage<RouteNetlinkMessage>, SocketAddr)>,
)> {
    new_connection_with_socket()
}

/// Equal to `ip monitor` command
#[cfg(feature = "tokio_socket")]
#[allow(clippy::type_complexity)]
pub fn new_multicast_connection(
    groups: &[MulticastGroup],
) -> io::Result<(
    Connection<RouteNetlinkMessage>,
    Handle,
    UnboundedReceiver<(NetlinkMessage<RouteNetlinkMessage>, SocketAddr)>,
)> {
    new_multicast_connection_with_socket(groups)
}

#[allow(clippy::type_complexity)]
pub fn new_connection_with_socket<S>() -> io::Result<(
    Connection<RouteNetlinkMessage, S>,
    Handle,
    UnboundedReceiver<(NetlinkMessage<RouteNetlinkMessage>, SocketAddr)>,
)>
where
    S: AsyncSocket,
{
    let (conn, handle, messages) =
        netlink_proto::new_connection_with_socket(NETLINK_ROUTE)?;
    Ok((conn, Handle::new(handle), messages))
}

/// Equal to `ip monitor` command
#[allow(clippy::type_complexity)]
pub fn new_multicast_connection_with_socket<S>(
    groups: &[MulticastGroup],
) -> io::Result<(
    Connection<RouteNetlinkMessage, S>,
    Handle,
    UnboundedReceiver<(NetlinkMessage<RouteNetlinkMessage>, SocketAddr)>,
)>
where
    S: AsyncSocket,
{
    let (mut conn, handle, messages) =
        netlink_proto::new_connection_with_socket::<RouteNetlinkMessage, S>(
            NETLINK_ROUTE,
        )?;
    let mut all_groups: u32 = 0;
    for group in groups.iter().filter(|g| !g.need_via_add_membership()) {
        all_groups |= 1 << (*group as u32 - 1);
    }

    let addr = SocketAddr::new(0, all_groups);
    conn.socket_mut().socket_mut().bind(&addr)?;

    for group in groups.iter().filter(|g| g.need_via_add_membership()) {
        conn.socket_mut()
            .socket_mut()
            .add_membership(*group as u32)?;
    }

    Ok((conn, Handle::new(handle), messages))
}

#[allow(clippy::type_complexity)]
pub fn from_socket<S>(
    socket: S,
) -> (
    Connection<RouteNetlinkMessage, S>,
    Handle,
    UnboundedReceiver<(NetlinkMessage<RouteNetlinkMessage>, SocketAddr)>,
)
where
    S: AsyncSocket,
{
    let (conn, handle, messages) =
        netlink_proto::from_socket_with_codec(socket);
    (conn, Handle::new(handle), messages)
}
