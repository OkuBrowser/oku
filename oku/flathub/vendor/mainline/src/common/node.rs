//! Struct and implementation of the Node entry in the Kademlia routing table
use std::{
    fmt::{self, Debug, Formatter},
    net::SocketAddrV4,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::common::Id;

/// The age of a node's last_seen time before it is considered stale and removed from a full bucket
/// on inserting a new node.
pub const STALE_TIME: Duration = Duration::from_secs(15 * 60);
const MIN_PING_BACKOFF_INTERVAL: Duration = Duration::from_secs(10);
pub const TOKEN_ROTATE_INTERVAL: Duration = Duration::from_secs(60 * 5);

#[derive(PartialEq)]
pub(crate) struct NodeInner {
    pub(crate) id: Id,
    pub(crate) address: SocketAddrV4,
    pub(crate) token: Option<Box<[u8]>>,
    pub(crate) last_seen: Instant,
}

impl NodeInner {
    pub fn random() -> Self {
        Self {
            id: Id::random(),
            address: SocketAddrV4::new(0.into(), 0),
            token: None,
            last_seen: Instant::now(),
        }
    }
}

#[derive(Clone, PartialEq)]
/// Node entry in Kademlia routing table
pub struct Node(pub(crate) Arc<NodeInner>);

impl Debug for Node {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Node")
            .field("id", &self.0.id)
            .field("address", &self.0.address)
            .field("last_seen", &self.0.last_seen.elapsed().as_secs())
            .finish()
    }
}

impl Node {
    /// Creates a new Node from an id and socket address.
    pub fn new(id: Id, address: SocketAddrV4) -> Node {
        Node(Arc::new(NodeInner {
            id,
            address,
            token: None,
            last_seen: Instant::now(),
        }))
    }

    pub(crate) fn new_with_token(id: Id, address: SocketAddrV4, token: Box<[u8]>) -> Self {
        Node(Arc::new(NodeInner {
            id,
            address,
            token: Some(token),
            last_seen: Instant::now(),
        }))
    }

    /// Creates a node with random Id for testing purposes.
    pub fn random() -> Node {
        Node(Arc::new(NodeInner::random()))
    }

    /// Create a node that is unique per `i` as it has a random Id and sets IP and port to `i`
    #[cfg(test)]
    pub fn unique(i: usize) -> Node {
        Node::new(Id::random(), SocketAddrV4::new((i as u32).into(), i as u16))
    }

    // === Getters ===

    /// Returns the id of this node
    pub fn id(&self) -> &Id {
        &self.0.id
    }

    /// Returns the address of this node
    pub fn address(&self) -> SocketAddrV4 {
        self.0.address
    }

    /// Returns the token we received from this node if any.
    pub fn token(&self) -> Option<Box<[u8]>> {
        self.0.token.clone()
    }

    /// Node is last seen more than a threshold ago.
    pub fn is_stale(&self) -> bool {
        self.0.last_seen.elapsed() > STALE_TIME
    }

    /// Node's token was received 5 minutes ago or less
    pub fn valid_token(&self) -> bool {
        self.0.last_seen.elapsed() <= TOKEN_ROTATE_INTERVAL
    }

    pub(crate) fn should_ping(&self) -> bool {
        self.0.last_seen.elapsed() > MIN_PING_BACKOFF_INTERVAL
    }

    /// Returns true if both nodes have the same ip and port
    pub fn same_address(&self, other: &Self) -> bool {
        self.0.address == other.0.address
    }

    /// Returns true if both nodes have the same ip
    pub fn same_ip(&self, other: &Self) -> bool {
        self.0.address.ip() == other.0.address.ip()
    }

    /// Node [Id] is valid for its IP address.
    ///
    /// Check [BEP_0042](https://www.bittorrent.org/beps/bep_0042.html).
    pub fn is_secure(&self) -> bool {
        self.0.id.is_valid_for_ip(*self.0.address.ip())
    }

    /// Returns true if Any of the existing nodes:
    ///  - Have the same IP as this node, And:
    ///    = The existing nodes is Not secure.
    ///    = The existing nodes is secure And shares the same first 21 bits.
    ///
    /// Effectively, allows only One non-secure node or Eight secure nodes from the same IP, in the routing table or ClosestNodes.
    pub(crate) fn already_exists(&self, nodes: &[Self]) -> bool {
        nodes.iter().any(|existing| {
            self.same_ip(existing)
                && (!existing.is_secure()
                    || self.id().first_21_bits() == existing.id().first_21_bits())
        })
    }
}
