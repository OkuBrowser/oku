//! Dht node.

use std::io;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::pin::Pin;
use std::task::{Context, Poll};

#[allow(unused_imports)]
use ed25519_dalek::SigningKey;
use futures_core::Stream;
use tokio::sync::{mpsc, oneshot};

#[allow(unused_imports)]
use crate::{
    Node, ServerSettings,
    actor::{ActorMessage, Info, ResponseSender, config::Config},
    common::{
        AnnouncePeerRequestArguments, AnnounceSignedPeerRequestArguments, FindNodeRequestArguments,
        GetPeersRequestArguments, GetValueRequestArguments, Id, MutableItem,
        PutImmutableRequestArguments, PutMutableRequestArguments, PutRequestSpecific,
        SignedAnnounce, hash_immutable,
    },
    core::{ConcurrencyError, PutError, PutQueryError, iterative_query::GetRequestSpecific},
};

mod testnet;

pub use testnet::Testnet;

/// Capacity of the actor inbox channel.
const ACTOR_INBOX_CAPACITY: usize = 256;

#[n0_error::stack_error(derive)]
#[derive(Clone, Copy, PartialEq, Eq)]
/// Error returned when the DHT actor task has shut down.
///
/// All [Dht] handles are still usable but every method will return this error
/// until the handles are dropped.
#[error("DHT actor task has shut down")]
pub struct ActorShutdown;

impl From<ActorShutdown> for io::Error {
    fn from(_: ActorShutdown) -> Self {
        io::Error::other("DHT actor task has shut down")
    }
}

#[derive(Debug, Clone)]
/// Mainline Dht node.
pub struct Dht(pub(crate) mpsc::Sender<ActorMessage>);

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
    pub fn bootstrap<T: ToString>(&mut self, bootstrap: &[T]) -> &mut Self {
        self.0.bootstrap = bootstrap.iter().map(|b| b.to_string()).collect();

        self
    }

    /// Add more bootstrap nodes to default bootstrapping nodes.
    ///
    /// Useful when you want to augment the default bootstrapping nodes with
    /// dynamic list of nodes you have seen in previous sessions.
    pub fn extra_bootstrap<T: ToString>(&mut self, extra_bootstrap: &[T]) -> &mut Self {
        for address in extra_bootstrap {
            self.0.bootstrap.push(address.to_string());
        }

        self
    }

    /// Remove the existing bootstrapping nodes, usually to create the first node in a new network.
    pub fn no_bootstrap(&mut self) -> &mut Self {
        self.0.bootstrap = vec![];

        self
    }

    /// Used to simulate a DHT that doesn't support `announce_signed_peers`
    #[cfg(test)]
    fn disable_signed_peers(&mut self) -> &mut Self {
        self.0.disable_announce_signed_peers = true;

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

    /// Create a [Dht] node.
    ///
    /// Must be called from within a Tokio runtime context, since the UDP socket is
    /// registered with the Tokio reactor during construction.
    pub fn build(&self) -> io::Result<Dht> {
        Dht::new(self.0.clone())
    }
}

impl Dht {
    /// Create a new Dht node.
    ///
    /// Binds the UDP socket synchronously, so any bind error is surfaced immediately.
    /// Must be called from within a Tokio runtime context: the socket is registered
    /// with the Tokio reactor here, and the actor loop is spawned as a background task.
    pub fn new(config: Config) -> io::Result<Self> {
        let actor = crate::actor::Actor::new(config)?;
        let (sender, receiver) = mpsc::channel(ACTOR_INBOX_CAPACITY);
        tokio::spawn(crate::actor::run(actor, receiver));
        Ok(Dht(sender))
    }

    /// Returns a builder to edit settings before creating a Dht node.
    pub fn builder() -> DhtBuilder {
        DhtBuilder::default()
    }

    /// Create a new DHT client with default bootstrap nodes.
    pub fn client() -> io::Result<Self> {
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
    pub fn server() -> io::Result<Self> {
        Dht::builder().server_mode().build()
    }

    // === Getters ===

    /// Information and statistics about this [Dht] node.
    pub async fn info(&self) -> Result<Info, ActorShutdown> {
        let (tx, rx) = oneshot::channel();
        self.send(ActorMessage::Info(tx)).await?;

        rx.await.map_err(|_| ActorShutdown)
    }

    /// Turn this node's routing table to a list of bootstrapping nodes.
    pub async fn to_bootstrap(&self) -> Result<Vec<String>, ActorShutdown> {
        let (tx, rx) = oneshot::channel();
        self.send(ActorMessage::ToBootstrap(tx)).await?;

        rx.await.map_err(|_| ActorShutdown)
    }

    // === Public Methods ===

    /// Await until the bootstrapping query is done.
    ///
    /// Returns true if the bootstrapping was successful.
    pub async fn bootstrapped(&self) -> Result<bool, ActorShutdown> {
        let info = self.info().await?;
        self.find_node(*info.id()).await?;

        let info = self.info().await?;
        Ok(info.routing_table_size() > 0)
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
    pub async fn find_node(&self, target: Id) -> Result<Box<[Node]>, ActorShutdown> {
        let (tx, rx) = oneshot::channel();
        self.send(ActorMessage::Get(
            GetRequestSpecific::FindNode(FindNodeRequestArguments { target }),
            ResponseSender::ClosestNodes(tx),
        ))
        .await?;

        rx.await.map_err(|_| ActorShutdown)
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
    pub async fn get_peers(
        &self,
        info_hash: Id,
    ) -> Result<GetStream<Vec<SocketAddrV4>>, ActorShutdown> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetPeers(GetPeersRequestArguments { info_hash }),
            ResponseSender::Peers(tx),
        ))
        .await?;

        Ok(GetStream(rx))
    }

    /// Announce a peer for a given infohash.
    ///
    /// The peer will be announced on this process IP.
    /// If explicit port is passed, it will be used, otherwise the port will be implicitly
    /// assumed by remote nodes to be the same ase port they received the request from.
    pub async fn announce_peer(
        &self,
        info_hash: Id,
        port: Option<u16>,
    ) -> Result<Id, PutQueryError> {
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
        .await
        .map_err(put_error_to_query_error)
    }

    // === Signed Peers ===

    /// Announce a signed peer for a given infohash.
    ///
    /// ## Namespacing
    /// It is important to distinguish your overlay network and any other differentiator like a
    /// sub-network or geographical distribution, by namespacing your `info_hash`, to avoid getting
    /// signed peers you can't or don't want to connect to.
    ///
    /// The easiest way for namespacing is to hash a concatenation of your original `info_hash`
    /// with the name of your network and any other filters, then pass the first 20 bytes as the
    /// `info_hash` to this method.
    ///
    /// Read [BEP_????](https://github.com/Nuhvi/mainline/blob/main/beps/bep_signed_peers.rst) for more information.
    #[cfg(feature = "unstable_signed_peers")]
    #[cfg_attr(n0_mainline_docsrs, doc(cfg(feature = "unstable_signed_peers")))]
    pub async fn announce_signed_peer(
        &self,
        info_hash: Id,
        signer: &SigningKey,
    ) -> Result<Id, PutQueryError> {
        let signed_announce = SignedAnnounce::new(signer, &info_hash);

        self.put(
            PutRequestSpecific::AnnounceSignedPeer(AnnounceSignedPeerRequestArguments {
                info_hash,
                k: *signed_announce.key(),
                t: signed_announce.timestamp(),
                sig: *signed_announce.signature(),
            }),
            None,
        )
        .await
        .map_err(put_error_to_query_error)
    }

    /// Get peers verifiably announced for a given infohash by their public key.
    ///
    /// ## Namespacing
    /// It is important to distinguish your overlay network and any other differentiator like a
    /// sub-network or geographical distribution, by namespacing your `info_hash`, to avoid getting
    /// signed peers you can't or don't want to connect to.
    ///
    /// The easiest way for namespacing is to hash a concatenation of your original `info_hash`
    /// with the name of your network and any other filters, then pass the first 20 bytes as the
    /// `info_hash` to this method.
    ///
    /// Note: each node of the network will only return a _random_ subset (usually 20)
    /// of the total peers it has for a given infohash, so if you are getting responses
    /// from 20 nodes, you can expect up to 400 peers in total, but if there are more
    /// announced peers on that infohash, you are likely to miss some, the logic here
    /// for Bittorrent is that any peer will introduce you to more peers through "peer exchange"
    /// so if you are implementing something different from Bittorrent, you might want
    /// to implement your own logic for gossipping more peers after you discover the first ones.
    ///
    /// Read [BEP_????](https://github.com/Nuhvi/mainline/blob/main/beps/bep_signed_peers.rst) for more information.
    #[cfg(feature = "unstable_signed_peers")]
    #[cfg_attr(n0_mainline_docsrs, doc(cfg(feature = "unstable_signed_peers")))]
    pub async fn get_signed_peers(
        &self,
        info_hash: Id,
    ) -> Result<GetStream<Vec<SignedAnnounce>>, ActorShutdown> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetSignedPeers(GetPeersRequestArguments { info_hash }),
            ResponseSender::SignedPeers(tx),
        ))
        .await?;

        Ok(GetStream(rx))
    }

    // === Immutable data ===

    /// Get an Immutable data by its sha1 hash.
    pub async fn get_immutable(&self, target: Id) -> Result<Option<Box<[u8]>>, ActorShutdown> {
        let (tx, rx) = oneshot::channel();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                seq: None,
                salt: None,
            }),
            ResponseSender::Immutable(Some(tx)),
        ))
        .await?;

        // Sender dropped without sending → query completed without finding a value.
        Ok(rx.await.ok())
    }

    /// Put an immutable data to the DHT.
    pub async fn put_immutable(&self, value: &[u8]) -> Result<Id, PutQueryError> {
        let target: Id = hash_immutable(value).into();

        self.put(
            PutRequestSpecific::PutImmutable(PutImmutableRequestArguments {
                target,
                v: value.into(),
            }),
            None,
        )
        .await
        .map_err(put_error_to_query_error)
    }

    // === Mutable data ===

    /// Get a mutable data by its `public_key` and optional `salt`.
    ///
    /// You can ask for items `more_recent_than` than a certain `seq`,
    /// usually one that you already have seen before, similar to `If-Modified-Since` header in HTTP.
    ///
    /// # Order
    ///
    /// The order of [MutableItem]s returned by this stream is not guaranteed to
    /// reflect their `seq` value. You should not assume that the later items are
    /// more recent than earlier ones.
    ///
    /// Consider using [Self::get_mutable_most_recent] if that is what you need.
    pub async fn get_mutable(
        &self,
        public_key: &[u8; 32],
        salt: Option<&[u8]>,
        more_recent_than: Option<i64>,
    ) -> Result<GetStream<MutableItem>, ActorShutdown> {
        let salt = salt.map(|s| s.into());
        let target = MutableItem::target_from_key(public_key, salt.as_deref());
        let (tx, rx) = mpsc::unbounded_channel();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                seq: more_recent_than,
                salt,
            }),
            ResponseSender::Mutable(tx),
        ))
        .await?;

        Ok(GetStream(rx))
    }

    /// Get the most recent [MutableItem] from the network.
    pub async fn get_mutable_most_recent(
        &self,
        public_key: &[u8; 32],
        salt: Option<&[u8]>,
    ) -> Result<Option<MutableItem>, ActorShutdown> {
        let mut most_recent: Option<MutableItem> = None;
        let mut stream = self.get_mutable(public_key, salt, None).await?;

        while let Some(item) = stream.0.recv().await {
            if let Some(mr) = &most_recent {
                if item.seq() == mr.seq && item.value() > &*mr.value {
                    most_recent = Some(item)
                }
            } else {
                most_recent = Some(item);
            }
        }

        Ok(most_recent)
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
    ///```rust,ignore
    /// use dht::{Dht, MutableItem, SigningKey, Testnet};
    ///
    /// let testnet = Testnet::new(3).await.unwrap();
    /// let dht = Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap();
    ///
    /// let secret_key = SigningKey::from_bytes(&[0; 32]);
    /// let key = secret_key.verifying_key().to_bytes();
    /// let salt = Some(b"salt".as_ref());
    ///
    /// let (item, cas) = if let Some(most_recent) = dht .get_mutable_most_recent(&key, salt).await {
    ///     // 1. Optionally Create a new value to take the most recent's value in consideration.
    ///     let mut new_value = most_recent.value().to_vec();
    ///     new_value.extend_from_slice(b" more data");
    ///
    ///     // 2. Increment the sequence number to be higher than the most recent's.
    ///     let most_recent_seq = most_recent.seq();
    ///     let new_seq = most_recent_seq + 1;
    ///
    ///     (
    ///         MutableItem::new(&secret_key, &new_value, new_seq, salt),
    ///         // 3. Use the most recent [MutableItem::seq] as a `CAS`.
    ///         Some(most_recent_seq)
    ///     )
    /// } else {
    ///     (MutableItem::new(&secret_key, b"first value", 1, salt), None)
    /// };
    ///
    /// dht.put_mutable(item, cas).await.unwrap();
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
    pub async fn put_mutable(
        &self,
        item: MutableItem,
        cas: Option<i64>,
    ) -> Result<Id, PutMutableError> {
        let request = PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(item, cas));

        self.put(request, None).await.map_err(|error| match error {
            PutError::Query(err) => PutMutableError::Query(err),
            PutError::Concurrency(err) => PutMutableError::Concurrency(err),
        })
    }

    // === Raw ===

    /// Get closet nodes to a specific target, that support [BEP_0044](https://www.bittorrent.org/beps/bep_0044.html).
    ///
    /// Useful to [Self::put] a request to nodes further from the 20 closest nodes to the
    /// [PutRequestSpecific::target]. Which itself is useful to circumvent [extreme vertical sybil attacks](https://github.com/nuhvi/mainline/blob/main/docs/censorship-resistance.md#extreme-vertical-sybil-attacks).
    pub async fn get_closest_nodes(&self, target: Id) -> Result<Box<[Node]>, ActorShutdown> {
        let (tx, rx) = oneshot::channel();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                salt: None,
                seq: None,
            }),
            ResponseSender::ClosestNodes(tx),
        ))
        .await?;

        rx.await.map_err(|_| ActorShutdown)
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
    pub async fn put(
        &self,
        request: PutRequestSpecific,
        extra_nodes: Option<Box<[Node]>>,
    ) -> Result<Id, PutError> {
        let (tx, rx) = oneshot::channel();
        self.send(ActorMessage::Put(request, tx, extra_nodes))
            .await?;

        rx.await.map_err(|_| ActorShutdown)?
    }

    // === Private Methods ===

    async fn send(&self, message: ActorMessage) -> Result<(), ActorShutdown> {
        self.0.send(message).await.map_err(|_| ActorShutdown)
    }
}

fn put_error_to_query_error(error: PutError) -> PutQueryError {
    match error {
        PutError::Query(error) => error,
        PutError::Concurrency(_) => {
            unreachable!("should not receive a concurrency error from this query type")
        }
    }
}

/// A [Stream] of incoming peers, immutable or mutable values.
pub struct GetStream<T>(mpsc::UnboundedReceiver<T>);

impl<T> Stream for GetStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        this.0.poll_recv(cx)
    }
}

#[n0_error::stack_error(derive, from_sources, std_sources)]
/// Put MutableItem errors.
pub enum PutMutableError {
    #[error(transparent)]
    /// Common PutQuery errors
    Query(PutQueryError),

    #[error(transparent)]
    /// PutQuery for [crate::MutableItem] errors
    Concurrency(ConcurrencyError),
}

impl From<ActorShutdown> for PutMutableError {
    fn from(_: ActorShutdown) -> Self {
        PutMutableError::Query(PutQueryError::Shutdown)
    }
}

impl From<ActorShutdown> for PutQueryError {
    fn from(_: ActorShutdown) -> Self {
        PutQueryError::Shutdown
    }
}

impl From<ActorShutdown> for PutError {
    fn from(_: ActorShutdown) -> Self {
        PutError::Query(PutQueryError::Shutdown)
    }
}

#[cfg(test)]
mod test {
    use std::{str::FromStr, time::Duration};

    use ed25519_dalek::SigningKey;
    use futures::StreamExt;
    #[allow(unused_imports)]
    use rand::Rng;

    use crate::core::ConcurrencyError;

    use super::*;

    #[tokio::test]
    #[ignore = "hits the real mainline DHT; run with --ignored"]
    async fn put_get_mutable_real_dht() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init();

        let dht = Dht::client().unwrap();

        let signer = SigningKey::from_bytes(&rand::random());
        let key = *signer.verifying_key().as_bytes();
        let value = b"hello from n0-mainline test";

        let item = MutableItem::new(&signer, value, 1, None);

        dht.put_mutable(item.clone(), None).await.unwrap();

        let response = dht
            .get_mutable(&key, None, None)
            .await
            .unwrap()
            .next()
            .await
            .expect("should resolve mutable item from real DHT");

        assert_eq!(response.value(), value.as_slice());
        assert_eq!(response.seq(), 1);
    }

    #[tokio::test]
    async fn bind_twice() {
        let a = Dht::client().unwrap();
        let result = Dht::builder()
            .port(a.info().await.unwrap().local_addr().port())
            .server_mode()
            .build();

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn announce_get_peer() {
        let testnet = Testnet::new(10).await.unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let info_hash = Id::random();

        a.announce_peer(info_hash, Some(45555))
            .await
            .expect("failed to announce");

        let peers = b
            .get_peers(info_hash)
            .await
            .unwrap()
            .next()
            .await
            .expect("No peers");

        assert_eq!(peers.first().unwrap().port(), 45555);
    }

    #[tokio::test]
    async fn put_get_immutable() {
        let testnet = Testnet::new(10).await.unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let value = b"Hello World!";
        let expected_target = Id::from_str("e5f96f6f38320f0f33959cb4d3d656452117aadb").unwrap();

        let target = a.put_immutable(value).await.unwrap();
        assert_eq!(target, expected_target);

        let response = b.get_immutable(target).await.unwrap().unwrap();

        assert_eq!(response, value.to_vec().into_boxed_slice());
    }

    #[tokio::test]
    async fn find_node_no_values() {
        let client = Dht::builder().no_bootstrap().build().unwrap();

        client.find_node(Id::random()).await.unwrap();
    }

    #[tokio::test]
    async fn put_get_immutable_no_values() {
        let client = Dht::builder().no_bootstrap().build().unwrap();

        assert_eq!(client.get_immutable(Id::random()).await.unwrap(), None);
    }

    #[tokio::test]
    async fn put_get_mutable() {
        let testnet = Testnet::new(10).await.unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let seq = 1000;
        let value = b"Hello World!";

        let item = MutableItem::new(&signer, value, seq, None);

        a.put_mutable(item.clone(), None).await.unwrap();

        let response = b
            .get_mutable(signer.verifying_key().as_bytes(), None, None)
            .await
            .unwrap()
            .next()
            .await
            .expect("No mutable values");

        assert_eq!(&response, &item);
    }

    #[tokio::test]
    async fn put_get_mutable_no_more_recent_value() {
        let testnet = Testnet::new(10).await.unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let seq = 1000;
        let value = b"Hello World!";

        let item = MutableItem::new(&signer, value, seq, None);

        a.put_mutable(item.clone(), None).await.unwrap();

        let response = b
            .get_mutable(signer.verifying_key().as_bytes(), None, Some(seq))
            .await
            .unwrap()
            .next()
            .await;

        assert!(&response.is_none());
    }

    #[tokio::test]
    async fn repeated_put_query() {
        let testnet = Testnet::new(10).await.unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let id = a.put_immutable(&[1, 2, 3]).await.unwrap();

        assert_eq!(a.put_immutable(&[1, 2, 3]).await.unwrap(), id);
    }

    #[tokio::test]
    async fn concurrent_get_mutable() {
        let testnet = Testnet::new(10).await.unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let key = *signer.verifying_key().as_bytes();
        let seq = 1000;
        let value = b"Hello World!";

        let item = MutableItem::new(&signer, value, seq, None);

        a.put_mutable(item.clone(), None).await.unwrap();

        let _response_first = b
            .get_mutable(&key, None, None)
            .await
            .unwrap()
            .next()
            .await
            .expect("No mutable values");

        let response_second = b
            .get_mutable(&key, None, None)
            .await
            .unwrap()
            .next()
            .await
            .expect("No mutable values");

        assert_eq!(&response_second, &item);
    }

    #[tokio::test]
    async fn concurrent_put_mutable_different_with_cas() {
        let testnet = Testnet::new(10).await.unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        // First
        {
            let item = MutableItem::new(&signer, &[], 1000, None);

            let (tx, _rx) = oneshot::channel();
            let request =
                PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(item, None));
            client
                .0
                .send(ActorMessage::Put(request, tx, None))
                .await
                .unwrap();
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Second
        {
            let item = MutableItem::new(&signer, &[], 1001, None);

            let most_recent = client
                .get_mutable_most_recent(item.key(), None)
                .await
                .unwrap();

            if let Some(cas) = most_recent.map(|item| item.seq()) {
                client.put_mutable(item, Some(cas)).await.unwrap();
            } else {
                client.put_mutable(item, None).await.unwrap();
            }
        }
    }

    #[tokio::test]
    async fn conflict_302_seq_less_than_current() {
        let testnet = Testnet::new(10).await.unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        client
            .put_mutable(MutableItem::new(&signer, &[], 1001, None), None)
            .await
            .unwrap();

        assert!(matches!(
            client
                .put_mutable(MutableItem::new(&signer, &[], 1000, None), None)
                .await,
            Err(PutMutableError::Concurrency(
                ConcurrencyError::NotMostRecent
            ))
        ));
    }

    #[tokio::test]
    async fn conflict_301_cas() {
        let testnet = Testnet::new(10).await.unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        client
            .put_mutable(MutableItem::new(&signer, &[], 1001, None), None)
            .await
            .unwrap();

        assert!(matches!(
            client
                .put_mutable(MutableItem::new(&signer, &[], 1002, None), Some(1000))
                .await,
            Err(PutMutableError::Concurrency(ConcurrencyError::CasFailed))
        ));
    }

    #[tokio::test]
    async fn populate_bootstrapping_node_routing_table() {
        let size = 3;

        let testnet = Testnet::new(size).await.unwrap();

        for n in &testnet.nodes {
            assert_eq!(n.to_bootstrap().await.unwrap().len(), size - 1);
        }
    }

    #[tokio::test]
    async fn bootstrap_with_one_node() {
        let testnet = Testnet::new(1).await.unwrap();

        let client = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        assert!(client.bootstrapped().await.unwrap());
    }

    #[cfg(feature = "unstable_signed_peers")]
    #[tokio::test]
    async fn announce_signed_peers_at_full_adoption() {
        let testnet = Testnet::new(10).await.unwrap();

        let a = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();
        let b = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .build()
            .unwrap();

        let info_hash = Id::random();

        let signers = [0, 1, 2]
            .iter()
            .map(|_| {
                let mut secret_key = [0; 32];
                rand::rng().fill_bytes(&mut secret_key);
                SigningKey::from_bytes(&secret_key)
            })
            .collect::<Vec<_>>();

        let mut expected_keys = signers
            .iter()
            .map(|s| s.verifying_key().as_bytes().to_vec())
            .collect::<Vec<_>>();
        expected_keys.sort();

        for signer in signers {
            a.announce_signed_peer(info_hash, &signer)
                .await
                .expect("failed to announce");
        }

        let peers = b
            .get_signed_peers(info_hash)
            .await
            .unwrap()
            .next()
            .await
            .expect("No peers");

        let mut keys = peers.iter().map(|a| a.key().to_vec()).collect::<Vec<_>>();
        keys.sort();

        assert_eq!(keys, expected_keys);
    }

    #[cfg(feature = "unstable_signed_peers")]
    #[tokio::test]
    async fn announce_signed_peers_at_low_adoption() {
        let testnet_legacy = Testnet::new_without_signed_peers(10).await.unwrap();

        let signers = [0, 1, 2]
            .iter()
            .map(|_| {
                let mut secret_key = [0; 32];
                rand::rng().fill_bytes(&mut secret_key);
                SigningKey::from_bytes(&secret_key)
            })
            .collect::<Vec<_>>();

        let mut expected_keys = signers
            .iter()
            .map(|s| s.verifying_key().as_bytes().to_vec())
            .collect::<Vec<_>>();
        expected_keys.sort();

        let info_hash = Id::random();

        // confirm that our code disables `signed_announce_peers` for older versions
        {
            let a = Dht::builder()
                .bootstrap(&testnet_legacy.bootstrap)
                .disable_signed_peers()
                .build()
                .unwrap();
            assert!(
                a.announce_signed_peer(info_hash, &signers[0])
                    .await
                    .is_err()
            );
            assert_eq!(
                a.get_signed_peers(info_hash).await.unwrap().next().await,
                None
            )
        }

        {
            let testnet_new = Testnet::new_with_bootstrap(3, &testnet_legacy.bootstrap)
                .await
                .unwrap();

            let bootstrap = testnet_new.bootstrap;

            let a = Dht::builder().bootstrap(&bootstrap).build().unwrap();
            let b = Dht::builder().bootstrap(&bootstrap).build().unwrap();

            for signer in &signers {
                a.announce_signed_peer(info_hash, signer)
                    .await
                    .expect("failed to announce");
            }

            let peers = b
                .get_signed_peers(info_hash)
                .await
                .unwrap()
                .next()
                .await
                .expect("No peers");

            let mut keys = peers.iter().map(|a| a.key().to_vec()).collect::<Vec<_>>();
            keys.sort();

            assert_eq!(keys, expected_keys);
        }
    }
}
