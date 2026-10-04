use std::net::SocketAddrV4;

use crate::Id;

use super::Rpc;

/// Information and statistics about this mainline node.
#[derive(Debug, Clone)]
pub struct Info {
    id: Id,
    local_addr: SocketAddrV4,
    public_address: Option<SocketAddrV4>,
    firewalled: bool,
    dht_size_estimate: (usize, f64),
    server_mode: bool,
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
    /// [Read more](https://github.com/pubky/mainline/blob/main/docs/dht_size_estimate.md)
    pub fn dht_size_estimate(&self) -> (usize, f64) {
        self.dht_size_estimate
    }
}

impl From<&Rpc> for Info {
    fn from(rpc: &Rpc) -> Self {
        Self {
            id: *rpc.id(),
            local_addr: rpc.local_addr(),
            dht_size_estimate: rpc.dht_size_estimate(),
            public_address: rpc.public_address(),
            firewalled: rpc.firewalled(),
            server_mode: rpc.server_mode(),
        }
    }
}
