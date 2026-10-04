//! IPFS node implementation
//!
//! [Ipfs](https://ipfs.io) is a peer-to-peer system with content addressed functionality. The main
//! entry point for users of this crate is the [`Ipfs`] facade, which allows access to most of the
//! implemented functionality.
//!
//! This crate passes a lot of the [interface-ipfs-core] test suite; most of that functionality is
//! in `ipfs-http` crate. The crate has some interoperability with the [go-ipfs] and [js-ipfs]
//! implementations.
//!
//! `ipfs` is an early alpha level crate: APIs and their implementation are subject to change in
//! any upcoming release at least for now. The aim of the crate is to become a library-first
//! production ready implementation of an Ipfs node.
//!
//! [interface-ipfs-core]: https://www.npmjs.com/package/interface-ipfs-core
//! [go-ipfs]: https://github.com/ipfs/go-ipfs/
//! [js-ipfs]: https://github.com/ipfs/js-ipfs/
// We are not done yet, but uncommenting this makes it easier to hunt down for missing docs.
//#![deny(missing_docs)]
//
// This isn't recognized in stable yet, but we should disregard any nags on these to keep making
// the docs better.
//#![allow(private_intra_doc_links)]

#[macro_use]
extern crate tracing;
pub mod block;
pub mod builder;
pub mod config;
mod context;
pub mod dag;
pub mod error;
#[cfg(feature = "gateway")]
mod gateway;
pub mod ipns;
pub mod mfs;
pub mod p2p;
pub mod path;
#[cfg(feature = "pinning")]
pub mod pinning;
pub mod prelude;
pub mod refs;
pub mod repo;
#[cfg(feature = "routing")]
mod routing;
pub mod unixfs;

pub use block::Block;

use anyhow::anyhow;
use bytes::Bytes;
use dag::{DagGet, DagPut};
use futures::{
    StreamExt,
    channel::oneshot::{self, Sender as OneshotSender, channel as oneshot_channel},
    future::BoxFuture,
    stream::BoxStream,
};

use p2p::{MultiaddrExt, PeerInfo};
use repo::{DefaultStorage, RepoFetch, RepoInsertPin, RepoRemovePin};

use tracing::Span;
use tracing_futures::Instrument;

use unixfs::UnixfsGet;
use unixfs::{AddOpt, IpfsUnixfs, UnixfsAdd, UnixfsCat, UnixfsLs};

use self::{dag::IpldDag, ipns::Ipns, p2p::TSwarm, repo::Repo};
pub use self::{
    error::Error,
    p2p::BehaviourEvent,
    p2p::KadResult,
    path::IpfsPath,
    repo::{PinKind, PinMode},
};
use async_rt::AbortableJoinHandle;
use connexa::handle::Connexa;
pub use connexa::prelude::dht::{Mode, Quorum, Record, RecordKey, ToRecordKey};
pub use connexa::prelude::request_response::{
    InboundRequestId, IntoRequest, OptionalStreamProtocol,
};
pub use connexa::prelude::swarm::derive_prelude::{ConnectionId, ListenerId};
pub use connexa::prelude::swarm::dial_opts::{DialOpts, PeerCondition};
pub use connexa::prelude::{
    ConnexaSwarmEvent, Multiaddr, PeerId, Protocol, StreamProtocol, identity::Keypair,
};
pub use connexa::prelude::{
    GossipsubMessage, Stream,
    connection_limits::ConnectionLimits,
    gossipsub, identify, ping,
    swarm::{self, NetworkBehaviour},
};
pub use connexa::{behaviour::request_response::RequestResponseConfig, dummy};
use ipld_core::cid::Cid;
use ipld_core::ipld::Ipld;

use connexa::keystore::Keychain;
use connexa::prelude::gossipsub::IntoGossipsubTopic;
use connexa::prelude::rendezvous::IntoNamespace;
#[cfg(feature = "stream")]
use connexa::prelude::stream::IntoStreamProtocol;
pub use connexa::prelude::transport::ConnectedPoint;
use serde::Serialize;
use std::{borrow::Borrow, path::PathBuf};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::Path,
    sync::Arc,
    time::Duration,
};

/// Ipfs node options used to configure the node to be created with [`UninitializedIpfs`].
struct IpfsOptions {
    /// The path of the ipfs repo (blockstore and datastore).
    ///
    /// This is always required but can be any path with in-memory backends. The filesystem backend
    /// creates a directory structure alike but not compatible to other ipfs implementations.
    ///
    /// # Incompatiblity and interop warning
    ///
    /// It is **not** recommended to set this to IPFS_PATH without first at least backing up your
    /// existing repository.
    pub ipfs_path: Option<PathBuf>,

    /// Enables and supply a name of the namespace used for indexeddb
    #[cfg(target_arch = "wasm32")]
    pub namespace: Option<Option<String>>,

    /// Nodes used as bootstrap peers.
    pub bootstrap: Vec<Multiaddr>,

    /// Bound listening addresses; by default the node will not listen on any address.
    pub listening_addrs: Vec<Multiaddr>,

    pub bitswap_config: Box<dyn Fn(p2p::bitswap::Config) -> p2p::bitswap::Config>,

    /// Address book configuration
    pub addr_config: AddressBookConfig,

    /// Repo Provider option
    pub provider: RepoProvider,

    /// Trustless HTTP gateways used as a retrieval fallback.
    #[cfg(feature = "gateway")]
    pub gateway: Option<Vec<String>>,

    /// Delegated routing V1 HTTP endpoints used for provider discovery.
    #[cfg(feature = "routing")]
    pub router: Option<Vec<String>>,

    /// The span for tracing purposes, `None` value is converted to `tracing::trace_span!("ipfs")`.
    ///
    /// All futures returned by `Ipfs`, background task actions and swarm actions are instrumented
    /// with this span or spans referring to this as their parent. Setting this other than `None`
    /// default is useful when running multiple nodes.
    pub span: Option<Span>,

    pub(crate) protocols: Libp2pProtocol,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct Libp2pProtocol {
    pub(crate) bitswap: bool,
    pub(crate) relay: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum RepoProvider {
    /// Dont provide any blocks automatically
    #[default]
    None,

    /// Provide all blocks stored automatically
    All,

    /// Provide pinned blocks
    Pinned,

    /// Provide root blocks only (Currently NO-OP)
    Roots,
}

impl Default for IpfsOptions {
    fn default() -> Self {
        Self {
            ipfs_path: None,
            #[cfg(target_arch = "wasm32")]
            namespace: None,
            bootstrap: Default::default(),
            bitswap_config: Box::new(|config| config),
            addr_config: Default::default(),
            provider: Default::default(),
            #[cfg(feature = "gateway")]
            gateway: None,
            #[cfg(feature = "routing")]
            router: None,
            listening_addrs: vec![],
            span: None,
            protocols: Default::default(),
        }
    }
}

impl fmt::Debug for IpfsOptions {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        // needed since libp2p::identity::Keypair does not have a Debug impl, and the IpfsOptions
        // is a struct with all public fields, don't enforce users to use this wrapper.
        fmt.debug_struct("IpfsOptions")
            .field("ipfs_path", &self.ipfs_path)
            .field("bootstrap", &self.bootstrap)
            .field("listening_addrs", &self.listening_addrs)
            .field("span", &self.span)
            .finish()
    }
}

/// The facade for the Ipfs node.
///
/// The facade has most of the functionality either directly as a method or the functionality can
/// be implemented using the provided methods. For more information, see examples or the HTTP
/// endpoint implementations in `ipfs-http`.
///
/// The facade is created through [`UninitializedIpfs`] which is configured with [`IpfsOptions`].
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct Ipfs {
    span: Span,
    repo: Repo<DefaultStorage>,
    connexa: Connexa<IpfsEvent, DefaultKeystore>,
    record_key_validator:
        Arc<HashMap<String, Box<dyn Fn(&str) -> anyhow::Result<RecordKey> + Sync + Send>>>,
    _gc_guard: AbortableJoinHandle<()>,
    _discovery_guard: AbortableJoinHandle<()>,
    #[cfg(feature = "gateway")]
    gateways: Option<gateway::GatewayList>,
    #[cfg(feature = "gateway")]
    _gateway_guard: AbortableJoinHandle<()>,
    #[cfg(feature = "routing")]
    routers: Option<routing::RouterList>,
    #[cfg(feature = "routing")]
    _routing_guard: AbortableJoinHandle<()>,
}

impl std::fmt::Debug for Ipfs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ipfs").finish()
    }
}

type Channel<T> = OneshotSender<Result<T, Error>>;
type ReceiverChannel<T> = oneshot::Receiver<Result<T, Error>>;
/// Events used internally to communicate with the swarm, which is executed in the the background
/// task.
#[derive(Debug)]
#[allow(clippy::type_complexity)]
enum IpfsEvent {
    /// Node supported protocol
    Protocol(OneshotSender<Vec<String>>),
    GetBitswapPeers(Channel<BoxFuture<'static, Vec<PeerId>>>),
    BitswapStats(Channel<BoxFuture<'static, crate::p2p::bitswap::BitswapStats>>),
    WantList(Option<PeerId>, Channel<BoxFuture<'static, Vec<Cid>>>),

    FindPeerIdentity(PeerId, Channel<ReceiverChannel<identify::Info>>),
    AddPeer(AddPeerOpt, Channel<()>),
    Addresses(Channel<Vec<(PeerId, Vec<Multiaddr>)>>),
    RemovePeer(PeerId, Option<Multiaddr>, Channel<bool>),
    GetBootstrappers(OneshotSender<Vec<Multiaddr>>),
    AddBootstrapper(Multiaddr, Channel<Multiaddr>),
    RemoveBootstrapper(Multiaddr, Channel<Multiaddr>),
    ClearBootstrappers(Channel<Vec<Multiaddr>>),
    DefaultBootstrap(Channel<Vec<Multiaddr>>),
}

#[derive(Debug, Copy, Clone)]
pub enum DhtMode {
    Auto,
    Client,
    Server,
}

impl From<DhtMode> for Option<Mode> {
    fn from(mode: DhtMode) -> Self {
        match mode {
            DhtMode::Auto => None,
            DhtMode::Client => Some(Mode::Client),
            DhtMode::Server => Some(Mode::Server),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PubsubEvent {
    /// Subscription event to a given topic
    Subscribe {
        peer_id: PeerId,
        topic: Option<String>,
    },

    /// Unsubscribing event to a given topic
    Unsubscribe {
        peer_id: PeerId,
        topic: Option<String>,
    },
}

type TSwarmEvent<C> = <TSwarm<C> as futures::Stream>::Item;
type TSwarmEventFn<C> = Arc<dyn Fn(&mut TSwarm<C>, &TSwarmEvent<C>) + Sync + Send>;

#[derive(Debug, Copy, Clone)]
pub enum FDLimit {
    Max,
    Custom(u64),
}

#[derive(Debug, Clone)]
pub enum PeerConnectionEvents {
    IncomingConnection {
        connection_id: ConnectionId,
        addr: Multiaddr,
    },
    OutgoingConnection {
        connection_id: ConnectionId,
        addr: Multiaddr,
    },
    ClosedConnection {
        connection_id: ConnectionId,
    },
}

impl Ipfs {
    /// Return an [`IpldDag`] for DAG operations
    pub fn dag(&self) -> IpldDag {
        IpldDag::new(self.clone())
    }

    /// Return an [`Repo`] to access the internal repo of the node
    pub fn repo(&self) -> &Repo<DefaultStorage> {
        &self.repo
    }

    /// Returns an [`IpfsUnixfs`] for files operations
    pub fn unixfs(&self) -> IpfsUnixfs {
        IpfsUnixfs::new(self.clone())
    }

    /// Returns the node's mutable filesystem ([`crate::mfs::Mfs`]) handle.
    pub fn mfs(&self) -> crate::mfs::Mfs {
        crate::mfs::Mfs::new(self.repo.clone())
    }

    /// Returns a [`Ipns`] for ipns operations
    pub fn ipns(&self) -> Ipns {
        Ipns::new(self.clone())
    }

    /// Returns a client for a remote pinning service at `endpoint` authenticated with `token`.
    #[cfg(feature = "pinning")]
    pub fn remote_pinning(
        &self,
        endpoint: impl Into<String>,
        token: impl Into<String>,
    ) -> crate::pinning::RemotePinningService {
        crate::pinning::RemotePinningService::new(self.clone(), endpoint, token)
    }

    /// Adds a trustless gateway to the retrieval list.
    #[cfg(feature = "gateway")]
    pub fn add_gateway(&self, url: &str) -> bool {
        self.gateways.as_ref().is_some_and(|g| g.add(url))
    }

    /// Removes a trustless gateway from the retrieval list.
    #[cfg(feature = "gateway")]
    pub fn remove_gateway(&self, url: &str) -> bool {
        self.gateways.as_ref().is_some_and(|g| g.remove(url))
    }

    /// Lists the trustless gateways currently used for retrieval.
    #[cfg(feature = "gateway")]
    pub fn list_gateways(&self) -> Vec<String> {
        self.gateways.as_ref().map(|g| g.list()).unwrap_or_default()
    }

    /// Adds a delegated routing endpoint used for provider discovery.
    #[cfg(feature = "routing")]
    pub fn add_router(&self, url: &str) -> bool {
        self.routers.as_ref().is_some_and(|r| r.add(url))
    }

    /// Removes a delegated routing endpoint.
    #[cfg(feature = "routing")]
    pub fn remove_router(&self, url: &str) -> bool {
        self.routers.as_ref().is_some_and(|r| r.remove(url))
    }

    /// Lists the delegated routing endpoints currently used for provider discovery.
    #[cfg(feature = "routing")]
    pub fn list_routers(&self) -> Vec<String> {
        self.routers.as_ref().map(|r| r.list()).unwrap_or_default()
    }

    /// Puts a block into the ipfs repo.
    pub fn put_block(&self, block: &Block) -> RepoPutBlock<DefaultStorage> {
        self.repo.put_block(block).span(self.span.clone())
    }

    /// Retrieves a block from the local blockstore, or starts fetching from the network or join an
    /// already started fetch.
    pub fn get_block(&self, cid: impl Borrow<Cid>) -> RepoGetBlock<DefaultStorage> {
        self.repo.get_block(cid).span(self.span.clone())
    }

    /// Remove block from the ipfs repo. A pinned block cannot be removed.
    pub async fn remove_block(
        &self,
        cid: impl Borrow<Cid>,
        recursive: bool,
    ) -> Result<Vec<Cid>, Error> {
        self.repo
            .remove_block(cid, recursive)
            .instrument(self.span.clone())
            .await
    }

    /// Cleans up of all unpinned blocks
    /// Note: This will prevent writing operations in [`Repo`] until it finish clearing unpinned
    ///       blocks.
    pub async fn gc(&self) -> Result<Vec<Cid>, Error> {
        let _g = self.repo.inner.gclock.write().await;
        self.repo.cleanup().instrument(self.span.clone()).await
    }

    /// Pins a given Cid recursively or directly (non-recursively).
    ///
    /// Pins on a block are additive in sense that a previously directly (non-recursively) pinned
    /// can be made recursive, but removing the recursive pin on the block removes also the direct
    /// pin as well.
    ///
    /// Pinning a Cid recursively (for supported dag-protobuf and dag-cbor) will walk its
    /// references and pin the references indirectly. When a Cid is pinned indirectly it will keep
    /// its previous direct or recursive pin and be indirect in addition.
    ///
    /// Recursively pinned Cids cannot be re-pinned non-recursively but non-recursively pinned Cids
    /// can be "upgraded to" being recursively pinned.
    ///
    /// # Crash unsafety
    ///
    /// If a recursive `insert_pin` operation is interrupted because of a crash or the crash
    /// prevents from synchronizing the data store to disk, this will leave the system in an inconsistent
    /// state. The remedy is to re-pin recursive pins.
    pub fn insert_pin(&self, cid: impl Borrow<Cid>) -> RepoInsertPin<DefaultStorage> {
        self.repo().pin(cid).span(self.span.clone())
    }

    /// Unpins a given Cid recursively or only directly.
    ///
    /// Recursively unpinning a previously only directly pinned Cid will remove the direct pin.
    ///
    /// Unpinning an indirectly pinned Cid is not possible other than through its recursively
    /// pinned tree roots.
    pub fn remove_pin(&self, cid: impl Borrow<Cid>) -> RepoRemovePin<DefaultStorage> {
        self.repo().remove_pin(cid).span(self.span.clone())
    }

    /// Checks whether a given block is pinned.
    ///
    /// Returns true if the block is pinned, false if not. See Crash unsafety notes for the false
    /// response.
    ///
    /// # Crash unsafety
    ///
    /// Cannot currently detect partially written recursive pins. Those can happen if
    /// [`Ipfs::insert_pin`] is interrupted by a crash for example.
    ///
    /// Works correctly only under no-crash situations. Workaround for hitting a crash is to re-pin
    /// any existing recursive pins.
    ///
    pub async fn is_pinned(&self, cid: impl Borrow<Cid>) -> Result<bool, Error> {
        let span = debug_span!(parent: &self.span, "is_pinned", cid = %cid.borrow());
        self.repo.is_pinned(cid).instrument(span).await
    }

    /// Lists all pins, or the specific kind thereof.
    ///
    /// # Crash unsafety
    ///
    /// Does not currently recover from partial recursive pin insertions.
    pub async fn list_pins(
        &self,
        filter: Option<PinMode>,
    ) -> BoxStream<'static, Result<(Cid, PinMode), Error>> {
        let span = debug_span!(parent: &self.span, "list_pins", ?filter);
        self.repo.list_pins(filter).instrument(span).await
    }

    /// Read specific pins. When `requirement` is `Some`, all pins are required to be of the given
    /// [`PinMode`].
    ///
    /// # Crash unsafety
    ///
    /// Does not currently recover from partial recursive pin insertions.
    pub async fn query_pins(
        &self,
        cids: Vec<Cid>,
        requirement: Option<PinMode>,
    ) -> Result<Vec<(Cid, PinKind<Cid>)>, Error> {
        let span = debug_span!(parent: &self.span, "query_pins", ids = cids.len(), ?requirement);
        self.repo
            .query_pins(cids, requirement)
            .instrument(span)
            .await
    }

    /// Puts an ipld node into the ipfs repo using `dag-cbor` codec and Sha2_256 hash.
    ///
    /// Returns Cid version 1 for the document
    pub fn put_dag(&self, ipld: impl Serialize) -> DagPut {
        self.dag().put_dag(ipld).span(self.span.clone())
    }

    /// Gets an ipld node from the ipfs, fetching the block if necessary.
    ///
    /// See [`IpldDag::get`] for more information.
    pub fn get_dag(&self, path: impl Into<IpfsPath>) -> DagGet {
        self.dag().get_dag(path).span(self.span.clone())
    }

    /// Creates a stream which will yield the bytes of an UnixFS file from the root Cid, with the
    /// optional file byte range. If the range is specified and is outside of the file, the stream
    /// will end without producing any bytes.
    pub fn cat_unixfs(&self, starting_point: impl Into<unixfs::StartingPoint>) -> UnixfsCat {
        self.unixfs().cat(starting_point).span(self.span.clone())
    }

    /// Add a file through a stream of data to the blockstore
    pub fn add_unixfs(&self, opt: impl Into<AddOpt>) -> UnixfsAdd {
        self.unixfs().add(opt).span(self.span.clone())
    }

    /// Retreive a file and saving it to a path.
    pub fn get_unixfs(&self, path: impl Into<IpfsPath>, dest: impl AsRef<Path>) -> UnixfsGet {
        self.unixfs().get(path, dest).span(self.span.clone())
    }

    /// List directory contents
    pub fn ls_unixfs(&self, path: impl Into<IpfsPath>) -> UnixfsLs {
        self.unixfs().ls(path).span(self.span.clone())
    }

    /// Resolves a ipns path to an ipld path; currently only supports dht and dnslink resolution.
    pub async fn resolve_ipns(
        &self,
        path: impl Borrow<IpfsPath>,
        recursive: bool,
    ) -> Result<IpfsPath, Error> {
        async move {
            let ipns = self.ipns();
            let mut resolved = ipns.resolve(path).await;

            if recursive {
                let mut seen = HashSet::with_capacity(1);
                while let Ok(ref res) = resolved {
                    if !seen.insert(res.clone()) {
                        break;
                    }
                    resolved = ipns.resolve(res).await;
                }
            }
            Ok(resolved?)
        }
        .instrument(self.span.clone())
        .await
    }

    /// Publish ipns record to DHT
    pub async fn publish_ipns(&self, path: impl Borrow<IpfsPath>) -> Result<IpfsPath, Error> {
        async move {
            let ipns = self.ipns();
            ipns.publish(None, path, Default::default())
                .await
                .map_err(anyhow::Error::from)
        }
        .instrument(self.span.clone())
        .await
    }

    /// Connects to the peer
    pub async fn connect(&self, target: impl Into<DialOpts>) -> Result<ConnectionId, Error> {
        self.connexa
            .swarm()
            .dial(target)
            .await
            .map_err(anyhow::Error::from)
    }

    /// Returns known peer addresses
    pub async fn addrs(&self) -> Result<Vec<(PeerId, Vec<Multiaddr>)>, Error> {
        let (tx, rx) = oneshot_channel();
        self.connexa
            .send_custom_event(IpfsEvent::Addresses(tx))
            .await?;
        rx.await?
    }

    /// Checks whether there is an established connection to a peer.
    pub async fn is_connected(&self, peer_id: PeerId) -> Result<bool, Error> {
        self.connexa
            .swarm()
            .is_connected(peer_id)
            .await
            .map_err(anyhow::Error::from)
    }

    /// Returns the connected peers
    pub async fn connected(&self) -> Result<Vec<PeerId>, Error> {
        self.connexa
            .swarm()
            .connected_peers()
            .await
            .map_err(anyhow::Error::from)
    }

    /// Disconnects a given peer.
    pub async fn disconnect(&self, target: PeerId) -> Result<(), Error> {
        self.connexa
            .swarm()
            .disconnect(target)
            .await
            .map_err(anyhow::Error::from)
    }

    /// Bans a peer.
    pub async fn ban_peer(&self, target: PeerId) -> Result<(), Error> {
        self.connexa
            .blacklist()
            .add(target)
            .await
            .map_err(anyhow::Error::from)
    }

    /// Unbans a peer.
    pub async fn unban_peer(&self, target: PeerId) -> Result<(), Error> {
        self.connexa
            .blacklist()
            .remove(target)
            .await
            .map_err(Into::into)
    }

    /// Returns the peer identity information. If no peer id is supplied the local node identity is used.
    pub async fn identity(&self, peer_id: Option<PeerId>) -> Result<PeerInfo, Error> {
        async move {
            match peer_id {
                Some(peer_id) => {
                    let (tx, rx) = oneshot_channel();

                    self.connexa
                        .send_custom_event(IpfsEvent::FindPeerIdentity(peer_id, tx))
                        .await?;

                    rx.await??.await?.map(PeerInfo::from)
                }
                None => {
                    let mut addresses = HashSet::new();

                    let (local_result, external_result) =
                        futures::join!(self.listening_addresses(), self.external_addresses());

                    let external: HashSet<Multiaddr> =
                        HashSet::from_iter(external_result.unwrap_or_default());
                    let local: HashSet<Multiaddr> =
                        HashSet::from_iter(local_result.unwrap_or_default());

                    addresses.extend(external.iter().cloned());
                    addresses.extend(local.iter().cloned());

                    let mut addresses = Vec::from_iter(addresses);

                    let (tx, rx) = oneshot_channel();
                    self.connexa
                        .send_custom_event(IpfsEvent::Protocol(tx))
                        .await?;

                    let protocols = rx
                        .await?
                        .iter()
                        .filter_map(|s| StreamProtocol::try_from_owned(s.clone()).ok())
                        .collect();

                    let public_key = self.keypair().public();
                    let peer_id = public_key.to_peer_id();

                    for addr in &mut addresses {
                        if !matches!(addr.iter().last(), Some(Protocol::P2p(_))) {
                            addr.push(Protocol::P2p(peer_id))
                        }
                    }

                    let info = PeerInfo {
                        peer_id,
                        public_key,
                        protocol_version: String::new(), // TODO
                        agent_version: String::new(),    // TODO
                        listen_addrs: addresses,
                        protocols,
                        observed_addr: None,
                    };

                    Ok(info)
                }
            }
        }
        .instrument(self.span.clone())
        .await
    }

    /// Subscribes to a given topic. Can unsubscribe by calling [`Ipfs::pubsub_unsubscribe`].
    pub async fn pubsub_subscribe(&self, topic: impl IntoGossipsubTopic) -> Result<(), Error> {
        self.connexa
            .gossipsub()
            .subscribe(topic)
            .await
            .map_err(anyhow::Error::from)
    }

    /// Creates a stream to listen on events of a given topic
    pub async fn pubsub_listener(
        &self,
        topic: impl IntoGossipsubTopic,
    ) -> Result<BoxStream<'static, connexa::prelude::GossipsubEvent>, Error> {
        let st = self
            .connexa
            .gossipsub()
            .listener(topic)
            .await
            .map_err(anyhow::Error::from)?;

        Ok(st)
    }

    /// Publishes to the topic which may have been subscribed to earlier
    pub async fn pubsub_publish(
        &self,
        topic: impl IntoGossipsubTopic,
        data: impl Into<Bytes>,
    ) -> Result<(), Error> {
        self.connexa
            .gossipsub()
            .publish(topic, data)
            .await
            .map_err(Into::into)
    }

    /// Forcibly unsubscribes a previously made [`SubscriptionStream`], which could also be
    /// unsubscribed by dropping the stream.
    ///
    /// Returns true if unsubscription was successful
    pub async fn pubsub_unsubscribe(&self, topic: impl IntoGossipsubTopic) -> Result<(), Error> {
        self.connexa
            .gossipsub()
            .unsubscribe(topic)
            .await
            .map_err(Into::into)
    }

    /// Returns all known pubsub peers within a given topic
    pub async fn pubsub_peers(&self, topic: impl IntoGossipsubTopic) -> Result<Vec<PeerId>, Error> {
        self.connexa
            .gossipsub()
            .peers(topic)
            .await
            .map_err(Into::into)
    }

    /// Returns all currently subscribed topics
    pub async fn pubsub_subscribed(&self) -> Result<Vec<String>, Error> {
        // self.connexa.gossipsub().
        unimplemented!()
    }

    /// Subscribe to a stream of request. If a protocol is not supplied,
    /// it will subscribe to the first or default protocol that was set in
    /// [UninitializedIpfs::with_request_response]
    pub async fn requests_subscribe(
        &self,
        protocol: impl Into<OptionalStreamProtocol>,
    ) -> Result<BoxStream<'static, (PeerId, InboundRequestId, Bytes)>, Error> {
        self.connexa
            .request_response()
            .listen_for_requests(protocol)
            .await
            .map_err(Into::into)
    }

    /// Sends a request to a specific peer.
    /// If a protocol is not supplied, it will use the first/default protocol that was set in
    /// [UninitializedIpfs::with_request_response].
    pub async fn send_request(
        &self,
        peer_id: PeerId,
        request: impl IntoRequest,
    ) -> Result<Bytes, Error> {
        self.connexa
            .request_response()
            .send_request(peer_id, request)
            .await
            .map_err(Into::into)
    }

    /// Sends a request to a list of peers.
    /// If a protocol is not supplied, it will use the first/default protocol that was set in
    /// [UninitializedIpfs::with_request_response]
    pub async fn send_requests(
        &self,
        peers: impl IntoIterator<Item = PeerId>,
        request: impl IntoRequest,
    ) -> Result<BoxStream<'static, (PeerId, Result<Bytes, connexa::error::Error>)>, Error> {
        self.connexa
            .request_response()
            .send_requests(peers, request)
            .await
            .map_err(Into::into)
    }

    /// Sends a request to a specific peer.
    /// If a protocol is not supplied, it will use the first/default protocol that was set in
    /// [UninitializedIpfs::with_request_response].
    pub async fn send_response(
        &self,
        peer_id: PeerId,
        id: InboundRequestId,
        response: impl IntoRequest,
    ) -> Result<(), Error> {
        self.connexa
            .request_response()
            .send_response(peer_id, id, response)
            .await
            .map_err(Into::into)
    }

    /// Returns the known wantlist for the local node when the `peer` is `None` or the wantlist of the given `peer`
    pub async fn bitswap_wantlist(
        &self,
        peer: impl Into<Option<PeerId>>,
    ) -> Result<Vec<Cid>, Error> {
        async move {
            let peer = peer.into();
            let (tx, rx) = oneshot_channel();

            self.connexa
                .send_custom_event(IpfsEvent::WantList(peer, tx))
                .await?;

            Ok(rx.await??.await)
        }
        .instrument(self.span.clone())
        .await
    }

    #[cfg(feature = "stream")]
    pub async fn stream_control(&self) -> Result<connexa::prelude::stream::Control, Error> {
        self.connexa
            .stream()
            .control_handle()
            .await
            .map_err(Into::into)
    }

    #[cfg(feature = "stream")]
    pub async fn new_stream(
        &self,
        protocol: impl IntoStreamProtocol,
    ) -> Result<connexa::prelude::stream::IncomingStreams, Error> {
        let protocol = protocol.into_protocol()?;
        self.connexa
            .stream()
            .new_stream(protocol)
            .await
            .map_err(Into::into)
    }

    #[cfg(feature = "stream")]
    pub async fn open_stream(
        &self,
        peer_id: PeerId,
        protocol: impl IntoStreamProtocol,
    ) -> Result<connexa::prelude::Stream, Error> {
        self.connexa
            .stream()
            .open_stream(peer_id, protocol)
            .await
            .map_err(Into::into)
    }

    /// Returns a list of local blocks
    pub async fn refs_local(&self) -> Vec<Cid> {
        self.repo
            .list_blocks()
            .instrument(self.span.clone())
            .await
            .collect::<Vec<_>>()
            .await
    }

    /// Returns local listening addresses
    pub async fn listening_addresses(&self) -> Result<Vec<Multiaddr>, Error> {
        self.connexa
            .swarm()
            .listening_addresses()
            .await
            .map_err(Into::into)
    }

    /// Returns external addresses
    pub async fn external_addresses(&self) -> Result<Vec<Multiaddr>, Error> {
        self.connexa
            .swarm()
            .external_addresses()
            .await
            .map_err(Into::into)
    }

    /// Add a given multiaddr as a listening address. Will fail if the address is unsupported, or
    /// if it is already being listened on. Currently will invoke `Swarm::listen_on` internally,
    /// returning the first `Multiaddr` that is being listened on.
    pub async fn add_listening_address(&self, addr: Multiaddr) -> Result<ListenerId, Error> {
        self.connexa
            .swarm()
            .listen_on(addr)
            .await
            .map_err(Into::into)
    }

    pub async fn get_listening_address(&self, id: ListenerId) -> Result<Vec<Multiaddr>, Error> {
        self.connexa
            .swarm()
            .get_listening_addresses(id)
            .await
            .map_err(Into::into)
    }

    /// Stop listening on a previously added listening address. Fails if the address is not being
    /// listened to.
    ///
    /// The removal of all listening addresses added through unspecified addresses is not supported.
    pub async fn remove_listening_address(&self, id: ListenerId) -> Result<(), Error> {
        self.connexa
            .swarm()
            .remove_listener(id)
            .await
            .map_err(Into::into)
    }

    /// Add a given multiaddr as a external address to indenticate how our node can be reached.
    /// Note: We will not perform checks
    pub async fn add_external_address(&self, addr: Multiaddr) -> Result<(), Error> {
        self.connexa
            .swarm()
            .add_external_address(addr)
            .await
            .map_err(Into::into)
    }

    /// Removes a previously added external address.
    pub async fn remove_external_address(&self, addr: Multiaddr) -> Result<(), Error> {
        self.connexa
            .swarm()
            .remove_external_address(addr)
            .await
            .map_err(Into::into)
    }

    pub async fn swarm_events(&self) -> Result<BoxStream<'static, ConnexaSwarmEvent>, Error> {
        self.connexa.swarm().listener().await.map_err(Into::into)
    }

    pub async fn peer_connection_events(
        &self,
        target: PeerId,
    ) -> Result<BoxStream<'static, PeerConnectionEvents>, Error> {
        let mut st = self.connexa.swarm().listener().await?;

        let st = async_stream::stream! {
            while let Some(event) = st.next().await {
                yield match event {
                    ConnexaSwarmEvent::ConnectionEstablished { peer_id, connection_id, endpoint, .. } if peer_id == target => {
                        match endpoint {
                            ConnectedPoint::Listener { send_back_addr, .. } => {
                                PeerConnectionEvents::IncomingConnection { connection_id, addr: send_back_addr }
                            }
                            ConnectedPoint::Dialer { address, ..  } => {
                                PeerConnectionEvents::OutgoingConnection { connection_id, addr: address }
                            }
                        }
                    },
                    ConnexaSwarmEvent::ConnectionClosed { peer_id, connection_id, .. } if peer_id == target => {
                        PeerConnectionEvents::ClosedConnection { connection_id }
                    }
                    _ => continue,
                }
            }
        };

        Ok(st.boxed())
    }

    /// Obtain the addresses associated with the given `PeerId`; they are first searched for locally
    /// and the DHT is used as a fallback: a `Kademlia::get_closest_peers(peer_id)` query is run and
    /// when it's finished, the newly added DHT records are checked for the existence of the desired
    /// `peer_id` and if it's there, the list of its known addresses is returned.
    pub async fn find_peer(&self, peer_id: PeerId) -> Result<Vec<Multiaddr>, Error> {
        self.connexa
            .dht()
            .find_peer(peer_id)
            .await
            .map_err(Into::into)
            .map(|list| list.into_iter().flat_map(|info| info.addrs).collect())
    }

    /// Performs a DHT lookup for providers of a value to the given key.
    ///
    /// Returns a list of peers found providing the Cid.
    pub async fn get_providers(
        &self,
        cid: Cid,
    ) -> Result<BoxStream<'static, Result<HashSet<PeerId>, connexa::error::Error>>, Error> {
        self.dht_get_providers(cid).await
    }

    /// Performs a DHT lookup for providers of a value to the given key.
    pub async fn dht_get_providers(
        &self,
        key: impl ToRecordKey,
    ) -> Result<BoxStream<'static, Result<HashSet<PeerId>, connexa::error::Error>>, Error> {
        self.connexa
            .dht()
            .get_providers(key)
            .await
            .map_err(Into::into)
    }

    /// Establishes the node as a provider of a block with the given Cid: it publishes a provider
    /// record with the given key (Cid) and the node's PeerId to the peers closest to the key. The
    /// publication of provider records is periodically repeated as per the interval specified in
    /// `libp2p`'s  `KademliaConfig`.
    pub async fn provide(&self, cid: Cid) -> Result<(), Error> {
        // don't provide things we don't actually have
        if !self.repo.contains(&cid).await? {
            return Err(anyhow!(
                "Error: block {} not found locally, cannot provide",
                cid
            ));
        }

        self.dht_provide(cid.hash().to_bytes()).await
    }

    /// Establishes the node as a provider of a given Key: it publishes a provider
    /// record with the given key and the node's PeerId to the peers closest to the key. The
    /// publication of provider records is periodically repeated as per the interval specified in
    /// `libp2p`'s  `KademliaConfig`.
    pub async fn dht_provide(&self, key: impl ToRecordKey) -> Result<(), Error> {
        self.connexa.dht().provide(key).await.map_err(Into::into)
    }

    /// Fetches the block, and, if set, recursively walk the graph loading all the blocks to the blockstore.
    pub fn fetch(&self, cid: &Cid) -> RepoFetch<DefaultStorage> {
        self.repo.fetch(cid).span(self.span.clone())
    }

    /// Returns a list of peers closest to the given `PeerId`, as suggested by the DHT. The
    /// node must have at least one known peer in its routing table in order for the query
    /// to return any values.
    pub async fn get_closest_peers(&self, peer_id: PeerId) -> Result<Vec<PeerId>, Error> {
        self.connexa
            .dht()
            .find_peer(peer_id)
            .await
            .map_err(Into::into)
            .map(|list| list.into_iter().map(|info| info.peer_id).collect())
    }

    /// Change the DHT mode
    pub async fn dht_mode(&self, mode: DhtMode) -> Result<(), Error> {
        let mode = match mode {
            DhtMode::Client => Some(Mode::Client),
            DhtMode::Server => Some(Mode::Server),
            DhtMode::Auto => None,
        };
        self.connexa.dht().set_mode(mode).await.map_err(Into::into)
    }

    /// Attempts to look a key up in the DHT and returns the values found in the records
    /// containing that key.
    pub async fn dht_get(
        &self,
        key: impl ToRecordKey,
    ) -> Result<BoxStream<'static, Record>, Error> {
        let st = self.connexa.dht().get(key).await?;
        let st = st
            .filter_map(|result| async move { result.ok() })
            .map(|record| record.record)
            .boxed();

        Ok(st)
    }

    /// Stores the given key + value record locally and replicates it in the DHT. It doesn't
    /// expire locally and is periodically replicated in the DHT, as per the `KademliaConfig`
    /// setup.
    pub async fn dht_put(
        &self,
        key: impl AsRef<[u8]>,
        value: impl Into<Bytes>,
        quorum: Quorum,
    ) -> Result<(), Error> {
        let key = key.as_ref();

        let key_str = String::from_utf8_lossy(key);

        let key = if let Ok((prefix, _)) = split_dht_key(&key_str) {
            if let Some(key_fn) = self.record_key_validator.get(prefix) {
                key_fn(&key_str)?
            } else {
                RecordKey::from(key.to_vec())
            }
        } else {
            RecordKey::from(key.to_vec())
        };

        self.dht_put_record(key, value, quorum).await
    }

    pub(crate) async fn dht_put_record(
        &self,
        key: RecordKey,
        value: impl Into<Bytes>,
        quorum: Quorum,
    ) -> Result<(), Error> {
        self.connexa
            .dht()
            .put(key, value, quorum)
            .await
            .map_err(Into::into)
    }

    /// Add static relay peer
    pub async fn add_static_relay(&self, peer_id: PeerId, addr: Multiaddr) -> Result<bool, Error> {
        self.connexa
            .relay()
            .add_static_relay(peer_id, addr)
            .await
            .map_err(Into::into)
    }

    /// Remove static relay peer
    pub async fn remove_static_relay(&self, peer_id: PeerId) -> Result<bool, Error> {
        self.connexa
            .relay()
            .remove_static_relay(peer_id)
            .await
            .map_err(Into::into)
    }

    /// List all static relays.
    pub async fn list_static_relays(&self) -> Result<Vec<(PeerId, Vec<Multiaddr>)>, Error> {
        self.connexa
            .relay()
            .list_static_relays()
            .await
            .map_err(Into::into)
    }

    /// Enables autorelay
    pub async fn enable_autorelay(&self) -> Result<(), Error> {
        self.connexa
            .relay()
            .enable_auto_relay()
            .await
            .map_err(Into::into)
    }

    /// Disables autorelay
    pub async fn disable_autorelay(&self) -> Result<(), Error> {
        self.connexa
            .relay()
            .disable_auto_relay()
            .await
            .map_err(Into::into)
    }

    pub async fn rendezvous_register_namespace(
        &self,
        namespace: impl IntoNamespace,
        ttl: impl Into<Option<u64>>,
        peer_id: PeerId,
    ) -> Result<(), Error> {
        self.connexa
            .rendezvous()
            .register(peer_id, namespace, ttl.into())
            .await
            .map_err(Into::into)
    }

    pub async fn rendezvous_unregister_namespace(
        &self,
        namespace: impl IntoNamespace,
        peer_id: PeerId,
    ) -> Result<(), Error> {
        self.connexa
            .rendezvous()
            .unregister(peer_id, namespace)
            .await
            .map_err(Into::into)
    }

    pub async fn rendezvous_namespace_discovery(
        &self,
        namespace: impl IntoNamespace,
        ttl: impl Into<Option<u64>>,
        peer_id: PeerId,
    ) -> Result<HashMap<PeerId, Vec<Multiaddr>>, Error> {
        self.connexa
            .rendezvous()
            .discovery(peer_id, namespace, ttl.into(), None)
            .await
            .map(|(_, list)| HashMap::from_iter(list))
            .map_err(anyhow::Error::from)
    }

    /// Walk the given Iplds' links up to `max_depth` (or indefinitely for `None`). Will return
    /// any duplicate trees unless `unique` is `true`.
    ///
    /// More information and a `'static` lifetime version available at [`refs::iplds_refs`].
    pub fn refs<'a, Iter>(
        &'a self,
        iplds: Iter,
        max_depth: Option<u64>,
        unique: bool,
    ) -> impl futures::Stream<Item = Result<refs::Edge, anyhow::Error>> + Send + 'a
    where
        Iter: IntoIterator<Item = (Cid, Ipld)> + Send + 'a,
    {
        refs::iplds_refs(self.repo(), iplds, max_depth, unique)
    }

    /// Obtain the list of addresses of bootstrapper nodes that are currently used.
    pub async fn get_bootstraps(&self) -> Result<Vec<Multiaddr>, Error> {
        async move {
            let (tx, rx) = oneshot_channel();

            self.connexa
                .send_custom_event(IpfsEvent::GetBootstrappers(tx))
                .await?;

            Ok(rx.await?)
        }
        .instrument(self.span.clone())
        .await
    }

    /// Extend the list of used bootstrapper nodes with an additional address.
    /// Return value cannot be used to determine if the `addr` was a new bootstrapper, subject to
    /// change.
    pub async fn add_bootstrap(&self, addr: Multiaddr) -> Result<Multiaddr, Error> {
        async move {
            let (tx, rx) = oneshot_channel();

            self.connexa
                .send_custom_event(IpfsEvent::AddBootstrapper(addr, tx))
                .await?;

            rx.await?
        }
        .instrument(self.span.clone())
        .await
    }

    /// Remove an address from the currently used list of bootstrapper nodes.
    /// Return value cannot be used to determine if the `addr` was an actual bootstrapper, subject to
    /// change.
    pub async fn remove_bootstrap(&self, addr: Multiaddr) -> Result<Multiaddr, Error> {
        async move {
            let (tx, rx) = oneshot_channel();

            self.connexa
                .send_custom_event(IpfsEvent::RemoveBootstrapper(addr, tx))
                .await?;

            rx.await?
        }
        .instrument(self.span.clone())
        .await
    }

    /// Clear the currently used list of bootstrapper nodes, returning the removed addresses.
    pub async fn clear_bootstrap(&self) -> Result<Vec<Multiaddr>, Error> {
        async move {
            let (tx, rx) = oneshot_channel();

            self.connexa
                .send_custom_event(IpfsEvent::ClearBootstrappers(tx))
                .await?;

            rx.await?
        }
        .instrument(self.span.clone())
        .await
    }

    /// Restore the originally configured bootstrapper node list by adding them to the list of the
    /// currently used bootstrapper node address list; returns the restored addresses.
    pub async fn default_bootstrap(&self) -> Result<Vec<Multiaddr>, Error> {
        async move {
            let (tx, rx) = oneshot_channel();

            self.connexa
                .send_custom_event(IpfsEvent::DefaultBootstrap(tx))
                .await?;

            rx.await?
        }
        .instrument(self.span.clone())
        .await
    }

    /// Bootstraps the local node to join the DHT: it looks up the node's own ID in the
    /// DHT and introduces it to the other nodes in it; at least one other node must be
    /// known in order for the process to succeed. Subsequently, additional queries are
    /// ran with random keys so that the buckets farther from the closest neighbor also
    /// get refreshed.
    pub async fn bootstrap(&self) -> Result<(), Error> {
        self.connexa.dht().bootstrap().await.map_err(Into::into)
    }

    /// Add address of a peer to the address book
    pub async fn add_peer(&self, opt: impl IntoAddPeerOpt) -> Result<(), Error> {
        let opt: AddPeerOpt = opt.into_opt()?;
        if opt.addresses().is_empty() {
            anyhow::bail!("no address supplied");
        }

        let (tx, rx) = oneshot::channel();

        self.connexa
            .send_custom_event(IpfsEvent::AddPeer(opt, tx))
            .await?;

        rx.await??;
        Ok(())
    }

    /// Remove peer from the address book
    pub async fn remove_peer(&self, peer_id: PeerId) -> Result<bool, Error> {
        let (tx, rx) = oneshot::channel();

        self.connexa
            .send_custom_event(IpfsEvent::RemovePeer(peer_id, None, tx))
            .await?;

        rx.await.map_err(anyhow::Error::from)?
    }

    /// Remove peer address from the address book
    pub async fn remove_peer_address(
        &self,
        peer_id: PeerId,
        addr: Multiaddr,
    ) -> Result<bool, Error> {
        let (tx, rx) = oneshot::channel();

        self.connexa
            .send_custom_event(IpfsEvent::RemovePeer(peer_id, Some(addr), tx))
            .await?;

        rx.await.map_err(anyhow::Error::from)?
    }

    /// Returns the Bitswap peers for the `Node`.
    pub async fn get_bitswap_peers(&self) -> Result<Vec<PeerId>, Error> {
        let (tx, rx) = oneshot_channel();

        self.connexa
            .send_custom_event(IpfsEvent::GetBitswapPeers(tx))
            .await?;

        Ok(rx.await??.await)
    }

    pub async fn bitswap_stats(&self) -> Result<crate::p2p::bitswap::BitswapStats, Error> {
        let (tx, rx) = oneshot_channel();

        self.connexa
            .send_custom_event(IpfsEvent::BitswapStats(tx))
            .await?;

        Ok(rx.await??.await)
    }

    /// Returns the keypair to the node
    pub fn keypair(&self) -> &Keypair {
        self.connexa.keypair()
    }

    /// Returns the keychain
    pub fn keychain(&self) -> &Keychain<DefaultKeystore> {
        self.connexa.keychain()
    }

    /// Exit daemon.
    pub async fn exit_daemon(self) {
        // FIXME: this is a stopgap measure needed while repo is part of the struct Ipfs instead of
        // the background task or stream. After that this could be handled by dropping.
        self.repo.shutdown();

        self.connexa.shutdown();

        // terminate task that handles GC
        self._gc_guard.abort();
        // give back to the runtime to process anything in the background
        async_rt::task::yield_now().await;
    }
}

#[derive(Debug)]
pub struct AddPeerOpt {
    peer_id: PeerId,
    addresses: Vec<Multiaddr>,
    condition: Option<PeerCondition>,
    dial: bool,
    keepalive: bool,
    reconnect: Option<(Duration, u8)>,
}

impl AddPeerOpt {
    pub fn with_peer_id(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            addresses: vec![],
            condition: None,
            dial: false,
            keepalive: false,
            reconnect: None,
        }
    }

    pub fn add_address(mut self, mut addr: Multiaddr) -> Self {
        if addr.is_empty() {
            return self;
        }

        match addr.iter().last() {
            // if the address contains a peerid, we should confirm it matches the initial peer
            Some(Protocol::P2p(peer_id)) if peer_id == self.peer_id => {
                addr.pop();
            }
            Some(Protocol::P2p(_)) => return self,
            _ => {}
        }

        if !self.addresses.contains(&addr) {
            self.addresses.push(addr);
        }

        self
    }

    pub fn set_addresses(mut self, addrs: Vec<Multiaddr>) -> Self {
        for addr in addrs {
            self = self.add_address(addr);
        }

        self
    }

    pub fn set_peer_condition(mut self, condition: PeerCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn set_dial(mut self, dial: bool) -> Self {
        self.dial = dial;
        self
    }

    pub fn set_reconnect(mut self, reconnect: impl Into<Option<(Duration, u8)>>) -> Self {
        self.reconnect = reconnect.into();
        self
    }

    pub fn reconnect(mut self, duration: Duration, interval: u8) -> Self {
        self.reconnect = Some((duration, interval));
        self
    }

    pub fn keepalive(mut self) -> Self {
        self.keepalive = true;
        self
    }

    pub fn set_keepalive(mut self, keepalive: bool) -> Self {
        self.keepalive = keepalive;
        self
    }
}

impl AddPeerOpt {
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn addresses(&self) -> &[Multiaddr] {
        &self.addresses
    }

    pub fn can_keep_alive(&self) -> bool {
        self.keepalive
    }

    pub fn reconnect_opt(&self) -> Option<(Duration, u8)> {
        self.reconnect
    }

    pub fn to_dial_opts(&self) -> Option<DialOpts> {
        if !self.dial {
            return None;
        }

        // We dial without addresses attached because it will they will be fetched within the address book
        // which will allow us not only to use those addresses but any addresses from other behaviours
        let opts = DialOpts::peer_id(self.peer_id)
            .condition(self.condition.unwrap_or_default())
            .build();

        Some(opts)
    }
}

pub trait IntoAddPeerOpt {
    fn into_opt(self) -> Result<AddPeerOpt, anyhow::Error>;
}

impl IntoAddPeerOpt for AddPeerOpt {
    fn into_opt(self) -> Result<AddPeerOpt, anyhow::Error> {
        Ok(self)
    }
}

impl IntoAddPeerOpt for (PeerId, Multiaddr) {
    fn into_opt(self) -> Result<AddPeerOpt, anyhow::Error> {
        let (peer_id, addr) = self;
        Ok(AddPeerOpt::with_peer_id(peer_id).add_address(addr))
    }
}

impl IntoAddPeerOpt for (PeerId, Vec<Multiaddr>) {
    fn into_opt(self) -> Result<AddPeerOpt, anyhow::Error> {
        let (peer_id, addrs) = self;
        Ok(AddPeerOpt::with_peer_id(peer_id).set_addresses(addrs))
    }
}

impl IntoAddPeerOpt for Multiaddr {
    fn into_opt(mut self) -> Result<AddPeerOpt, anyhow::Error> {
        let peer_id = self
            .extract_peer_id()
            .ok_or(anyhow::anyhow!("address does not contain peer id"))
            .map_err(std::io::Error::other)?;
        Ok(AddPeerOpt::with_peer_id(peer_id).add_address(self))
    }
}

#[inline]
pub(crate) fn split_dht_key(key: &str) -> anyhow::Result<(&str, &str)> {
    anyhow::ensure!(!key.is_empty(), "Key cannot be empty");

    let (key, val) = {
        let data = key
            .split('/')
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>();

        anyhow::ensure!(
            !data.is_empty() && data.len() == 2,
            "split dats cannot be empty"
        );

        (data[0], data[1])
    };

    Ok((key, val))
}

#[inline]
pub(crate) fn ipns_to_dht_key<B: AsRef<str>>(key: B) -> anyhow::Result<RecordKey> {
    let default_ipns_prefix = b"/ipns/";

    let mut key = key.as_ref().trim().to_string();

    anyhow::ensure!(!key.is_empty(), "Key cannot be empty");

    if key.starts_with('1') || key.starts_with('Q') {
        key.insert(0, 'z');
    }

    let mut data = multibase::decode(key).map(|(_, data)| data)?;

    if data[0] != 0x01 && data[1] != 0x72 {
        data = [vec![0x01, 0x72], data].concat();
    }

    data = [default_ipns_prefix.to_vec(), data[2..].to_vec()].concat();

    Ok(data.into())
}

#[inline]
pub(crate) fn to_dht_key<B: AsRef<str>, F: Fn(&str) -> anyhow::Result<RecordKey>>(
    (prefix, func): (&str, F),
    key: B,
) -> anyhow::Result<RecordKey> {
    let key = key.as_ref().trim();

    let (key, val) = split_dht_key(key)?;

    anyhow::ensure!(!key.is_empty(), "Key cannot be empty");
    anyhow::ensure!(!val.is_empty(), "Value cannot be empty");

    if key == prefix {
        return func(val);
    }

    anyhow::bail!("Invalid prefix")
}

use crate::p2p::AddressBookConfig;
use crate::repo::{DefaultKeystore, RepoGetBlock, RepoPutBlock};
#[cfg(all(feature = "full", not(target_arch = "wasm32")))]
#[doc(hidden)]
pub use node::Node;

/// Node module provides an easy to use interface used in `tests/`.
#[cfg(all(feature = "full", not(target_arch = "wasm32")))]
mod node {
    use super::*;
    use crate::builder::DefaultIpfsBuilder;

    /// Node encapsulates everything to setup a testing instance so that multi-node tests become
    /// easier.
    pub struct Node {
        /// The Ipfs facade.
        pub ipfs: Ipfs,
        /// The peer identifier on the network.
        pub id: PeerId,
        /// The listened to and externally visible addresses. The addresses are suffixed with the
        /// P2p protocol containing the node's PeerID.
        pub addrs: Vec<Multiaddr>,
    }

    impl IntoAddPeerOpt for &Node {
        fn into_opt(self) -> Result<AddPeerOpt, anyhow::Error> {
            Ok(AddPeerOpt::with_peer_id(self.id).set_addresses(self.addrs.clone()))
        }
    }

    impl Node {
        /// Initialises a new `Node` with an in-memory store backed configuration.
        ///
        /// This will use the testing defaults for the `IpfsOptions`. If `IpfsOptions` has been
        /// initialised manually, use `Node::with_options` instead.
        pub async fn new<T: AsRef<str>>(name: T) -> Self {
            Self::with_options(Some(trace_span!("ipfs", node = name.as_ref())), None).await
        }

        /// Connects to a peer at the given address.
        pub async fn connect(&self, opt: impl Into<DialOpts>) -> Result<(), Error> {
            let opts = opt.into();
            if let Some(peer_id) = opts.get_peer_id()
                && self.ipfs.is_connected(peer_id).await?
            {
                return Ok(());
            }
            self.ipfs.connect(opts).await.map(|_| ())
        }

        /// Returns a new `Node` based on `IpfsOptions`.
        pub async fn with_options(span: Option<Span>, addr: Option<Vec<Multiaddr>>) -> Self {
            // for future: assume UninitializedIpfs handles instrumenting any futures with the
            // given span
            let mut uninit = DefaultIpfsBuilder::new()
                .with_default()
                .enable_tcp()
                .enable_memory_transport()
                .with_request_response(Default::default());

            if let Some(span) = span {
                uninit = uninit.set_span(span);
            }

            let list = addr.unwrap_or_else(|| vec![Multiaddr::empty().with(Protocol::Memory(0))]);

            let ipfs = uninit.start().await.unwrap();

            let id = ipfs.keypair().public().to_peer_id();
            for addr in list {
                ipfs.add_listening_address(addr.clone())
                    .await
                    .expect("To succeed");
            }

            let addrs = ipfs.listening_addresses().await.unwrap();

            for addr in addrs {
                let addr = addr.with_p2p(id).expect("doesnt contain peer_id");
                ipfs.add_external_address(addr).await.expect("To succeed");
            }

            let addrs = ipfs
                .external_addresses()
                .await
                .expect("at least one address");

            Node { ipfs, id, addrs }
        }

        /// Returns the subscriptions for a `Node`.
        #[allow(clippy::type_complexity)]
        pub fn get_subscriptions(
            &self,
        ) -> &parking_lot::Mutex<HashMap<Cid, HashMap<u64, crate::repo::BlockWaiter>>> {
            &self.ipfs.repo.inner.subscriptions
        }

        /// Bootstraps the local node to join the DHT: it looks up the node's own ID in the
        /// DHT and introduces it to the other nodes in it; at least one other node must be
        /// known in order for the process to succeed. Subsequently, additional queries are
        /// ran with random keys so that the buckets farther from the closest neighbor also
        /// get refreshed.
        pub async fn bootstrap(&self) -> Result<(), Error> {
            self.ipfs.bootstrap().await
        }

        pub async fn add_node(&self, node: &Self) -> Result<(), Error> {
            for addr in &node.addrs {
                self.add_peer((node.id, addr.to_owned())).await?;
            }

            Ok(())
        }

        /// Shuts down the `Node`.
        pub async fn shutdown(self) {
            self.ipfs.exit_daemon().await;
        }
    }

    impl std::ops::Deref for Node {
        type Target = Ipfs;

        fn deref(&self) -> &Self::Target {
            &self.ipfs
        }
    }

    impl std::ops::DerefMut for Node {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.ipfs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::block::BlockCodec;
    use ipld_core::ipld;
    use multihash_codetable::Code;
    use multihash_derive::MultihashDigest;

    #[tokio::test]
    async fn test_put_and_get_block() {
        let ipfs = Node::new("test_node").await;

        let data = b"hello block\n".to_vec();
        let cid = Cid::new_v1(BlockCodec::Raw.into(), Code::Sha2_256.digest(&data));
        let block = Block::new(cid, data).unwrap();

        let cid: Cid = ipfs.put_block(&block).await.unwrap();
        let new_block = ipfs.get_block(cid).await.unwrap();
        assert_eq!(block, new_block);
    }

    #[tokio::test]
    async fn test_put_and_get_dag() {
        let ipfs = Node::new("test_node").await;

        let data = ipld!([-1, -2, -3]);
        let cid = ipfs.put_dag(data.clone()).await.unwrap();
        let new_data = ipfs.get_dag(cid).await.unwrap();
        assert_eq!(data, new_data);
    }

    #[tokio::test]
    async fn test_pin_and_unpin() {
        let ipfs = Node::new("test_node").await;

        let data = ipld!([-1, -2, -3]);
        let cid = ipfs.put_dag(data.clone()).pin(false).await.unwrap();

        assert!(ipfs.is_pinned(cid).await.unwrap());
        ipfs.remove_pin(cid).await.unwrap();
        assert!(!ipfs.is_pinned(cid).await.unwrap());
    }
}
