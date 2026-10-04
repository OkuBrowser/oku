//! Manage iterative queries and their corresponding request/response.

use std::collections::HashMap;
use std::collections::HashSet;
use std::net::SocketAddrV4;

use tracing::{debug, trace};

use crate::ClosestNodes;
use crate::PutRequestSpecific;
use crate::actor::socket::KrpcSocket;
use crate::common::{FindNodeRequestArguments, GetPeersRequestArguments, GetValueRequestArguments};
use crate::common::{Id, MAX_BUCKET_SIZE_K, Node, RequestSpecific, RequestTypeSpecific};
use crate::core::Response;

/// An iterative process of concurrently sending a request to the closest known nodes to
/// the target, updating the routing table with closer nodes discovered in the responses, and
/// repeating this process until no closer nodes (that aren't already queried) are found.
#[derive(Debug)]
pub(crate) struct IterativeQuery {
    pub request: RequestSpecific,
    closest: ClosestNodes,
    responders: ClosestNodes,
    inflight_requests: Vec<u32>,
    visited: HashSet<SocketAddrV4>,
    responses: Vec<Response>,
    public_address_votes: HashMap<SocketAddrV4, u32>,
}

#[derive(Debug)]
pub enum GetRequestSpecific {
    FindNode(FindNodeRequestArguments),
    GetPeers(GetPeersRequestArguments),
    GetSignedPeers(GetPeersRequestArguments),
    GetValue(GetValueRequestArguments),
}

impl GetRequestSpecific {
    pub fn target(&self) -> Id {
        match self {
            GetRequestSpecific::FindNode(args) => args.target,
            GetRequestSpecific::GetPeers(args) => args.info_hash,
            GetRequestSpecific::GetSignedPeers(args) => args.info_hash,
            GetRequestSpecific::GetValue(args) => args.target,
        }
    }
}

impl From<&PutRequestSpecific> for GetRequestSpecific {
    fn from(request: &PutRequestSpecific) -> Self {
        match request {
            PutRequestSpecific::PutImmutable(args) => {
                GetRequestSpecific::GetValue(GetValueRequestArguments {
                    target: args.target,
                    seq: None,
                    salt: None,
                })
            }
            PutRequestSpecific::PutMutable(args) => {
                GetRequestSpecific::GetValue(GetValueRequestArguments {
                    target: args.target,
                    seq: None,
                    salt: args.salt.clone(),
                })
            }
            PutRequestSpecific::AnnouncePeer(args) => {
                GetRequestSpecific::GetPeers(GetPeersRequestArguments {
                    info_hash: args.info_hash,
                })
            }
            PutRequestSpecific::AnnounceSignedPeer(args) => {
                GetRequestSpecific::GetSignedPeers(GetPeersRequestArguments {
                    info_hash: args.info_hash,
                })
            }
        }
    }
}

impl IterativeQuery {
    pub fn new(requester_id: Id, target: Id, request: GetRequestSpecific) -> Self {
        let request_type = match request {
            GetRequestSpecific::FindNode(s) => RequestTypeSpecific::FindNode(s),
            GetRequestSpecific::GetPeers(s) => RequestTypeSpecific::GetPeers(s),
            GetRequestSpecific::GetSignedPeers(s) => RequestTypeSpecific::GetSignedPeers(s),
            GetRequestSpecific::GetValue(s) => RequestTypeSpecific::GetValue(s),
        };

        trace!(?target, ?request_type, "New Query");

        Self {
            request: RequestSpecific {
                requester_id,
                request_type,
            },

            closest: ClosestNodes::new(target),
            responders: ClosestNodes::new(target),

            inflight_requests: Vec::new(),
            visited: HashSet::new(),

            responses: Vec::new(),

            public_address_votes: HashMap::new(),
        }
    }

    // === Getters ===

    pub fn target(&self) -> Id {
        self.responders.target()
    }

    /// Closest nodes according to other nodes.
    pub fn closest(&self) -> &ClosestNodes {
        &self.closest
    }

    /// Return the closest responding nodes after the query is done.
    pub fn responders(&self) -> &ClosestNodes {
        &self.responders
    }

    pub fn responses(&self) -> &[Response] {
        &self.responses
    }

    pub fn best_address(&self) -> Option<SocketAddrV4> {
        let mut max = 0_u32;
        let mut best_addr = None;

        for (addr, count) in self.public_address_votes.iter() {
            if *count > max {
                max = *count;
                best_addr = Some(*addr);
            };
        }

        best_addr
    }

    // === Public Methods ===

    /// Add a candidate node to query on next tick if it is among the closest nodes.
    pub fn add_candidate(&mut self, node: Node) {
        // ready for a ipv6 routing table?
        self.closest.add(node);
    }

    /// Add a vote for this node's address.
    pub fn add_address_vote(&mut self, address: SocketAddrV4) {
        self.public_address_votes
            .entry(address)
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }

    /// Visit explicitly given addresses, and add them to the visited set.
    /// only used from the Rpc when calling bootstrapping nodes.
    pub fn visit(&mut self, socket: &mut KrpcSocket, address: SocketAddrV4) {
        let tid = socket.request(address, self.request.clone());
        self.inflight_requests.push(tid);
        self.visited.insert(address);
    }

    /// Return true if a response (by transaction_id) is expected by this query.
    pub fn inflight(&self, tid: u32) -> bool {
        self.inflight_requests.contains(&tid)
    }

    /// Add a node that responded with a token as a probable storage node.
    pub fn add_responding_node(&mut self, node: Node) {
        self.responders.add(node)
    }

    /// Store received response.
    pub fn response(&mut self, from: SocketAddrV4, response: Response) {
        let target = self.target();

        trace!(?target, ?response, ?from, "Query got response");

        self.responses.push(response.to_owned());
    }

    pub fn visit_closest(&mut self, socket: &mut KrpcSocket) {
        let to_visit = self.closest_candidates();

        for address in to_visit {
            self.visit(socket, address);
        }
    }

    pub fn closest_candidates(&self) -> Vec<SocketAddrV4> {
        self.closest
            .nodes()
            .iter()
            .take(MAX_BUCKET_SIZE_K)
            .filter(|node| !self.visited.contains(&node.address()))
            .map(|node| node.address())
            .collect()
    }

    /// Returns true if it is done.
    pub fn is_done(&self, socket: &KrpcSocket) -> bool {
        // If no more inflight_requests are inflight in the socket (not timed out),
        // then the query is done.
        let done = !self
            .inflight_requests
            .iter()
            .any(|&tid| socket.inflight(&tid));

        if done {
            debug!(id=?self.target(), closest = ?self.closest.len(), visited = ?self.visited.len(), responders = ?self.responders.len(), "Done query");
        };

        done
    }
}
