//! K-RPC implementation.

mod closest_nodes;
pub(crate) mod config;
mod info;
mod iterative_query;
mod put_query;
pub(crate) mod server;
mod socket;

use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use lru::LruCache;
use tracing::{debug, error, info};

use iterative_query::IterativeQuery;
use put_query::PutQuery;

use crate::common::{
    validate_immutable, ErrorSpecific, FindNodeRequestArguments, GetImmutableResponseArguments,
    GetMutableResponseArguments, GetPeersResponseArguments, GetValueRequestArguments, Id, Message,
    MessageType, MutableItem, NoMoreRecentValueResponseArguments, NoValuesResponseArguments, Node,
    PutRequestSpecific, RequestSpecific, RequestTypeSpecific, ResponseSpecific, RoutingTable,
    MAX_BUCKET_SIZE_K,
};
use server::Server;

use self::messages::{GetPeersRequestArguments, PutMutableRequestArguments};
use server::ServerSettings;
use socket::KrpcSocket;

pub use crate::common::messages;
pub use closest_nodes::ClosestNodes;
pub use info::Info;
pub use iterative_query::{GetMutableOutcome, GetRequestSpecific};
pub use put_query::{ConcurrencyError, PutError, PutOutcome, PutQueryError};
pub use socket::DEFAULT_REQUEST_TIMEOUT;

/// Default bootstrap nodes used to discover peers when no custom bootstrap list is configured.
pub const DEFAULT_BOOTSTRAP_NODES: [&str; 4] = [
    "router.bittorrent.com:6881",
    "dht.transmissionbt.com:6881",
    "dht.libtorrent.org:25401",
    "relay.pkarr.org:6881",
];

const REFRESH_TABLE_INTERVAL: Duration = Duration::from_secs(15 * 60);
const PING_TABLE_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Result of `tick_get_queries`: completed queries and whether self-findnode finished.
type GetQueriesResult = (Vec<GetQueryOutcome>, bool);

const MAX_CACHED_ITERATIVE_QUERIES: usize = 1000;

#[derive(Debug)]
/// Internal Rpc called in the Dht thread loop, useful to create your own actor setup.
pub struct Rpc {
    // Options
    bootstrap: Box<[SocketAddrV4]>,

    socket: KrpcSocket,

    // Routing
    /// Closest nodes to this node
    routing_table: RoutingTable,
    /// Last time we refreshed the routing table with a find_node query.
    last_table_refresh: Instant,
    /// Last time we pinged nodes in the routing table.
    last_table_ping: Instant,
    /// Closest responding nodes to specific target
    ///
    /// as well as the:
    /// 1. dht size estimate based on closest claimed nodes,
    /// 2. dht size estimate based on closest responding nodes.
    /// 3. number of subnets with unique 6 bits prefix in ipv4
    cached_iterative_queries: LruCache<Id, CachedIterativeQuery>,

    // Active IterativeQueries
    iterative_queries: HashMap<Id, IterativeQuery>,
    /// Put queries are special, since they have to wait for a corresponding
    /// get query to finish, update the closest_nodes, then `query_all` these.
    put_queries: HashMap<Id, PutQuery>,

    /// Sum of Dht size estimates from closest nodes from get queries.
    dht_size_estimates_sum: f64,

    /// Sum of Dht size estimates from closest _responding_ nodes from get queries.
    responders_based_dht_size_estimates_sum: f64,
    responders_based_dht_size_estimates_count: usize,

    /// Sum of the number of subnets with 6 bits prefix in the closest nodes ipv4
    subnets_sum: usize,

    server: Server,

    public_address: Option<SocketAddrV4>,
    firewalled: bool,
}

impl Rpc {
    /// Creates a new RPC instance and prepares the routing table and socket.
    ///
    /// This does not perform network IO by itself. Call [`Rpc::tick`] to bootstrap
    /// and perform scheduled maintenance.
    ///
    /// Returns an instance ready to accept `get`/`put` requests and handle incoming
    /// messages via [`Rpc::handle_message`] if you integrate it with your socket loop.
    pub fn new(config: config::Config) -> Result<Self, std::io::Error> {
        let id = if let Some(ip) = config.public_ip {
            Id::from_ip(ip.into())
        } else {
            Id::random()
        };

        let socket = KrpcSocket::new(&config)?;

        Ok(Rpc {
            bootstrap: config
                .bootstrap
                .unwrap_or_else(|| to_socket_address(&DEFAULT_BOOTSTRAP_NODES))
                .into(),
            socket,

            routing_table: RoutingTable::new(id),
            iterative_queries: HashMap::new(),
            put_queries: HashMap::new(),

            cached_iterative_queries: LruCache::new(
                NonZeroUsize::new(MAX_CACHED_ITERATIVE_QUERIES)
                    .expect("MAX_CACHED_BUCKETS is NonZeroUsize"),
            ),

            last_table_refresh: Instant::now(),
            last_table_ping: Instant::now(),

            dht_size_estimates_sum: 0.0,
            responders_based_dht_size_estimates_count: 0,

            // Don't store to too many nodes just because you are in a cold start.
            responders_based_dht_size_estimates_sum: 1_000_000.0,
            subnets_sum: 20,

            server: Server::new(config.server_settings),

            public_address: None,
            firewalled: true,
        })
    }

    // === Getters ===

    /// Returns the node's Id
    pub fn id(&self) -> &Id {
        self.routing_table.id()
    }

    /// Returns the address the server is listening to.
    #[inline]
    pub fn local_addr(&self) -> SocketAddrV4 {
        self.socket.local_addr()
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
        self.socket.server_mode
    }

    pub fn routing_table(&self) -> &RoutingTable {
        &self.routing_table
    }

    pub fn routing_table_mut(&mut self) -> &mut RoutingTable {
        &mut self.routing_table
    }

    /// Returns:
    ///  1. Normal Dht size estimate based on all closer `nodes` in query responses.
    ///  2. Standard deviaiton as a function of the number of samples used in this estimate.
    ///
    /// [Read more](https://github.com/pubky/mainline/blob/main/docs/dht_size_estimate.md)
    pub fn dht_size_estimate(&self) -> (usize, f64) {
        let normal =
            self.dht_size_estimates_sum as usize / self.cached_iterative_queries.len().max(1);

        // See https://github.com/pubky/mainline/blob/main/docs/standard-deviation-vs-lookups.png
        let std_dev = 0.281 * (self.cached_iterative_queries.len() as f64).powf(-0.529);

        (normal, std_dev)
    }

    /// Returns a thread safe and lightweight summary of this node's
    /// information and statistics.
    pub fn info(&self) -> Info {
        Info::from(self)
    }

    // === Public Methods ===

    /// Advances maintenance and in-flight queries by one step.
    ///
    /// - Performs routing-table refreshes and liveness checks on schedule.
    /// - Progresses outstanding `get`/`put` queries and evicts completed ones.
    /// - May emit newly-available query responses.
    ///
    /// Returns a [`RpcTickReport`] summarizing work done during this call.
    ///
    /// Call this periodically; typical intervals are tied to IO loop cadence
    /// or a fixed timer. Missing calls will delay query completion and degrade
    /// the routing table quality.
    pub fn tick(&mut self) -> RpcTickReport {
        let mut done_put_queries = self.tick_put_queries();

        let (done_get_queries, finished_self_findnode) = self.tick_get_queries();

        self.cleanup_done_queries(&done_get_queries, &mut done_put_queries);

        self.periodic_node_maintenance();

        let new_query_response = self.handle_message();

        if finished_self_findnode {
            self.log_bootstrap(self.id());
        }

        RpcTickReport {
            done_get_queries,
            done_put_queries,
            new_query_response,
        }
    }

    /// Send a request to the given address and return the transaction_id
    pub fn request(&mut self, address: SocketAddrV4, request: RequestSpecific) -> u32 {
        self.socket.request(address, request)
    }

    /// Send a response to the given address.
    pub fn response(
        &mut self,
        address: SocketAddrV4,
        transaction_id: u32,
        response: ResponseSpecific,
    ) {
        self.socket.response(address, transaction_id, response)
    }

    /// Send an error to the given address.
    pub fn error(&mut self, address: SocketAddrV4, transaction_id: u32, error: ErrorSpecific) {
        self.socket.error(address, transaction_id, error)
    }

    /// Store a value in the closest nodes, optionally trigger a lookup query if
    /// the cached closest_nodes aren't fresh enough.
    ///
    /// - `request`: the put request.
    pub fn put(
        &mut self,
        request: PutRequestSpecific,
        extra_nodes: Option<Box<[Node]>>,
    ) -> Result<(), PutError> {
        let target = *request.target();

        if let PutRequestSpecific::PutMutable(PutMutableRequestArguments {
            sig, cas, seq, ..
        }) = &request
        {
            if let Some(PutRequestSpecific::PutMutable(inflight_request)) = self
                .put_queries
                .get(&target)
                .map(|existing| &existing.request)
            {
                debug!(?inflight_request, ?request, "Possible conflict risk");

                if *sig == inflight_request.sig {
                    // Noop, the inflight query is sufficient.
                    return Ok(());
                } else if *seq < inflight_request.seq {
                    return Err(PutError::Concurrency(ConcurrencyError::NotMostRecent));
                } else if let Some(cas) = cas {
                    if *cas == inflight_request.seq {
                        // The user is aware of the inflight query and whiches to overrides it.
                        //
                        // Remove the inflight request, and create a new one.
                        self.put_queries.remove(&target);
                    } else {
                        return Err(PutError::Concurrency(ConcurrencyError::CasFailed));
                    }
                } else {
                    return Err(PutError::Concurrency(ConcurrencyError::ConflictRisk));
                };
            };
        }

        let mut query = PutQuery::new(target, request.clone(), extra_nodes);

        if let Some(closest_nodes) = self
            .cached_iterative_queries
            .get(&target)
            .map(|cached| cached.closest_responding_nodes.clone())
            .filter(|closest_nodes| {
                !closest_nodes.is_empty() && closest_nodes.iter().any(Node::valid_token)
            })
        {
            query.start(&mut self.socket, &closest_nodes)?
        } else {
            let salt = match request {
                PutRequestSpecific::PutMutable(args) => args.salt,
                _ => None,
            };

            self.get(
                GetRequestSpecific::GetValue(GetValueRequestArguments {
                    target,
                    seq: None,
                    salt,
                }),
                None,
            );
        };

        self.put_queries.insert(target, query);

        Ok(())
    }

    /// Send a message to closer and closer nodes until we can't find any more nodes.
    ///
    /// Queries take few seconds to fully traverse the network, once it is done, it will be removed from
    /// self.iterative_queries. But until then, calling [Rpc::get] multiple times, will just return the list
    /// of responses seen so far.
    ///
    /// Subsequent responses can be obtained from the [RpcTickReport::new_query_response] you get after calling [Rpc::tick].
    ///
    /// Effectively, we are caching responses and backing off the network for the duration it takes
    /// to traverse it.
    ///
    /// - `request` [RequestTypeSpecific], except [RequestTypeSpecific::Ping] and
    ///   [RequestTypeSpecific::Put] which will be ignored.
    /// - `extra_nodes` option allows the query to visit specific nodes, that won't necessesarily be visited
    ///   through the query otherwise.
    pub fn get(
        &mut self,
        request: GetRequestSpecific,
        extra_nodes: Option<&[SocketAddrV4]>,
    ) -> Option<Vec<Response>> {
        let target = match request {
            GetRequestSpecific::FindNode(FindNodeRequestArguments { target }) => target,
            GetRequestSpecific::GetPeers(GetPeersRequestArguments { info_hash, .. }) => info_hash,
            GetRequestSpecific::GetValue(GetValueRequestArguments { target, .. }) => target,
        };

        // If query is still active, no need to create a new one.
        if let Some(query) = self.iterative_queries.get(&target) {
            return Some(query.responses().to_vec());
        }

        let node_id = self.routing_table.id();

        if target == *node_id {
            debug!(?node_id, "Bootstrapping the routing table");
        }

        let mut query = IterativeQuery::new(*self.id(), target, request);

        // Seed the query either with the closest nodes from the routing table, or the
        // bootstrapping nodes if the closest nodes are not enough.

        let routing_table_closest = self.routing_table.closest_secure(
            target,
            self.responders_based_dht_size_estimate(),
            self.average_subnets(),
        );

        // If we don't have enough or any closest nodes, call the bootstrapping nodes.
        if routing_table_closest.is_empty() || routing_table_closest.len() < self.bootstrap.len() {
            for bootstrapping_node in self.bootstrap.clone() {
                query.visit(&mut self.socket, bootstrapping_node);
            }
        }

        if let Some(extra_nodes) = extra_nodes {
            for extra_node in extra_nodes {
                query.visit(&mut self.socket, *extra_node)
            }
        }

        // Seed this query with the closest nodes we know about.
        for node in routing_table_closest {
            query.add_candidate(node)
        }

        if let Some(CachedIterativeQuery {
            closest_responding_nodes,
            ..
        }) = self.cached_iterative_queries.get(&target)
        {
            for node in closest_responding_nodes {
                query.add_candidate(node.clone())
            }
        }

        // After adding the nodes, we need to start the query.
        query.start(&mut self.socket);

        self.iterative_queries.insert(target, query);

        None
    }

    // === Private Methods ===

    /// Handles a single inbound KRPC request.
    ///
    /// Responsibilities:
    /// - During initial bootstrap (no known bootstrap nodes), adds the requester of
    ///   a `FindNode` to the routing table to seed it.
    /// - If running in server mode, forwards the request to the embedded server and
    ///   emits a response or error using the provided `transaction_id`.
    /// - Detects successful NAT traversal: when a `Ping` arrives from our own public
    ///   address, clears `firewalled`.
    /// - Ensures node ID/IP consistency: if our ID is invalid for the observed
    ///   public IPv4, generates a new secure ID, resets the routing table, and
    ///   initiates a `FindNode` to repopulate it.
    ///
    /// Parameters:
    /// - `from`: Source socket address of the requester.
    /// - `transaction_id`: Transaction ID to echo in any response/error.
    /// - `request_specific`: Parsed request payload and type.
    ///
    /// Side effects:
    /// - May add `from` to the routing table (bootstrap exception).
    /// - May send a protocol response or error.
    /// - May set `firewalled = false` on self-`Ping`.
    /// - May rotate this node’s ID, reset the routing table, and trigger a
    ///   rebootstrap query.
    ///
    /// Returns: Nothing.
    fn handle_request(
        &mut self,
        from: SocketAddrV4,
        transaction_id: u32,
        request_specific: RequestSpecific,
    ) {
        // By default we only add nodes that responds to our requests.
        //
        // This is the only exception; the first node creating the DHT,
        // without this exception, the bootstrapping node's routing table
        // will never be populated.
        if self.bootstrap.is_empty() {
            if let RequestTypeSpecific::FindNode(param) = &request_specific.request_type {
                self.routing_table.add(Node::new(param.target, from));
            }
        }

        let is_ping = matches!(request_specific.request_type, RequestTypeSpecific::Ping);

        if self.server_mode() {
            let server = &mut self.server;

            match server.handle_request(&self.routing_table, from, request_specific) {
                Some(MessageType::Error(error)) => {
                    self.error(from, transaction_id, error);
                }
                Some(MessageType::Response(response)) => {
                    self.response(from, transaction_id, response);
                }
                _ => {}
            };
        }

        if let Some(our_address) = self.public_address {
            if from == our_address && is_ping {
                self.firewalled = false;

                let ipv4 = our_address.ip();

                // Restarting our routing table with new secure Id if necessary.
                if !self.id().is_valid_for_ip(*ipv4) {
                    let new_id = Id::from_ipv4(*ipv4);

                    info!(
                        "Our current id {} is not valid for address {}. Using new id {}",
                        self.id(),
                        our_address,
                        new_id
                    );

                    self.get(
                        GetRequestSpecific::FindNode(FindNodeRequestArguments { target: new_id }),
                        None,
                    );

                    self.routing_table = RoutingTable::new(new_id);
                }
            }
        }
    }

    /// Handles an inbound KRPC response for RPC, updating in-flight queries and optionally
    /// returning a final value for the associated target.
    ///
    /// Behavior:
    /// - Ignores responses marked read-only, recording them as invalid responses.
    /// - If it matches an in-flight PutQuery, treats `Ping` as a storage ACK (success/error)
    ///   and stops further handling.
    /// - If it matches an in-flight iterative query:
    ///   - Incorporates network info: adds closer candidates, records responder token,
    ///     and votes on the observed requester IP.
    ///   - On value responses:
    ///     - `GetPeers` → returns `(target, Response::Peers)`
    ///     - `GetImmutable` → validates content; on success returns `(target, Response::Immutable)`
    ///     - `GetMutable` → verifies record (sig/seq/salt); on success returns `(target, Response::Mutable)`
    ///   - Logs and continues on `NoValues` / `NoMoreRecentValue` / `Error`.
    /// - On any expected response, adds the responder (by author ID) to the routing table.
    ///
    /// Parameters:
    /// - `from`: Responder socket address.
    /// - `message`: Decoded KRPC message.
    ///
    /// Returns:
    /// - `Some((target, Response))` when a terminal value is obtained for the query.
    /// - `None` otherwise.
    fn handle_response(&mut self, from: SocketAddrV4, message: Message) -> Option<(Id, Response)> {
        // If someone claims to be readonly, then let's not store anything even if they respond.
        if message.read_only {
            if let Some(query) = self
                .iterative_queries
                .values_mut()
                .find(|query| query.is_inflight_query_request(message.transaction_id))
            {
                query.record_invalid_response();
            }

            return None;
        };

        // If the response looks like a Ping response, check StoreQueries for the transaction_id.
        if let Some(query) = self
            .put_queries
            .values_mut()
            .find(|query| query.inflight(message.transaction_id))
        {
            match message.message_type {
                MessageType::Response(ResponseSpecific::Ping(_)) => {
                    // Mark storage at that node as a success.
                    query.success();
                }
                MessageType::Error(error) => query.error(error),
                _ => {}
            };

            return None;
        }

        let mut should_add_node = false;
        let author_id = message.get_author_id();
        let from_version = message.version.to_owned();

        // Get corresponding query for message.transaction_id
        if let Some(query) = self
            .iterative_queries
            .values_mut()
            .find(|query| query.is_inflight(message.transaction_id))
        {
            let is_query_response = query.is_inflight_query_request(message.transaction_id);
            let is_mutable_get_query_response = is_query_response
                && matches!(
                    &query.request.request_type,
                    RequestTypeSpecific::GetValue(_)
                );

            // KrpcSocket would not give us a response from the wrong address for the transaction_id
            should_add_node = true;

            if let Some(nodes) = message.get_closer_nodes() {
                for node in nodes {
                    query.add_candidate(node.clone());
                }
            }

            if let Some((responder_id, token)) = message.get_token() {
                query.add_responding_node(Node::new_with_token(responder_id, from, token.into()));
            }

            if let Some(proposed_ip) = message.requester_ip {
                query.add_address_vote(proposed_ip);
            }

            let target = query.target();

            match message.message_type {
                MessageType::Response(ResponseSpecific::GetPeers(GetPeersResponseArguments {
                    values,
                    ..
                })) => {
                    if is_mutable_get_query_response {
                        query.record_invalid_response();
                    }

                    let response = Response::Peers(values);
                    query.response(from, response.clone());

                    return Some((target, response));
                }
                MessageType::Response(ResponseSpecific::GetImmutable(
                    GetImmutableResponseArguments {
                        v, responder_id, ..
                    },
                )) => {
                    if is_mutable_get_query_response {
                        query.record_invalid_response();
                    }

                    if validate_immutable(&v, query.target()) {
                        let response = Response::Immutable(v);
                        query.response(from, response.clone());

                        return Some((target, response));
                    }

                    let target = query.target();
                    debug!(
                        ?v,
                        ?target,
                        ?responder_id,
                        ?from,
                        ?from_version,
                        "Invalid immutable value"
                    );
                }
                MessageType::Response(ResponseSpecific::GetMutable(
                    GetMutableResponseArguments {
                        v,
                        seq,
                        sig,
                        k,
                        responder_id,
                        ..
                    },
                )) => {
                    let salt = match query.request.request_type.clone() {
                        RequestTypeSpecific::GetValue(args) => args.salt,
                        _ => None,
                    };
                    let target = query.target();

                    match MutableItem::from_dht_message(query.target(), &k, v, seq, &sig, salt) {
                        Ok(item) => {
                            if is_mutable_get_query_response {
                                query.record_mutable_value();
                            }

                            let response = Response::Mutable(item);
                            query.response(from, response.clone());

                            return Some((target, response));
                        }
                        Err(error) => {
                            if is_mutable_get_query_response {
                                query.record_invalid_mutable_value();
                            }

                            debug!(
                                ?error,
                                ?from,
                                ?responder_id,
                                ?from_version,
                                "Invalid mutable record"
                            );
                        }
                    }
                }
                MessageType::Response(ResponseSpecific::NoMoreRecentValue(
                    NoMoreRecentValueResponseArguments {
                        seq, responder_id, ..
                    },
                )) => {
                    if is_mutable_get_query_response {
                        query.record_no_more_recent();
                    }

                    debug!(
                        target= ?query.target(),
                        salt= ?match query.request.request_type.clone() {
                            RequestTypeSpecific::GetValue(args) => args.salt,
                            _ => None,
                        },
                        ?seq,
                        ?from,
                        ?responder_id,
                        ?from_version,
                        "No more recent value"
                    );
                }
                MessageType::Response(ResponseSpecific::NoValues(NoValuesResponseArguments {
                    responder_id,
                    ..
                })) => {
                    if is_mutable_get_query_response {
                        query.record_no_values();
                    }

                    debug!(
                        target= ?query.target(),
                        salt= ?match query.request.request_type.clone() {
                            RequestTypeSpecific::GetValue(args) => args.salt,
                            _ => None,
                        },
                        ?from,
                        ?responder_id,
                        ?from_version ,
                        "No values"
                    );
                }
                MessageType::Error(error) => {
                    if is_mutable_get_query_response {
                        query.record_krpc_error();
                    }

                    debug!(?error, ?from_version, "Get query got error response");
                }
                // Ping response is already handled in add_node()
                // FindNode response is already handled in query.add_candidate()
                // Requests are handled elsewhere
                MessageType::Response(ResponseSpecific::Ping(_))
                | MessageType::Response(ResponseSpecific::FindNode(_))
                | MessageType::Request(_) => {
                    if is_mutable_get_query_response {
                        query.record_invalid_response();
                    }
                }
            };
        };

        if should_add_node {
            // Add a node to our routing table on any expected incoming response.

            if let Some(id) = author_id {
                self.routing_table.add(Node::new(id, from));
            }
        }

        None
    }

    /// Periodically maintain the routing table:
    /// - Switches to server mode if eligible (and refresh is due)
    /// - Pings nodes and purges stale entries when needed
    /// - Repopulates via bootstrap if table is empty or refresh is due
    /// - Updates last_table_refresh and last_table_ping timers as needed
    fn periodic_node_maintenance(&mut self) {
        let refresh_is_due = self.last_table_refresh.elapsed() >= REFRESH_TABLE_INTERVAL;
        let ping_is_due = self.last_table_ping.elapsed() >= PING_TABLE_INTERVAL;

        // Decide first, act once: avoid double populate in the same tick.
        let should_populate = self.routing_table.is_empty() || refresh_is_due;

        if refresh_is_due {
            self.try_switching_to_server_mode();
        }

        if ping_is_due {
            self.ping_and_purge();
        }

        if should_populate {
            self.populate();
        }
    }

    /// Attempts to switch this node into server mode if eligible.
    ///
    /// If the node is not currently operating
    /// in server mode and is not detected as being behind a firewall, it will promote the
    /// node into server mode (by setting the server_mode field to `true`).
    ///
    /// Server mode enables the node to answer unsolicited requests and fulfill a key
    /// responsibility in the DHT. Nodes that are firewalled, or behind NAT, should not
    /// enable server mode unless explicitly configured to do so.
    fn try_switching_to_server_mode(&mut self) {
        if !self.server_mode() && !self.firewalled() {
            info!("Adaptive mode: have been running long enough (not firewalled), switching to server mode");
            self.socket.server_mode = true;
        }
    }

    /// Purge stale nodes and ping nodes that need probing when due is reached.
    ///
    /// It will purge stale nodes from the routing table and periodcially ping nodes.
    /// It will reset the last_table_ping timer.
    fn ping_and_purge(&mut self) {
        self.last_table_ping = Instant::now();

        let (to_purge, to_ping) = self.purge_and_ping_candidates();

        self.purge_nodes(&to_purge);
        self.ping_nodes(&to_ping);

        if to_purge.is_empty() && to_ping.is_empty() {
            return;
        }

        debug!(
            removed = to_purge.len(),
            pinged = to_ping.len(),
            "Node maintenance executed"
        );
    }

    /// Pure decision function: compute which nodes to remove and which to ping.
    fn purge_and_ping_candidates(&self) -> (Vec<Id>, Vec<SocketAddrV4>) {
        let mut to_purge = Vec::with_capacity(self.routing_table.size());
        let mut to_ping = Vec::with_capacity(self.routing_table.size());

        for node in self.routing_table.nodes() {
            if node.is_stale() {
                to_purge.push(*node.id())
            } else if node.should_ping() {
                to_ping.push(node.address())
            }
        }

        (to_purge, to_ping)
    }

    /// Remove nodes from the routing table.
    fn purge_nodes(&mut self, ids: &[Id]) {
        for id in ids {
            self.routing_table.remove(id);
        }
    }

    /// Ping nodes.
    fn ping_nodes(&mut self, addrs: &[SocketAddrV4]) {
        for address in addrs {
            self.ping(*address);
        }
    }

    /// Populate routing table by asking bootstrap nodes to find ourselves,
    /// Response will allow to add closest nodes candidates to routing table.
    ///
    /// Reset the last_table_refresh timer.
    fn populate(&mut self) {
        self.last_table_refresh = Instant::now();

        if self.bootstrap.is_empty() {
            return;
        }

        self.get(
            GetRequestSpecific::FindNode(FindNodeRequestArguments { target: *self.id() }),
            None,
        );
    }

    /// Send a ping request to a node.
    fn ping(&mut self, address: SocketAddrV4) {
        self.socket.request(
            address,
            RequestSpecific {
                requester_id: *self.id(),
                request_type: RequestTypeSpecific::Ping,
            },
        );
    }

    fn update_address_votes_from_iterative_query(&mut self, query: &IterativeQuery) {
        let Some(new_address) = query.best_address() else {
            return;
        };

        let needs_confirm = match self.public_address {
            None => true,
            Some(current) => current != new_address,
        };

        if needs_confirm {
            debug!(
                ?new_address,
                "Query responses suggest a different public_address, trying to confirm.."
            );

            self.firewalled = true;
            self.ping(new_address);
        }

        self.public_address = Some(new_address);
    }

    fn cache_iterative_query(&mut self, query: &IterativeQuery, closest_responding_nodes: &[Node]) {
        if self.cached_iterative_queries.len() >= MAX_CACHED_ITERATIVE_QUERIES {
            let q = self.cached_iterative_queries.pop_lru();
            self.decrement_cached_iterative_query_stats(q.map(|q| q.1));
        }

        let closest = query.closest();
        let responders = query.responders();

        if closest.nodes().is_empty() {
            // We are clearly offline.
            return;
        }

        let dht_size_estimate = closest.dht_size_estimate();
        let responders_dht_size_estimate = responders.dht_size_estimate();
        let subnets_count = closest.subnets_count();

        let previous = self.cached_iterative_queries.put(
            query.target(),
            CachedIterativeQuery {
                closest_responding_nodes: closest_responding_nodes.into(),
                dht_size_estimate,
                responders_dht_size_estimate,
                subnets: subnets_count,

                is_find_node: matches!(
                    query.request.request_type,
                    RequestTypeSpecific::FindNode(_)
                ),
            },
        );

        self.decrement_cached_iterative_query_stats(previous);

        self.dht_size_estimates_sum += dht_size_estimate;
        self.responders_based_dht_size_estimates_sum += responders_dht_size_estimate;
        self.subnets_sum += subnets_count as usize;
        self.responders_based_dht_size_estimates_count += 1;
    }

    fn responders_based_dht_size_estimate(&self) -> usize {
        self.responders_based_dht_size_estimates_sum as usize
            / self.responders_based_dht_size_estimates_count.max(1)
    }

    fn average_subnets(&self) -> usize {
        self.subnets_sum / self.cached_iterative_queries.len().max(1)
    }

    fn decrement_cached_iterative_query_stats(&mut self, query: Option<CachedIterativeQuery>) {
        if let Some(CachedIterativeQuery {
            dht_size_estimate,
            responders_dht_size_estimate,
            subnets,
            is_find_node,
            ..
        }) = query
        {
            self.dht_size_estimates_sum -= dht_size_estimate;
            self.responders_based_dht_size_estimates_sum -= responders_dht_size_estimate;
            self.subnets_sum -= subnets as usize;

            if !is_find_node {
                self.responders_based_dht_size_estimates_count -= 1;
            }
        };
    }

    // === tick() helpers ===

    /// Advance all PUT queries, return done ones.
    fn tick_put_queries(&mut self) -> Vec<(Id, Result<PutOutcome, PutError>)> {
        let mut done_put_queries = Vec::with_capacity(self.put_queries.len());

        for (id, query) in self.put_queries.iter_mut() {
            match query.poll_completion(&self.socket) {
                Ok(Some(outcome)) => done_put_queries.push((*id, Ok(outcome))),
                Ok(None) => (),
                Err(error) => done_put_queries.push((*id, Err(error))),
            };
        }

        done_put_queries
    }

    /// Advance all GET/FIND_NODE queries, return done ones and whether table refresh/find_node to self is finished.
    fn tick_get_queries(&mut self) -> GetQueriesResult {
        let self_id = *self.id();
        let responders_based_dht_size_estimate = self.responders_based_dht_size_estimate();
        let average_subnets = self.average_subnets();

        let mut done_get_queries = Vec::with_capacity(self.iterative_queries.len());
        let mut finished_self_findnode = false;

        for (id, query) in self.iterative_queries.iter_mut() {
            if !query.tick(&mut self.socket) {
                continue;
            }

            let closest_nodes = if let RequestTypeSpecific::FindNode(_) = query.request.request_type
            {
                finished_self_findnode = *id == self_id;

                query
                    .closest()
                    .nodes()
                    .iter()
                    .take(MAX_BUCKET_SIZE_K)
                    .cloned()
                    .collect::<Box<[_]>>()
            } else {
                query
                    .responders()
                    .take_until_secure(responders_based_dht_size_estimate, average_subnets)
                    .to_vec()
                    .into_boxed_slice()
            };

            done_get_queries.push(GetQueryOutcome {
                id: *id,
                closest_nodes,
                mutable_outcome: matches!(
                    query.request.request_type,
                    RequestTypeSpecific::GetValue(_)
                )
                .then(|| query.mutable_outcome()),
            });
        }

        (done_get_queries, finished_self_findnode)
    }

    /// Remove completed GET and PUT queries from internal state.
    fn cleanup_done_queries(
        &mut self,
        done_get: &[GetQueryOutcome],
        done_put: &mut Vec<(Id, Result<PutOutcome, PutError>)>,
    ) {
        // Has to happen _before_ `self.socket.recv_from()`.
        for done in done_get {
            let query = match self.iterative_queries.remove(&done.id) {
                Some(query) => query,
                None => continue,
            };

            self.update_address_votes_from_iterative_query(&query);
            self.cache_iterative_query(&query, &done.closest_nodes);

            // Only for get queries, not find node.
            if matches!(query.request.request_type, RequestTypeSpecific::FindNode(_)) {
                continue;
            }

            let put_query = match self.put_queries.get_mut(&done.id) {
                Some(put_query) => put_query,
                None => continue,
            };

            if put_query.started() {
                continue;
            }

            if let Err(error) = put_query.start(&mut self.socket, &done.closest_nodes) {
                done_put.push((done.id, Err(error)))
            }
        }

        for (id, _) in done_put.iter() {
            self.put_queries.remove(id);
        }
    }

    /// Handle one incoming message, either a request or a response message. One message per tick.
    fn handle_message(&mut self) -> Option<(Id, Response)> {
        self.socket
            .recv_from()
            .and_then(|(message, from)| match message.message_type {
                MessageType::Request(request_specific) => {
                    self.handle_request(from, message.transaction_id, request_specific);
                    None
                }
                _ => self.handle_response(from, message),
            })
    }

    /// Check if routing table is empty and log an error if so.
    fn log_bootstrap(&self, self_id: &Id) {
        let table_size = self.routing_table.size();
        if table_size == 0 {
            error!("Could not bootstrap the routing table");
        } else {
            debug!(?self_id, table_size, "Populated the routing table");
        }
    }
}

struct CachedIterativeQuery {
    closest_responding_nodes: Box<[Node]>,
    dht_size_estimate: f64,
    responders_dht_size_estimate: f64,
    subnets: u8,

    /// Keeping track of find_node queries, because they shouldn't
    /// be counted in `responders_based_dht_size_estimates_count`
    is_find_node: bool,
}

/// State change after a call to [Rpc::tick], including
/// done PUT, GET, and FIND_NODE queries, as well as any
/// incoming value response for any GET query.
#[derive(Debug, Clone)]
/// Completed GET/FIND_NODE query details returned from [Rpc::tick].
pub struct GetQueryOutcome {
    /// Query target id.
    pub id: Id,
    /// Closest responding nodes discovered by the query.
    pub closest_nodes: Box<[Node]>,
    /// Mutable GET diagnostics, present for GET value queries.
    pub mutable_outcome: Option<GetMutableOutcome>,
}

#[derive(Debug, Clone)]
pub struct RpcTickReport {
    /// All the [Id]s of the done [Rpc::get] queries.
    pub done_get_queries: Vec<GetQueryOutcome>,
    /// All the [Id]s of the done [Rpc::put] queries,
    /// and either the successful [PutOutcome] or a [PutError].
    pub done_put_queries: Vec<(Id, Result<PutOutcome, PutError>)>,
    /// Received GET query response.
    pub new_query_response: Option<(Id, Response)>,
}

#[derive(Debug, Clone)]
pub enum Response {
    Peers(Vec<SocketAddrV4>),
    Immutable(Box<[u8]>),
    Mutable(MutableItem),
}

pub(crate) fn to_socket_address<T: ToSocketAddrs>(bootstrap: &[T]) -> Vec<SocketAddrV4> {
    bootstrap
        .iter()
        .flat_map(|s| {
            s.to_socket_addrs().map(|addrs| {
                addrs
                    .filter_map(|addr| match addr {
                        SocketAddr::V4(addr_v4) => Some(addr_v4),
                        _ => None,
                    })
                    .collect::<Box<[_]>>()
            })
        })
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use crate::common::FindNodeResponseArguments;
    use ed25519_dalek::SigningKey;

    use super::*;

    #[test]
    fn get_does_not_echo_inflight_mutable_put() {
        let mut rpc = Rpc::new(config::Config {
            bootstrap: Some(vec![]),
            bind_address: Some(Ipv4Addr::LOCALHOST),
            ..Default::default()
        })
        .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let item = MutableItem::new(signer, b"value", 1000, None);
        let request = PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(item, None));
        let target = *request.target();

        rpc.put(request, None).unwrap();

        let responses = rpc
            .get(
                GetRequestSpecific::GetValue(GetValueRequestArguments {
                    target,
                    seq: None,
                    salt: None,
                }),
                None,
            )
            .unwrap();

        assert!(responses.is_empty());
    }

    #[test]
    fn mutable_get_counts_wrong_primary_response_as_invalid() {
        let mut rpc = Rpc::new(config::Config {
            bootstrap: Some(vec![]),
            bind_address: Some(Ipv4Addr::LOCALHOST),
            ..Default::default()
        })
        .unwrap();

        let target = Id::random();
        let responder = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 42_000);

        rpc.get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                seq: None,
                salt: None,
            }),
            Some(&[responder]),
        );

        let response = rpc.handle_response(
            responder,
            Message {
                transaction_id: 0,
                message_type: MessageType::Response(ResponseSpecific::FindNode(
                    FindNodeResponseArguments {
                        responder_id: Id::random(),
                        nodes: Vec::new().into_boxed_slice(),
                    },
                )),
                version: None,
                read_only: false,
                requester_ip: None,
            },
        );

        let outcome = rpc
            .iterative_queries
            .get(&target)
            .expect("mutable GET query should still be active")
            .mutable_outcome();

        assert!(response.is_none());
        assert_eq!(outcome.valid_responses(), 0);
        assert_eq!(outcome.invalid_responses, 1);
        assert_eq!(outcome.responded(), 1);
        assert_eq!(outcome.timed_out(), 0);
    }
}
