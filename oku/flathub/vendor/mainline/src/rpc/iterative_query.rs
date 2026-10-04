//! Manage iterative queries and their corresponding request/response.

use std::collections::HashMap;
use std::collections::HashSet;
use std::net::SocketAddrV4;

use tracing::{debug, trace};

use super::{socket::KrpcSocket, ClosestNodes};
use crate::common::{FindNodeRequestArguments, GetPeersRequestArguments, GetValueRequestArguments};
use crate::{
    common::{Id, Node, RequestSpecific, RequestTypeSpecific, MAX_BUCKET_SIZE_K},
    rpc::Response,
};

/// Aggregate diagnostics for a mutable GET query.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GetMutableOutcome {
    /// Number of unique DHT nodes queried.
    pub queried: u32,
    /// Number of valid mutable values returned.
    pub values: u32,
    /// Number of `NoValues` responses returned.
    pub no_values: u32,
    /// Number of `NoMoreRecentValue` responses returned.
    pub no_more_recent: u32,
    /// Number of mutable value responses that failed validation.
    pub invalid_values: u32,
    /// Number of invalid response shapes returned.
    pub invalid_responses: u32,
    /// Number of KRPC error responses returned.
    pub krpc_errors: u32,
}

impl GetMutableOutcome {
    /// Return the number of nodes that returned a GET response before timing out.
    pub fn responded(&self) -> u32 {
        self.valid_responses() + self.invalid_values + self.invalid_responses + self.krpc_errors
    }

    /// Return the number of nodes that returned a valid GET response.
    pub fn valid_responses(&self) -> u32 {
        self.values + self.no_values + self.no_more_recent
    }

    /// Return the number of queried nodes that did not return a GET response before timeout.
    pub fn timed_out(&self) -> u32 {
        self.queried.saturating_sub(self.responded())
    }

    fn record_value(&mut self) {
        self.values += 1;
    }

    fn record_no_values(&mut self) {
        self.no_values += 1;
    }

    fn record_no_more_recent(&mut self) {
        self.no_more_recent += 1;
    }

    fn record_invalid_value(&mut self) {
        self.invalid_values += 1;
    }

    fn record_invalid_response(&mut self) {
        self.invalid_responses += 1;
    }

    fn record_krpc_error(&mut self) {
        self.krpc_errors += 1;
    }

    fn finish(mut self, queried: u32) -> Self {
        self.queried = queried;
        self
    }
}

/// An iterative process of concurrently sending a request to the closest known nodes to
/// the target, updating the routing table with closer nodes discovered in the responses, and
/// repeating this process until no closer nodes (that aren't already queried) are found.
#[derive(Debug)]
pub(crate) struct IterativeQuery {
    pub request: RequestSpecific,
    closest: ClosestNodes,
    responders: ClosestNodes,
    inflight_requests: Vec<u32>,
    query_requests: Vec<u32>,
    visited: HashSet<SocketAddrV4>,
    responses: Vec<Response>,
    mutable_outcome: GetMutableOutcome,
    public_address_votes: HashMap<SocketAddrV4, u16>,
}

#[derive(Debug)]
pub enum GetRequestSpecific {
    FindNode(FindNodeRequestArguments),
    GetPeers(GetPeersRequestArguments),
    GetValue(GetValueRequestArguments),
}

impl GetRequestSpecific {
    pub fn target(&self) -> &Id {
        match self {
            GetRequestSpecific::FindNode(args) => &args.target,
            GetRequestSpecific::GetPeers(args) => &args.info_hash,
            GetRequestSpecific::GetValue(args) => &args.target,
        }
    }
}

impl IterativeQuery {
    pub fn new(requester_id: Id, target: Id, request: GetRequestSpecific) -> Self {
        let request_type = match request {
            GetRequestSpecific::FindNode(s) => RequestTypeSpecific::FindNode(s),
            GetRequestSpecific::GetPeers(s) => RequestTypeSpecific::GetPeers(s),
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
            query_requests: Vec::new(),
            visited: HashSet::new(),

            responses: Vec::new(),
            mutable_outcome: GetMutableOutcome::default(),

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

    pub fn mutable_outcome(&self) -> GetMutableOutcome {
        self.mutable_outcome
            .clone()
            .finish(self.visited.len() as u32)
    }

    pub fn best_address(&self) -> Option<SocketAddrV4> {
        let mut max = 0_u16;
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

    /// Force start query traversal by visiting closest nodes.
    pub fn start(&mut self, socket: &mut KrpcSocket) {
        self.visit_closest(socket);
    }

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
        self.query_requests.push(tid);

        let tid = socket.request(
            address,
            RequestSpecific {
                requester_id: Id::random(),
                request_type: RequestTypeSpecific::Ping,
            },
        );
        self.inflight_requests.push(tid);

        self.visited.insert(address);
    }

    /// Return true if a response (by transaction_id) is expected by this query.
    pub fn is_inflight(&self, tid: u32) -> bool {
        self.inflight_requests.contains(&tid)
    }

    /// Return true if the transaction belongs to the primary query request, not the liveness ping.
    pub fn is_inflight_query_request(&self, tid: u32) -> bool {
        self.query_requests.contains(&tid)
    }

    /// Add a node that responded with a token as a probable storage node.
    pub fn add_responding_node(&mut self, node: Node) {
        self.responders.add(node)
    }

    /// Store received response.
    pub fn response(&mut self, from: SocketAddrV4, response: Response) {
        let target = self.target();

        debug!(?target, ?response, ?from, "Query got response");

        self.responses.push(response.to_owned());
    }

    pub fn record_mutable_value(&mut self) {
        self.mutable_outcome.record_value();
    }

    pub fn record_no_values(&mut self) {
        self.mutable_outcome.record_no_values();
    }

    pub fn record_no_more_recent(&mut self) {
        self.mutable_outcome.record_no_more_recent();
    }

    pub fn record_invalid_response(&mut self) {
        self.mutable_outcome.record_invalid_response();
    }

    pub fn record_krpc_error(&mut self) {
        self.mutable_outcome.record_krpc_error();
    }

    pub fn record_invalid_mutable_value(&mut self) {
        self.mutable_outcome.record_invalid_value();
    }

    /// Query closest nodes for this query's target and message.
    ///
    /// Returns true if it is done.
    pub fn tick(&mut self, socket: &mut KrpcSocket) -> bool {
        // Visit closest nodes
        self.visit_closest(socket);

        // If no more inflight_requests are inflight in the socket (not timed out),
        // then the query is done.
        let done = !self
            .inflight_requests
            .iter()
            .any(|&tid| socket.inflight(tid));

        if done {
            debug!(id=?self.target(), closest = ?self.closest.len(), visited = ?self.visited.len(), responders = ?self.responders.len(), "Done query");
        };

        done
    }

    // === Private Methods ===

    /// Visit the closest candidates and remove them as candidates
    fn visit_closest(&mut self, socket: &mut KrpcSocket) {
        let to_visit = self
            .closest
            .nodes()
            .iter()
            .take(MAX_BUCKET_SIZE_K)
            .filter(|node| !self.visited.contains(&node.address()))
            .map(|node| node.address())
            .collect::<Vec<_>>();

        for address in to_visit {
            self.visit(socket, address);
        }
    }
}
