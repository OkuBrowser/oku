use std::net::SocketAddrV4;

use crate::Id;

use super::Actor;

/// Information and statistics about this mainline node.
#[derive(Debug, Clone)]
pub struct Info {
    id: Id,
    local_addr: SocketAddrV4,
    public_address: Option<SocketAddrV4>,
    firewalled: bool,
    dht_size_estimate: (usize, f64),
    server_mode: bool,

    routing_table_size: usize,
    singing_peers_routing_table_size: usize,
}

impl Info {
    /// This Node's [Id]
    pub fn id(&self) -> &Id {
        &self.id
    }
    /// Local UDP Ipv4 socket address that this node is listening on.
    pub fn local_addr(&self) -> SocketAddrV4 {
        self.local_addr
    }
    /// Returns the best guess for this node's Public address.
    ///
    /// If [crate::DhtBuilder::public_ip] was set, this is what will be returned
    /// (plus the local port), otherwise it will rely on consensus from
    /// responding nodes voting on our public IP and port.
    pub fn public_address(&self) -> Option<SocketAddrV4> {
        self.public_address
    }
    /// Returns `true` if we can't confirm that [Self::public_address] is publicly addressable.
    ///
    /// If this node is firewalled, it won't switch to server mode if it is in adaptive mode,
    /// but if [crate::DhtBuilder::server_mode] was set to true, then whether or not this node is firewalled
    /// won't matter.
    pub fn firewalled(&self) -> bool {
        self.firewalled
    }

    /// Returns whether or not this node is running in server mode.
    pub fn server_mode(&self) -> bool {
        self.server_mode
    }

    /// Returns:
    ///  1. Normal Dht size estimate based on all closer `nodes` in query responses.
    ///  2. Standard deviaiton as a function of the number of samples used in this estimate.
    ///
    /// [Read more](https://github.com/nuhvi/mainline/blob/main/docs/dht_size_estimate.md)
    pub fn dht_size_estimate(&self) -> (usize, f64) {
        self.dht_size_estimate
    }

    /// Returns the size of the main routing table
    pub fn routing_table_size(&self) -> usize {
        self.routing_table_size
    }

    /// Returns the size of the routing table of nodes supporting signed peers BEP
    pub fn singing_peers_routing_table_size(&self) -> usize {
        self.singing_peers_routing_table_size
    }
}

impl From<&Actor> for Info {
    fn from(actor: &Actor) -> Self {
        Self {
            id: *actor.id(),
            public_address: actor.core.public_address,
            firewalled: actor.core.firewalled,
            server_mode: actor.core.server_mode,
            local_addr: actor.socket.local_addr(),
            dht_size_estimate: actor.core.routing_table.dht_size_estimate(),
            routing_table_size: actor.core.routing_table.size(),
            singing_peers_routing_table_size: actor.core.signed_peers_routing_table.size(),
        }
    }
}
