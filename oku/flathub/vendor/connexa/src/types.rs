#![allow(unused_imports)]
use crate::error::ConnexaResult;
use crate::handle::swarm::ConnectionTarget;
use crate::prelude::swarm::{DialError, ListenError, ListenOpts, SwarmEvent};
use bytes::Bytes;
use either::Either;
use futures::channel::{mpsc, oneshot};
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use indexmap::IndexSet;
use libp2p::core::ConnectedPoint;
#[cfg(feature = "gossipsub")]
use libp2p::gossipsub::{MessageAcceptance, MessageId};
#[cfg(feature = "kad")]
use libp2p::kad::{Mode, PeerInfo, PeerRecord, ProviderRecord, Quorum, Record, RecordKey};
#[cfg(feature = "rendezvous")]
use libp2p::rendezvous::Cookie;
#[cfg(feature = "request-response")]
use libp2p::request_response::InboundRequestId;
use libp2p::swarm::derive_prelude::ListenerId;
use libp2p::swarm::dial_opts::DialOpts;
use libp2p::swarm::{ConnectionError, ConnectionId};
use libp2p::{Multiaddr, PeerId, StreamProtocol};
use libp2p_connection_limits::ConnectionLimits;
use other_error::ArcError;
use std::collections::HashSet;
use std::convert::Infallible;
use std::sync::Arc;

type Result<T> = std::io::Result<T>;

#[derive(Debug)]
pub enum Command<T = ()> {
    Swarm(SwarmCommand),
    #[cfg(feature = "gossipsub")]
    Gossipsub(GossipsubCommand),
    #[cfg(feature = "floodsub")]
    Floodsub(FloodsubCommand),
    #[cfg(feature = "kad")]
    Dht(DHTCommand),
    #[cfg(feature = "request-response")]
    RequestResponse(RequestResponseCommand),
    #[cfg(feature = "stream")]
    Stream(StreamCommand),
    #[cfg(feature = "rendezvous")]
    Rendezvous(RendezvousCommand),
    #[cfg(feature = "autonat")]
    Autonat(AutonatCommand),
    #[cfg(feature = "relay")]
    AutoRelay(AutoRelayCommand),
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "relay")]
    RelayServer(RelayServerCommand),
    Whitelist(WhitelistCommand),
    Blacklist(BlacklistCommand),
    ConnectionLimits(ConnectionLimitsCommand),
    Peerstore(PeerstoreCommand),
    Custom(T),
}

impl<T> From<SwarmCommand> for Command<T> {
    fn from(cmd: SwarmCommand) -> Self {
        Command::Swarm(cmd)
    }
}

#[cfg(feature = "autonat")]
impl<T> From<AutonatCommand> for Command<T> {
    fn from(cmd: AutonatCommand) -> Self {
        Command::Autonat(cmd)
    }
}

#[cfg(feature = "gossipsub")]
impl<T> From<GossipsubCommand> for Command<T> {
    fn from(cmd: GossipsubCommand) -> Self {
        Command::Gossipsub(cmd)
    }
}

#[cfg(feature = "floodsub")]
impl<T> From<FloodsubCommand> for Command<T> {
    fn from(cmd: FloodsubCommand) -> Self {
        Command::Floodsub(cmd)
    }
}

#[cfg(feature = "kad")]
impl<T> From<DHTCommand> for Command<T> {
    fn from(cmd: DHTCommand) -> Self {
        Command::Dht(cmd)
    }
}

#[cfg(feature = "request-response")]
impl<T> From<RequestResponseCommand> for Command<T> {
    fn from(cmd: RequestResponseCommand) -> Self {
        Command::RequestResponse(cmd)
    }
}

#[cfg(feature = "stream")]
impl<T> From<StreamCommand> for Command<T> {
    fn from(cmd: StreamCommand) -> Self {
        Command::Stream(cmd)
    }
}

#[cfg(feature = "rendezvous")]
impl<T> From<RendezvousCommand> for Command<T> {
    fn from(cmd: RendezvousCommand) -> Self {
        Command::Rendezvous(cmd)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "relay")]
impl<T> From<RelayServerCommand> for Command<T> {
    fn from(cmd: RelayServerCommand) -> Self {
        Command::RelayServer(cmd)
    }
}

#[cfg(feature = "relay")]
impl<T> From<AutoRelayCommand> for Command<T> {
    fn from(cmd: AutoRelayCommand) -> Self {
        Command::AutoRelay(cmd)
    }
}

impl<T> From<WhitelistCommand> for Command<T> {
    fn from(cmd: WhitelistCommand) -> Self {
        Command::Whitelist(cmd)
    }
}

impl<T> From<BlacklistCommand> for Command<T> {
    fn from(cmd: BlacklistCommand) -> Self {
        Command::Blacklist(cmd)
    }
}

impl<T> From<ConnectionLimitsCommand> for Command<T> {
    fn from(cmd: ConnectionLimitsCommand) -> Self {
        Command::ConnectionLimits(cmd)
    }
}

impl<T> From<PeerstoreCommand> for Command<T> {
    fn from(value: PeerstoreCommand) -> Self {
        Command::Peerstore(value)
    }
}

#[derive(Debug)]
pub enum SwarmCommand {
    Dial {
        opt: DialOpts,
        resp: oneshot::Sender<ConnexaResult<ConnectionId>>,
    },
    IsConnected {
        peer_id: PeerId,
        resp: oneshot::Sender<bool>,
    },
    Disconnect {
        target_type: ConnectionTarget,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    ConnectedPeers {
        resp: oneshot::Sender<Vec<PeerId>>,
    },
    ListenOn {
        address: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<ListenerId>>,
    },
    GetListeningAddress {
        id: ListenerId,
        resp: oneshot::Sender<ConnexaResult<Vec<Multiaddr>>>,
    },
    RemoveListener {
        listener_id: ListenerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    AddExternalAddress {
        address: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    RemoveExternalAddress {
        address: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    ListExternalAddresses {
        resp: oneshot::Sender<Vec<Multiaddr>>,
    },
    ListListeningAddresses {
        resp: oneshot::Sender<Vec<Multiaddr>>,
    },
    AddPeerAddress {
        peer_id: PeerId,
        address: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Listener {
        resp: oneshot::Sender<mpsc::Receiver<ConnexaSwarmEvent>>,
    },
}

#[derive(Debug, Clone)]
pub enum ConnexaSwarmEvent {
    ConnectionEstablished {
        peer_id: PeerId,
        connection_id: ConnectionId,
        endpoint: ConnectedPoint,
        established: u32,
    },
    ConnectionClosed {
        peer_id: PeerId,
        connection_id: ConnectionId,
        endpoint: ConnectedPoint,
        num_established: u32,
        cause: Option<ArcError<ConnectionError>>,
    },
    IncomingConnection {
        connection_id: ConnectionId,
        local_addr: Multiaddr,
        send_back_addr: Multiaddr,
    },
    IncomingConnectionError {
        connection_id: ConnectionId,
        local_addr: Multiaddr,
        send_back_addr: Multiaddr,
        error: ArcError<ListenError>,
        peer_id: Option<PeerId>,
    },
    OutgoingConnectionError {
        connection_id: ConnectionId,
        peer_id: Option<PeerId>,
        error: ArcError<DialError>,
    },
    NewListenAddr {
        id: ListenerId,
        address: Multiaddr,
    },
    ListenAddrExpired {
        id: ListenerId,
        address: Multiaddr,
    },
    ListenAddrClosed {
        id: ListenerId,
        addresses: Vec<Multiaddr>,
    },
    NewExternalAddr {
        address: Multiaddr,
    },
    ExternalAddrExpired {
        address: Multiaddr,
    },
    ExternalAddrOfPeer {
        peer_id: PeerId,
        address: Multiaddr,
    },
}

#[cfg(feature = "floodsub")]
#[derive(Debug)]
pub enum FloodsubCommand {
    Subscribe {
        topic: libp2p::floodsub::Topic,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Unsubscribe {
        topic: libp2p::floodsub::Topic,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    AddNodeToPartialView {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    RemoveNodeFromPartialView {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    FloodsubListener {
        topic: libp2p::floodsub::Topic,
        resp: oneshot::Sender<ConnexaResult<mpsc::Receiver<FloodsubEvent>>>,
    },
    Publish(PubsubFloodsubPublish, oneshot::Sender<ConnexaResult<()>>),
}

#[cfg(feature = "gossipsub")]
#[derive(Debug)]
pub enum GossipsubCommand {
    Subscribe {
        topic: libp2p::gossipsub::TopicHash,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Unsubscribe {
        topic: libp2p::gossipsub::TopicHash,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Subscribed {
        resp: oneshot::Sender<ConnexaResult<Vec<libp2p::gossipsub::TopicHash>>>,
    },
    Peers {
        topic: libp2p::gossipsub::TopicHash,
        resp: oneshot::Sender<ConnexaResult<Vec<PeerId>>>,
    },
    GossipsubListener {
        topic: libp2p::gossipsub::TopicHash,
        resp: oneshot::Sender<ConnexaResult<mpsc::Receiver<GossipsubEvent>>>,
    },
    Publish {
        topic: libp2p::gossipsub::TopicHash,
        data: Bytes,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    ReportMessage {
        peer_id: PeerId,
        message_id: MessageId,
        accept: MessageAcceptance,
        resp: oneshot::Sender<ConnexaResult<bool>>,
    },
}

#[derive(Debug)]
pub enum ConnectionLimitsCommand {
    Get {
        resp: oneshot::Sender<Result<ConnectionLimits>>,
    },
    Set {
        limits: ConnectionLimits,
        resp: oneshot::Sender<Result<()>>,
    },
}

#[cfg(feature = "floodsub")]
#[derive(Debug)]
pub enum PubsubFloodsubPublish {
    Publish {
        topic: libp2p::floodsub::Topic,
        data: Bytes,
    },
    PublishAny {
        topic: libp2p::floodsub::Topic,
        data: Bytes,
    },
    PublishMany {
        topics: Vec<libp2p::floodsub::Topic>,
        data: Bytes,
    },
    PublishManyAny {
        topics: Vec<libp2p::floodsub::Topic>,
        data: Bytes,
    },
}

#[cfg(feature = "autonat")]
#[derive(Debug)]
pub enum AutonatCommand {
    PublicAddress {
        resp: oneshot::Sender<ConnexaResult<Option<Multiaddr>>>,
    },
    NatStatus {
        resp: oneshot::Sender<ConnexaResult<libp2p::autonat::NatStatus>>,
    },
    AddServer {
        peer: PeerId,
        address: Option<Multiaddr>,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    RemoveServer {
        peer: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Probe {
        address: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
}

#[cfg(feature = "kad")]
#[derive(Debug)]
pub enum DHTCommand {
    FindPeer {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<Vec<PeerInfo>>>,
    },
    Bootstrap {
        lazy: bool,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Provide {
        key: RecordKey,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    StopProviding {
        key: RecordKey,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    GetProviders {
        key: RecordKey,
        resp: oneshot::Sender<ConnexaResult<mpsc::Receiver<ConnexaResult<HashSet<PeerId>>>>>,
    },
    SetDHTMode {
        mode: Option<Mode>,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    DHTMode {
        resp: oneshot::Sender<ConnexaResult<Mode>>,
    },

    AddAddress {
        peer_id: PeerId,
        addr: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },

    RemoveAddress {
        peer_id: PeerId,
        addr: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },

    RemovePeer {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Get {
        key: RecordKey,
        resp: oneshot::Sender<ConnexaResult<mpsc::Receiver<ConnexaResult<PeerRecord>>>>,
    },
    Put {
        key: RecordKey,
        data: Bytes,
        quorum: Quorum,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Remove {
        key: RecordKey,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    PutTo {
        key: RecordKey,
        target: Vec<PeerId>,
        data: Bytes,
        quorum: Quorum,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Listener {
        key: Option<RecordKey>,
        resp: oneshot::Sender<ConnexaResult<mpsc::Receiver<DHTEvent>>>,
    },
}

#[cfg(feature = "relay")]
#[derive(Debug)]
pub enum AutoRelayCommand {
    AddStaticRelay {
        peer_id: PeerId,
        relay_addr: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<bool>>,
    },
    RemoveStaticRelay {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<bool>>,
    },
    DisableRelays {
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    ListStaticRelays {
        resp: oneshot::Sender<ConnexaResult<PeerAddressList>>,
    },
    GetStaticRelay {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<Vec<Multiaddr>>>,
    },
    EnableAutoRelay {
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    DisableAutoRelay {
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
}

#[cfg(feature = "request-response")]
type ResponseStream = BoxStream<'static, (PeerId, ConnexaResult<Bytes>)>;
#[cfg(feature = "request-response")]
type ResponseFuture = BoxFuture<'static, ConnexaResult<Bytes>>;

type PeerAddressList = Vec<(PeerId, Vec<Multiaddr>)>;

#[cfg(feature = "rendezvous")]
type RendezvousDiscoverResponse = ConnexaResult<(Cookie, Vec<(PeerId, Vec<Multiaddr>)>)>;

#[cfg(feature = "request-response")]
#[derive(Debug)]
pub enum RequestResponseCommand {
    SendRequests {
        protocol: Option<StreamProtocol>,
        peers: IndexSet<PeerId>,
        request: Bytes,
        resp: oneshot::Sender<ConnexaResult<ResponseStream>>,
    },
    SendRequest {
        protocol: Option<StreamProtocol>,
        peer_id: PeerId,
        request: Bytes,
        resp: oneshot::Sender<ConnexaResult<ResponseFuture>>,
    },
    SendResponse {
        protocol: Option<StreamProtocol>,
        peer_id: PeerId,
        request_id: InboundRequestId,
        response: Bytes,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    ListenForRequests {
        protocol: Option<StreamProtocol>,
        resp: oneshot::Sender<ConnexaResult<mpsc::Receiver<(PeerId, InboundRequestId, Bytes)>>>,
    },
}

#[cfg(feature = "stream")]
#[derive(Debug)]
pub enum StreamCommand {
    NewStream {
        protocol: StreamProtocol,
        resp: oneshot::Sender<ConnexaResult<libp2p_stream::IncomingStreams>>,
    },
    ControlHandle {
        resp: oneshot::Sender<ConnexaResult<libp2p_stream::Control>>,
    },
}

#[cfg(feature = "rendezvous")]
#[derive(Debug)]
pub enum RendezvousCommand {
    Register {
        namespace: libp2p::rendezvous::Namespace,
        peer_id: PeerId,
        ttl: Option<u64>,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Unregister {
        namespace: libp2p::rendezvous::Namespace,
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Discover {
        namespace: Option<libp2p::rendezvous::Namespace>,
        peer_id: PeerId,
        cookie: Option<Cookie>,
        ttl: Option<u64>,
        resp: oneshot::Sender<RendezvousDiscoverResponse>,
    },
}

#[cfg(feature = "kad")]
#[derive(Clone, Debug)]
pub enum DHTEvent {
    PutRecord {
        source: PeerId,
        record: RecordHandle<Record>,
    },
    ProvideRecord {
        record: RecordHandle<ProviderRecord>,
    },
}

#[cfg(feature = "kad")]
impl DHTEvent {
    pub(crate) fn set_record_confirmation(&self, ch: oneshot::Sender<Record>) -> Self {
        let mut event = self.clone();
        match &mut event {
            DHTEvent::PutRecord { record, .. } => {
                record.confirm.replace(ch);
                event
            }
            _ => unreachable!("DHTEvent::PutRecord called on non-PutRecord"),
        }
    }
    pub(crate) fn set_provider_confirmation(&self, ch: oneshot::Sender<ProviderRecord>) -> Self {
        let mut event = self.clone();
        match &mut event {
            DHTEvent::ProvideRecord { record, .. } => {
                record.confirm.replace(ch);
                event
            }
            _ => unreachable!("DHTEvent::ProvideRecord called on non-ProvideRecord"),
        }
    }
}

#[cfg(feature = "kad")]
#[derive(Debug)]
pub struct RecordHandle<R> {
    /// The underlining record, if available
    /// Note: a record is only provided if configured by kademlia behaviour
    pub(crate) record: Option<R>,
    pub(crate) confirm: Option<oneshot::Sender<R>>,
}

#[cfg(feature = "kad")]
impl<R> RecordHandle<R> {
    /// The record awaiting confirmation, if any.
    pub fn record(&self) -> Option<&R> {
        self.record.as_ref()
    }

    /// Accept the record as received, storing it locally.
    pub fn accept(mut self) {
        if let (Some(ch), Some(record)) = (self.confirm.take(), self.record.take()) {
            let _ = ch.send(record);
        }
    }

    /// Accept the record, storing the provided (possibly modified) value.
    pub fn accept_with(mut self, record: R) {
        if let Some(ch) = self.confirm.take() {
            let _ = ch.send(record);
        }
    }

    /// Reject the record so it is not stored. Dropping the handle has the same effect.
    pub fn reject(self) {}
}

#[cfg(feature = "kad")]
impl<R: Clone> Clone for RecordHandle<R> {
    fn clone(&self) -> Self {
        Self {
            record: self.record.clone(),
            confirm: None,
        }
    }
}

#[cfg(feature = "gossipsub")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GossipsubEvent {
    Subscribed { peer_id: PeerId },
    Unsubscribed { peer_id: PeerId },
    Message { message: GossipsubMessage },
}

#[cfg(feature = "floodsub")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloodsubEvent {
    Subscribed { peer_id: PeerId },
    Unsubscribed { peer_id: PeerId },
    Message { message: FloodsubMessage },
}

#[cfg(feature = "gossipsub")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GossipsubMessage {
    pub message_id: MessageId,
    pub propagated_source: PeerId,
    pub source: Option<PeerId>,
    pub data: Bytes,
    pub sequence_number: Option<u64>,
}

#[cfg(feature = "floodsub")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FloodsubMessage {
    pub source: PeerId,
    pub data: Bytes,
    pub sequence_number: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "relay")]
#[derive(Debug)]
pub enum RelayServerCommand {
    StatusChanged {
        status: Option<libp2p::relay::Status>,
        resp: oneshot::Sender<Result<()>>,
    },
}

#[derive(Debug)]
pub enum WhitelistCommand {
    Add {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Remove {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    List {
        resp: oneshot::Sender<ConnexaResult<Vec<PeerId>>>,
    },
}

#[derive(Debug)]
pub enum BlacklistCommand {
    Add {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Remove {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    List {
        resp: oneshot::Sender<ConnexaResult<Vec<PeerId>>>,
    },
}

#[derive(Debug)]
pub enum PeerstoreCommand {
    Add {
        peer_id: PeerId,
        addr: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    RemoveAddress {
        peer_id: PeerId,
        addr: Multiaddr,
        resp: oneshot::Sender<ConnexaResult<()>>,
    },
    Remove {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<Vec<Multiaddr>>>,
    },
    List {
        peer_id: PeerId,
        resp: oneshot::Sender<ConnexaResult<Vec<Multiaddr>>>,
    },
    ListAll {
        resp: oneshot::Sender<ConnexaResult<PeerAddressList>>,
    },
}
