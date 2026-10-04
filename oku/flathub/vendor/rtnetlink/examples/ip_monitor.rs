// SPDX-License-Identifier: MIT

use futures_util::stream::StreamExt;
use rtnetlink::{new_multicast_connection, MulticastGroup};

#[tokio::main]
async fn main() -> Result<(), String> {
    // conn - `Connection` that has a netlink socket which is a `Future` that
    // polls the socket and thus must have an event loop
    //
    // handle - `Handle` to the `Connection`. Used to send/recv netlink
    // messages.
    //
    // messages - A channel receiver.
    let (conn, mut _handle, mut messages) = new_multicast_connection(&[
        MulticastGroup::Link,
        MulticastGroup::Ipv4Ifaddr,
        MulticastGroup::Ipv6Ifaddr,
        MulticastGroup::Ipv4Route,
        MulticastGroup::Ipv6Route,
        MulticastGroup::MplsRoute,
        MulticastGroup::Ipv4Mroute,
        MulticastGroup::Ipv6Mroute,
        MulticastGroup::Neigh,
        MulticastGroup::Ipv4Netconf,
        MulticastGroup::Ipv6Netconf,
        MulticastGroup::Ipv4Rule,
        MulticastGroup::Ipv6Rule,
        MulticastGroup::Nsid,
        MulticastGroup::MplsNetconf,
    ])
    .map_err(|e| format!("{e}"))?;

    // Spawn `Connection` to start polling netlink socket.
    tokio::spawn(conn);

    // Start receiving events through `messages` channel.
    while let Some((message, _)) = messages.next().await {
        let payload = message.payload;
        println!("{payload:?}");
    }
    Ok(())
}
