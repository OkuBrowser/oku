//! Dht node.

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4, ToSocketAddrs},
    thread,
    time::Duration,
};

use flume::{Receiver, Sender, TryRecvError};

use tracing::info;

use crate::{
    common::{
        hash_immutable, most_recent_mutable_item, AnnouncePeerRequestArguments,
        FindNodeRequestArguments, GetPeersRequestArguments, GetValueRequestArguments, Id,
        MutableItem, PutImmutableRequestArguments, PutMutableRequestArguments, PutRequestSpecific,
    },
    rpc::{
        to_socket_address, ConcurrencyError, GetMutableOutcome, GetRequestSpecific, Info, PutError,
        PutOutcome, PutQueryError, Response, Rpc,
    },
    Node, ServerSettings,
};

use crate::rpc::config::Config;

#[derive(Debug, Clone)]
/// Mainline Dht node.
pub struct Dht(pub(crate) Sender<ActorMessage>);

#[derive(Debug, Default, Clone)]
/// A builder for the [Dht] node.
pub struct DhtBuilder(Config);

impl DhtBuilder {
    /// Set this node's server_mode.
    pub fn server_mode(&mut self) -> &mut Self {
        self.0.server_mode = true;

        self
    }

    /// Set a custom settings for the node to use at server mode.
    ///
    /// Defaults to [ServerSettings::default]
    pub fn server_settings(&mut self, server_settings: ServerSettings) -> &mut Self {
        self.0.server_settings = server_settings;

        self
    }

    /// Set bootstrapping nodes.
    pub fn bootstrap<T: ToSocketAddrs>(&mut self, bootstrap: &[T]) -> &mut Self {
        self.0.bootstrap = Some(to_socket_address(bootstrap));

        self
    }

    /// Add more bootstrap nodes to default bootstrapping nodes.
    ///
    /// Useful when you want to augment the default bootstrapping nodes with
    /// dynamic list of nodes you have seen in previous sessions.
    pub fn extra_bootstrap<T: ToSocketAddrs>(&mut self, extra_bootstrap: &[T]) -> &mut Self {
        let mut bootstrap = self.0.bootstrap.clone().unwrap_or_default();
        for address in to_socket_address(extra_bootstrap) {
            bootstrap.push(address);
        }
        self.0.bootstrap = Some(bootstrap);

        self
    }

    /// Remove the existing bootstrapping nodes, usually to create the first node in a new network.
    pub fn no_bootstrap(&mut self) -> &mut Self {
        self.0.bootstrap = Some(vec![]);

        self
    }

    /// Set an explicit port to listen on.
    pub fn port(&mut self, port: u16) -> &mut Self {
        self.0.port = Some(port);

        self
    }

    /// A known public IPv4 address for this node to generate
    /// a secure node Id from according to [BEP_0042](https://www.bittorrent.org/beps/bep_0042.html)
    ///
    /// Defaults to depending on suggestions from responding nodes.
    pub fn public_ip(&mut self, public_ip: Ipv4Addr) -> &mut Self {
        self.0.public_ip = Some(public_ip);

        self
    }

    /// UDP socket request timeout duration.
    ///
    /// The longer this duration is, the longer queries take until they are deemeed "done".
    /// The shortet this duration is, the more responses from busy nodes we miss out on,
    /// which affects the accuracy of queries trying to find closest nodes to a target.
    ///
    /// Defaults to [crate::DEFAULT_REQUEST_TIMEOUT]
    pub fn request_timeout(&mut self, request_timeout: Duration) -> &mut Self {
        self.0.request_timeout = request_timeout;

        self
    }

    /// Set the address to bind to.
    ///
    /// Defaults to 0.0.0.0 (all interfaces).
    pub fn bind_address(&mut self, bind_address: Ipv4Addr) -> &mut Self {
        self.0.bind_address = Some(bind_address);

        self
    }

    /// Create a Dht node.
    pub fn build(&self) -> Result<Dht, std::io::Error> {
        Dht::new(self.0.clone())
    }
}

impl Dht {
    /// Create a new Dht node.
    ///
    /// Could return an error if it failed to bind to the specified
    /// port or other io errors while binding the udp socket.
    pub fn new(config: Config) -> Result<Self, std::io::Error> {
        let (sender, receiver) = flume::unbounded();

        thread::Builder::new()
            .name("Mainline Dht actor thread".to_string())
            .spawn(move || run(config, receiver))?;

        let (tx, rx) = flume::bounded(1);

        sender
            .send(ActorMessage::Check(tx))
            .expect("actor thread unexpectedly shutdown");

        rx.recv().expect("actor thread unexpectedly shutdown")?;

        Ok(Dht(sender))
    }

    /// Returns a builder to edit settings before creating a Dht node.
    pub fn builder() -> DhtBuilder {
        DhtBuilder::default()
    }

    /// Create a new DHT client with default bootstrap nodes.
    pub fn client() -> Result<Self, std::io::Error> {
        Dht::builder().build()
    }

    /// Create a new DHT node that is running in [Server mode][DhtBuilder::server_mode] as
    /// soon as possible.
    ///
    /// You shouldn't use this option unless you are sure your
    /// DHT node is publicly accessible (not firewalled) _AND_ will be long running,
    /// and/or you are running your own local network for testing.
    ///
    /// If you are not sure, use [Self::client] and it will switch
    /// to server mode when/if these two conditions are met.
    pub fn server() -> Result<Self, std::io::Error> {
        Dht::builder().server_mode().build()
    }

    // === Getters ===

    /// Information and statistics about this [Dht] node.
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn info(&self) -> Info {
        let (tx, rx) = flume::bounded::<Info>(1);
        self.send(ActorMessage::Info(tx));

        rx.recv().expect("actor thread unexpectedly shutdown")
    }

    /// Turn this node's routing table to a list of bootstrapping nodes.   
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn to_bootstrap(&self) -> Vec<String> {
        let (tx, rx) = flume::bounded::<Vec<String>>(1);
        self.send(ActorMessage::ToBootstrap(tx));

        rx.recv().expect("actor thread unexpectedly shutdown")
    }

    // === Public Methods ===

    /// Block until the bootstrapping query is done.
    ///
    /// Returns true if the bootstrapping was successful.
    #[allow(deprecated)]
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn bootstrapped(&self) -> bool {
        let info = self.info();
        let nodes = self.find_node(*info.id());

        !nodes.is_empty()
    }

    // === Find nodes ===

    /// Returns the closest 20 [secure](Node::is_secure) nodes to a target [Id].
    ///
    /// Mostly useful to crawl the DHT.
    ///
    /// The returned nodes are claims by other nodes, they may be lies, or may have churned
    /// since they were last seen, but haven't been pinged yet.
    ///
    /// You might need to ping them to confirm they exist, and responsive, or if you want to
    /// learn more about them like the client they are using, or if they support a given BEP.
    ///
    /// If you are trying to find the closest nodes to a target with intent to [Self::put],
    /// a request directly to these nodes (using `extra_nodes` parameter), then you should
    /// use [Self::get_closest_nodes] instead.
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn find_node(&self, target: Id) -> Box<[Node]> {
        let (tx, rx) = flume::bounded::<Box<[Node]>>(1);
        self.send(ActorMessage::Get(
            GetRequestSpecific::FindNode(FindNodeRequestArguments { target }),
            ResponseSender::ClosestNodes(tx),
        ));

        rx.recv()
            .expect("Query was dropped before sending a response, please open an issue.")
    }

    // === Peers ===

    /// Get peers for a given infohash.
    ///
    /// Note: each node of the network will only return a _random_ subset (usually 20)
    /// of the total peers it has for a given infohash, so if you are getting responses
    /// from 20 nodes, you can expect up to 400 peers in total, but if there are more
    /// announced peers on that infohash, you are likely to miss some, the logic here
    /// for Bittorrent is that any peer will introduce you to more peers through "peer exchange"
    /// so if you are implementing something different from Bittorrent, you might want
    /// to implement your own logic for gossipping more peers after you discover the first ones.
    #[allow(deprecated)]
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn get_peers(&self, info_hash: Id) -> GetIterator<Vec<SocketAddrV4>> {
        let (tx, rx) = flume::unbounded::<Vec<SocketAddrV4>>();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetPeers(GetPeersRequestArguments { info_hash }),
            ResponseSender::Peers(tx),
        ));

        GetIterator(rx.into_iter())
    }

    /// Announce a peer for a given infohash.
    ///
    /// The peer will be announced on this process IP.
    /// If explicit port is passed, it will be used, otherwise the port will be implicitly
    /// assumed by remote nodes to be the same ase port they received the request from.
    #[allow(deprecated)]
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn announce_peer(&self, info_hash: Id, port: Option<u16>) -> Result<Id, PutQueryError> {
        let (port, implied_port) = match port {
            Some(port) => (port, None),
            None => (0, Some(true)),
        };

        self.put(
            PutRequestSpecific::AnnouncePeer(AnnouncePeerRequestArguments {
                info_hash,
                port,
                implied_port,
            }),
            None,
        )
        .map_err(|error| match error {
            PutError::Query(error) => error,
            PutError::Concurrency(_) => {
                unreachable!("should not receive a concurrency error from announce peer query")
            }
        })
    }

    // === Immutable data ===

    /// Get an Immutable data by its sha1 hash.
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn get_immutable(&self, target: Id) -> Option<Box<[u8]>> {
        let (tx, rx) = flume::unbounded::<Box<[u8]>>();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                seq: None,
                salt: None,
            }),
            ResponseSender::Immutable(tx),
        ));

        rx.recv().map(Some).unwrap_or(None)
    }

    /// Put an immutable data to the DHT.
    #[allow(deprecated)]
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn put_immutable(&self, value: &[u8]) -> Result<Id, PutQueryError> {
        let target: Id = hash_immutable(value).into();

        self.put(
            PutRequestSpecific::PutImmutable(PutImmutableRequestArguments {
                target,
                v: value.into(),
            }),
            None,
        )
        .map_err(|error| match error {
            PutError::Query(error) => error,
            PutError::Concurrency(_) => {
                unreachable!("should not receive a concurrency error from put immutable query")
            }
        })
    }

    // === Mutable data ===

    /// Get a mutable data by its `public_key` and optional `salt`.
    ///
    /// You can ask for items `more_recent_than` than a certain `seq`,
    /// usually one that you already have seen before, similar to `If-Modified-Since` header in HTTP.
    ///
    /// # Order
    ///
    /// The order of [MutableItem]s returned by this iterator is not guaranteed to
    /// reflect their `seq` value. You should not assume that the later items are
    /// more recent than earlier ones.
    ///
    /// Consider using [Self::get_mutable_most_recent] if that is what you need.
    #[allow(deprecated)]
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn get_mutable(
        &self,
        public_key: &[u8; 32],
        salt: Option<&[u8]>,
        more_recent_than: Option<i64>,
    ) -> GetIterator<MutableItem> {
        let salt = salt.map(|s| s.into());
        let target = MutableItem::target_from_key(public_key, salt.as_deref());
        let (tx, rx) = flume::unbounded::<MutableItem>();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                seq: more_recent_than,
                salt,
            }),
            ResponseSender::Mutable(tx),
        ));

        GetIterator(rx.into_iter())
    }

    /// Get the most recent [MutableItem] from the network.
    #[allow(deprecated)]
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn get_mutable_most_recent(
        &self,
        public_key: &[u8; 32],
        salt: Option<&[u8]>,
    ) -> Option<MutableItem> {
        self.get_mutable(public_key, salt, None)
            .fold(None::<MutableItem>, most_recent_mutable_item)
    }

    /// Put a mutable data to the DHT.
    ///
    /// # Lost Update Problem
    ///
    /// As mainline DHT is a distributed system, it is vulnerable to [Write–write conflict](https://en.wikipedia.org/wiki/Write-write_conflict).
    ///
    /// ## Read first
    ///
    /// To mitigate the risk of lost updates, you should call the [Self::get_mutable_most_recent] method
    /// then start authoring the new [MutableItem] based on the most recent as in the following example:
    ///
    ///```rust
    /// use mainline::{Dht, MutableItem, SigningKey, Testnet};
    /// use std::net::Ipv4Addr;
    ///
    /// let testnet = Testnet::builder(3).build().unwrap();
    /// let dht = Dht::builder()
    ///     .bootstrap(&testnet.bootstrap)
    ///     .bind_address(Ipv4Addr::LOCALHOST)
    ///     .build()
    ///     .unwrap();
    ///
    /// let signing_key = SigningKey::from_bytes(&[0; 32]);
    /// let key = signing_key.verifying_key().to_bytes();
    /// let salt = Some(b"salt".as_ref());
    ///
    /// let (item, cas) = if let Some(most_recent) = dht .get_mutable_most_recent(&key, salt) {
    ///     // 1. Optionally Create a new value to take the most recent's value in consideration.
    ///     let mut new_value = most_recent.value().to_vec();
    ///     new_value.extend_from_slice(b" more data");
    ///
    ///     // 2. Increment the sequence number to be higher than the most recent's.
    ///     let most_recent_seq = most_recent.seq();
    ///     let new_seq = most_recent_seq + 1;
    ///
    ///     (
    ///         MutableItem::new(signing_key, &new_value, new_seq, salt),
    ///         // 3. Use the most recent [MutableItem::seq] as a `CAS`.
    ///         Some(most_recent_seq)
    ///     )
    /// } else {
    ///     (MutableItem::new(signing_key, b"first value", 1, salt), None)
    /// };
    ///
    /// dht.put_mutable(item, cas).unwrap();
    /// ```
    ///
    /// ## Errors
    ///
    /// In addition to the [PutQueryError] common with all PUT queries, PUT mutable item
    /// query has other [Concurrency errors][ConcurrencyError], that try to detect write conflict
    /// risks or obvious conflicts.
    ///
    /// If you are lucky to get one of these errors (which is not guaranteed), then you should
    /// read the most recent item again, and repeat the steps in the previous example.
    #[allow(deprecated)]
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn put_mutable(&self, item: MutableItem, cas: Option<i64>) -> Result<Id, PutMutableError> {
        let request = PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(item, cas));

        self.put(request, None).map_err(|error| match error {
            PutError::Query(err) => PutMutableError::Query(err),
            PutError::Concurrency(err) => PutMutableError::Concurrency(err),
        })
    }

    // === Raw ===

    /// Get closet nodes to a specific target, that support [BEP_0044](https://www.bittorrent.org/beps/bep_0044.html).
    ///
    /// Useful to [Self::put] a request to nodes further from the 20 closest nodes to the
    /// [PutRequestSpecific::target]. Which itself is useful to circumvent [extreme vertical sybil attacks](https://github.com/pubky/mainline/blob/main/docs/censorship-resistance.md#extreme-vertical-sybil-attacks).
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn get_closest_nodes(&self, target: Id) -> Box<[Node]> {
        let (tx, rx) = flume::unbounded::<Box<[Node]>>();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                salt: None,
                seq: None,
            }),
            ResponseSender::ClosestNodes(tx),
        ));

        rx.recv()
            .expect("Query was dropped before sending a response, please open an issue.")
    }

    /// Send a PUT request to the closest nodes, and optionally some extra nodes.
    ///
    /// This is useful to put data to regions of the DHT other than the closest nodes
    /// to this request's [target][PutRequestSpecific::target].
    ///
    /// You can find nodes close to other regions of the network by calling
    /// [Self::get_closest_nodes] with the target that you want to find the closest nodes to.
    ///
    /// Note: extra nodes need to have [Node::valid_token].
    #[deprecated(note = "use the async API via Dht::as_async() instead")]
    pub fn put(
        &self,
        request: PutRequestSpecific,
        extra_nodes: Option<Box<[Node]>>,
    ) -> Result<Id, PutError> {
        self.put_inner(request, extra_nodes)
            .recv()
            .expect("Query was dropped before sending a response, please open an issue.")
            .map(|outcome| outcome.target)
    }

    // === Private Methods ===

    pub(crate) fn put_inner(
        &self,
        request: PutRequestSpecific,
        extra_nodes: Option<Box<[Node]>>,
    ) -> flume::Receiver<Result<PutOutcome, PutError>> {
        let (tx, rx) = flume::bounded::<Result<PutOutcome, PutError>>(1);
        self.send(ActorMessage::Put(request, tx, extra_nodes));

        rx
    }

    pub(crate) fn send(&self, message: ActorMessage) {
        self.0
            .send(message)
            .expect("actor thrread unexpectedly shutdown");
    }
}

#[deprecated(note = "use the async stream returned by AsyncDht instead")]
pub struct GetIterator<T>(flume::IntoIter<T>);

#[allow(deprecated)]
impl<T> Iterator for GetIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

fn run(config: Config, receiver: Receiver<ActorMessage>) {
    match Rpc::new(config) {
        Ok(mut rpc) => {
            let address = rpc.local_addr();
            info!(?address, "Mainline DHT listening");

            let mut put_senders = HashMap::new();
            let mut get_senders = HashMap::new();

            loop {
                match receiver.try_recv() {
                    Ok(actor_message) => match actor_message {
                        ActorMessage::Check(sender) => {
                            let _ = sender.send(Ok(()));
                        }
                        ActorMessage::Info(sender) => {
                            let _ = sender.send(rpc.info());
                        }
                        ActorMessage::Put(request, sender, extra_nodes) => {
                            let target = *request.target();

                            match rpc.put(request, extra_nodes) {
                                Ok(()) => {
                                    let senders = put_senders.entry(target).or_insert(vec![]);

                                    senders.push(sender);
                                }
                                Err(error) => {
                                    let _ = sender.send(Err(error));
                                }
                            };
                        }
                        ActorMessage::Get(request, sender) => {
                            let target = *request.target();

                            if let Some(responses) = rpc.get(request, None) {
                                for response in responses {
                                    send(&sender, response);
                                }
                            };

                            let senders = get_senders.entry(target).or_insert(vec![]);

                            senders.push(sender);
                        }
                        ActorMessage::ToBootstrap(sender) => {
                            let _ = sender.send(rpc.routing_table().to_bootstrap());
                        }
                        ActorMessage::SeedRouting(nodes, sender) => {
                            for node in nodes {
                                rpc.routing_table_mut().add(node);
                            }
                            let _ = sender.send(());
                        }
                    },
                    Err(TryRecvError::Disconnected) => {
                        // Node was dropped, kill this thread.
                        tracing::debug!("mainline::Dht's actor thread was shutdown after Drop.");
                        break;
                    }
                    Err(TryRecvError::Empty) => {
                        // No op
                    }
                }

                let report = rpc.tick();

                // Response for an ongoing GET query
                if let Some((target, response)) = report.new_query_response {
                    if let Some(senders) = get_senders.get(&target) {
                        for sender in senders {
                            send(sender, response.clone());
                        }
                    }
                }

                // Cleanup done GET queries
                for done in report.done_get_queries {
                    if let Some(senders) = get_senders.remove(&done.id) {
                        for sender in senders {
                            match sender {
                                ResponseSender::ClosestNodes(sender) => {
                                    let _ = sender.send(done.closest_nodes.clone());
                                }
                                ResponseSender::MutableDetailed { outcome, .. } => {
                                    let _ = outcome
                                        .send(done.mutable_outcome.clone().unwrap_or_default());
                                }
                                ResponseSender::Peers(_)
                                | ResponseSender::Mutable(_)
                                | ResponseSender::Immutable(_) => {}
                            }
                        }
                    }
                }

                // Cleanup done PUT query and send its final result.
                for (id, result) in report.done_put_queries {
                    if let Some(senders) = put_senders.remove(&id) {
                        for sender in senders {
                            let _ = sender.send(result.clone());
                        }
                    }
                }
            }
        }
        Err(err) => {
            if let Ok(ActorMessage::Check(sender)) = receiver.try_recv() {
                let _ = sender.send(Err(err));
            }
        }
    };
}

fn send(sender: &ResponseSender, response: Response) {
    match (sender, response) {
        (ResponseSender::Peers(s), Response::Peers(r)) => {
            let _ = s.send(r);
        }
        (ResponseSender::Mutable(s), Response::Mutable(r)) => {
            let _ = s.send(r);
        }
        (ResponseSender::MutableDetailed { values, .. }, Response::Mutable(r)) => {
            let _ = values.send(r);
        }
        (ResponseSender::Immutable(s), Response::Immutable(r)) => {
            let _ = s.send(r);
        }
        _ => {}
    }
}

#[derive(Debug)]
pub(crate) enum ActorMessage {
    Info(Sender<Info>),
    Put(
        PutRequestSpecific,
        Sender<Result<PutOutcome, PutError>>,
        Option<Box<[Node]>>,
    ),
    Get(GetRequestSpecific, ResponseSender),
    Check(Sender<Result<(), std::io::Error>>),
    ToBootstrap(Sender<Vec<String>>),
    SeedRouting(Vec<Node>, Sender<()>),
}

#[derive(Debug, Clone)]
pub enum ResponseSender {
    ClosestNodes(Sender<Box<[Node]>>),
    Peers(Sender<Vec<SocketAddrV4>>),
    Mutable(Sender<MutableItem>),
    MutableDetailed {
        values: Sender<MutableItem>,
        outcome: Sender<GetMutableOutcome>,
    },
    Immutable(Sender<Box<[u8]>>),
}

/// Builder for creating a [Testnet] with custom configuration.
///
/// # Defaults
///
/// - `bind_address`: `127.0.0.1` (localhost)
/// - `seeded`: `true` - nodes start with fully populated routing tables
///
/// # Example
///
/// ```ignore
/// use std::net::Ipv4Addr;
/// use mainline::Testnet;
///
/// // Use localhost (default)
/// let testnet = Testnet::builder(3).build().unwrap();
///
/// // Use all interfaces (0.0.0.0)
/// let testnet = Testnet::builder(3)
///     .bind_address(Ipv4Addr::UNSPECIFIED)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct TestnetBuilder {
    count: usize,
    bind_address: Ipv4Addr,
    seeded: bool,
}

impl TestnetBuilder {
    /// Create a new builder with the specified number of nodes.
    ///
    /// # Defaults
    ///
    /// - `bind_address`: `127.0.0.1` (localhost)
    /// - `seeded`: `true`
    pub fn new(count: usize) -> Self {
        Self {
            count,
            bind_address: Ipv4Addr::LOCALHOST,
            seeded: true,
        }
    }

    /// Set the address to bind all nodes to.
    ///
    /// Defaults to `127.0.0.1` (localhost).
    /// Use `Ipv4Addr::UNSPECIFIED` (`0.0.0.0`) to bind to all interfaces.
    pub fn bind_address(&mut self, bind_address: Ipv4Addr) -> &mut Self {
        self.bind_address = bind_address;
        self
    }

    /// Whether to pre-seed routing tables with all nodes.
    ///
    /// Defaults to `true`.
    ///
    /// When `true`, all nodes start with fully populated routing tables.
    /// When `false`, nodes bootstrap from each other which is faster at startup
    /// but may not have immediate full connectivity.
    pub fn seeded(&mut self, seeded: bool) -> &mut Self {
        self.seeded = seeded;
        self
    }

    /// Build the testnet.
    ///
    /// Nodes will be bound to the configured `bind_address` (default: `127.0.0.1`).
    ///
    /// This will block until all nodes are created (and seeded if `seeded` is true).
    pub fn build(&self) -> Result<Testnet, std::io::Error> {
        if self.seeded {
            Testnet::build_seeded(self.count, self.bind_address)
        } else {
            Testnet::build_unseeded(self.count, self.bind_address)
        }
    }
}

/// Create a testnet of Dht nodes to run tests against instead of the real mainline network.
///
/// # Bind Address
///
/// The convenience methods ([`Self::new`], [`Self::new_unseeded`], etc.) bind to `0.0.0.0`
/// for backwards compatibility. Use [`Self::builder`] to bind to a different address.
// TODO(breaking): In the next major version, change the default bind address from
// `0.0.0.0` to `127.0.0.1` for better cross-platform compatibility (especially macOS).
#[derive(Debug)]
pub struct Testnet {
    /// bootstrapping nodes for this testnet.
    pub bootstrap: Vec<String>,
    /// all nodes in this testnet
    pub nodes: Vec<Dht>,
}

// TODO(breaking): In the next major version, change `new()` and related methods to bind
// to `127.0.0.1` instead of `0.0.0.0` for better macOS compatibility. The builder already
// defaults to `127.0.0.1`.

impl Testnet {
    /// Returns a builder to configure and create a [Testnet].
    ///
    /// The builder defaults to binding to `127.0.0.1` (localhost).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let testnet = Testnet::builder(3).build().unwrap();
    /// ```
    pub fn builder(count: usize) -> TestnetBuilder {
        TestnetBuilder::new(count)
    }

    /// Create a new testnet with a certain size.
    ///
    /// Note: this network will be shutdown as soon as this struct
    /// gets dropped, if you want the network to be `'static`, then
    /// you should call [Self::leak].
    ///
    /// This will block until all nodes are seeded with local peers.
    /// If you are using an async runtime, consider using [Self::new_async].
    ///
    /// # Bind Address
    ///
    /// Nodes are bound to `0.0.0.0` (all interfaces). Use [`Self::builder`] to bind to
    /// a different address.
    pub fn new(count: usize) -> Result<Testnet, std::io::Error> {
        Testnet::build_seeded(count, Ipv4Addr::UNSPECIFIED)
    }

    /// Create a new testnet without pre-seeding routing tables.
    ///
    /// This is faster at startup, but nodes will not start with fully populated routing tables.
    /// Use this when your tests do not require immediate full connectivity.
    ///
    /// # Bind Address
    ///
    /// Nodes are bound to `0.0.0.0` (all interfaces). Use [`Self::builder`] with `.seeded(false)`
    /// to bind to a different address.
    pub fn new_unseeded(count: usize) -> Result<Testnet, std::io::Error> {
        Testnet::build_unseeded(count, Ipv4Addr::UNSPECIFIED)
    }

    #[cfg(feature = "async")]
    /// Similar to [Self::new], but available for async contexts.
    ///
    /// # Bind Address
    ///
    /// Nodes are bound to `0.0.0.0` (all interfaces). Use [`Self::builder`] to bind to
    /// a different address.
    pub async fn new_async(count: usize) -> Result<Testnet, std::io::Error> {
        Testnet::build_seeded(count, Ipv4Addr::UNSPECIFIED)
    }

    #[cfg(feature = "async")]
    /// Similar to [Self::new_unseeded], but available for async contexts.
    ///
    /// # Bind Address
    ///
    /// Nodes are bound to `0.0.0.0` (all interfaces). Use [`Self::builder`] with `.seeded(false)`
    /// to bind to a different address.
    pub async fn new_unseeded_async(count: usize) -> Result<Testnet, std::io::Error> {
        Testnet::build_unseeded(count, Ipv4Addr::UNSPECIFIED)
    }

    #[allow(deprecated)]
    fn build_seeded(count: usize, bind_address: Ipv4Addr) -> Result<Testnet, std::io::Error> {
        let mut nodes = Vec::with_capacity(count);

        for _ in 0..count {
            let node = Dht::builder()
                .server_mode()
                .no_bootstrap()
                .bind_address(bind_address)
                .build()?;
            nodes.push(node);
        }

        let infos: Vec<_> = nodes.iter().map(|node| node.info()).collect();
        let bootstrap = infos
            .iter()
            .map(|info| info.local_addr().to_string())
            .collect::<Vec<_>>();
        let seeded_nodes: Vec<_> = infos
            .iter()
            .map(|info| Node::new(*info.id(), info.local_addr()))
            .collect();

        for (node, info) in nodes.iter().zip(infos.iter()) {
            let peers = seeded_nodes
                .iter()
                .filter(|peer| peer.id() != info.id())
                .cloned()
                .collect::<Vec<_>>();
            let (tx, rx) = flume::bounded(1);
            node.send(ActorMessage::SeedRouting(peers, tx));
            let _ = rx.recv();
        }

        Ok(Self { bootstrap, nodes })
    }

    #[allow(deprecated)]
    fn build_unseeded(count: usize, bind_address: Ipv4Addr) -> Result<Testnet, std::io::Error> {
        let mut nodes = Vec::with_capacity(count);
        let mut bootstrap = Vec::new();

        for i in 0..count {
            if i == 0 {
                let node = Dht::builder()
                    .server_mode()
                    .no_bootstrap()
                    .bind_address(bind_address)
                    .build()?;

                let info = node.info();

                bootstrap.push(info.local_addr().to_string());

                nodes.push(node);
            } else {
                let node = Dht::builder()
                    .server_mode()
                    .bootstrap(&bootstrap)
                    .bind_address(bind_address)
                    .build()?;
                nodes.push(node);
            }
        }

        Ok(Self { bootstrap, nodes })
    }

    /// By default as soon as this testnet gets dropped,
    /// all the nodes get dropped and the entire network is shutdown.
    ///
    /// This method uses [Box::leak] to keep nodes running, which is
    /// useful if you need to keep running the testnet in the process
    /// even if this struct gets dropped.
    pub fn leak(&self) {
        for node in self.nodes.clone() {
            Box::leak(Box::new(node));
        }
    }
}

#[derive(thiserror::Error, Debug)]
/// Put MutableItem errors.
pub enum PutMutableError {
    #[error(transparent)]
    /// Common PutQuery errors
    Query(#[from] PutQueryError),

    #[error(transparent)]
    /// PutQuery for [crate::MutableItem] errors
    Concurrency(#[from] ConcurrencyError),
}

#[cfg(test)]
mod test {
    #![allow(deprecated)]

    use std::net::Ipv4Addr;
    use std::str::FromStr;

    use ed25519_dalek::SigningKey;

    use crate::rpc::ConcurrencyError;

    use super::*;

    #[test]
    fn bind_twice() {
        let a = Dht::client().unwrap();
        let result = Dht::builder()
            .port(a.info().local_addr().port())
            .server_mode()
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn announce_get_peer() {
        let testnet = Testnet::builder(10).build().unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let info_hash = Id::random();

        a.announce_peer(info_hash, Some(45555))
            .expect("failed to announce");

        let peers = b.get_peers(info_hash).next().expect("No peers");

        assert_eq!(peers.first().unwrap().port(), 45555);
    }

    #[test]
    fn put_get_immutable() {
        let testnet = Testnet::builder(10).build().unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let value = b"Hello World!";
        let expected_target = Id::from_str("e5f96f6f38320f0f33959cb4d3d656452117aadb").unwrap();

        let target = a.put_immutable(value).unwrap();
        assert_eq!(target, expected_target);

        let response = b.get_immutable(target).unwrap();

        assert_eq!(response, value.to_vec().into_boxed_slice());
    }

    #[test]
    fn find_node_no_values() {
        let client = Dht::builder().no_bootstrap().build().unwrap();

        client.find_node(Id::random());
    }

    #[test]
    fn put_get_immutable_no_values() {
        let client = Dht::builder().no_bootstrap().build().unwrap();

        assert_eq!(client.get_immutable(Id::random()), None);
    }

    #[test]
    fn put_get_mutable() {
        let testnet = Testnet::builder(10).build().unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let seq = 1000;
        let value = b"Hello World!";

        let item = MutableItem::new(signer.clone(), value, seq, None);

        a.put_mutable(item.clone(), None).unwrap();

        let response = b
            .get_mutable(signer.verifying_key().as_bytes(), None, None)
            .next()
            .expect("No mutable values");

        assert_eq!(&response, &item);
    }

    #[test]
    fn put_get_mutable_no_more_recent_value() {
        let testnet = Testnet::builder(10).build().unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let seq = 1000;
        let value = b"Hello World!";

        let item = MutableItem::new(signer.clone(), value, seq, None);

        a.put_mutable(item.clone(), None).unwrap();

        let response = b
            .get_mutable(signer.verifying_key().as_bytes(), None, Some(seq))
            .next();

        assert!(&response.is_none());
    }

    #[test]
    fn get_mutable_most_recent_prefers_highest_seq() {
        let testnet = Testnet::builder(10).build().unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let newer = MutableItem::new(signer.clone(), b"newer", 1001, None);
        client.put_mutable(newer.clone(), None).unwrap();

        let older = MutableItem::new(signer, b"older", 1000, None);
        let (sender, _) = flume::bounded::<Result<PutOutcome, PutError>>(1);
        let request = PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(older, None));
        client
            .0
            .send(ActorMessage::Put(request, sender, None))
            .unwrap();

        let most_recent = client
            .get_mutable_most_recent(newer.key(), None)
            .expect("No mutable values");

        assert_eq!(most_recent.seq(), newer.seq());
        assert_eq!(most_recent.value(), newer.value());
    }

    #[test]
    fn repeated_put_query() {
        let testnet = Testnet::builder(10).build().unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let id = a.put_immutable(&[1, 2, 3]).unwrap();

        assert_eq!(a.put_immutable(&[1, 2, 3]).unwrap(), id);
    }

    #[test]
    fn concurrent_get_mutable() {
        let testnet = Testnet::builder(10).build().unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let key = signer.verifying_key().to_bytes();
        let seq = 1000;
        let value = b"Hello World!";

        let item = MutableItem::new(signer.clone(), value, seq, None);

        a.put_mutable(item.clone(), None).unwrap();

        let _response_first = b
            .get_mutable(&key, None, None)
            .next()
            .expect("No mutable values");

        let response_second = b
            .get_mutable(&key, None, None)
            .next()
            .expect("No mutable values");

        assert_eq!(&response_second, &item);
    }

    #[test]
    fn concurrent_put_mutable_same() {
        let testnet = Testnet::builder(10).build().unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let seq = 1000;
        let value = b"Hello World!";

        let item = MutableItem::new(signer.clone(), value, seq, None);

        let mut handles = vec![];

        for _ in 0..2 {
            let client = client.clone();
            let item = item.clone();

            let handle = std::thread::spawn(move || client.put_mutable(item, None).unwrap());

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn concurrent_put_mutable_different() {
        let testnet = Testnet::builder(10).build().unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let mut handles = vec![];

        for i in 0..2 {
            let client = client.clone();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let seq = 1000;

            let mut value = b"Hello World!".to_vec();
            value.push(i);

            let item = MutableItem::new(signer.clone(), &value, seq, None);

            let handle = std::thread::spawn(move || {
                let result = client.put_mutable(item, None);
                if i == 0 {
                    assert!(result.is_ok())
                } else {
                    assert!(matches!(
                        result,
                        Err(PutMutableError::Concurrency(ConcurrencyError::ConflictRisk))
                    ))
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn concurrent_put_mutable_different_with_cas() {
        let testnet = Testnet::builder(10).build().unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        // First
        {
            let item = MutableItem::new(signer.clone(), &[], 1000, None);

            let (sender, _) = flume::bounded::<Result<PutOutcome, PutError>>(1);
            let request =
                PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(item, None));
            client
                .0
                .send(ActorMessage::Put(request, sender, None))
                .unwrap();
        }

        std::thread::sleep(Duration::from_millis(100));

        // Second
        {
            let item = MutableItem::new(signer, &[], 1001, None);

            client.put_mutable(item, Some(1000)).unwrap();
        }
    }

    #[test]
    fn conflict_302_seq_less_than_current() {
        let testnet = Testnet::builder(10).build().unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        client
            .put_mutable(MutableItem::new(signer.clone(), &[], 1001, None), None)
            .unwrap();

        assert!(matches!(
            client.put_mutable(MutableItem::new(signer, &[], 1000, None), None),
            Err(PutMutableError::Concurrency(
                ConcurrencyError::NotMostRecent
            ))
        ));
    }

    #[test]
    fn conflict_301_cas() {
        let testnet = Testnet::builder(10).build().unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        client
            .put_mutable(MutableItem::new(signer.clone(), &[], 1001, None), None)
            .unwrap();

        assert!(matches!(
            client.put_mutable(MutableItem::new(signer, &[], 1002, None), Some(1000)),
            Err(PutMutableError::Concurrency(ConcurrencyError::CasFailed))
        ));
    }

    #[test]
    fn populate_bootstrapping_node_routing_table() {
        let size = 3;

        let testnet = Testnet::builder(size).build().unwrap();

        assert!(testnet
            .nodes
            .iter()
            .all(|n| n.to_bootstrap().len() == size - 1));
    }
}
