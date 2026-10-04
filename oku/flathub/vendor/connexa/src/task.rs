#![allow(unused_imports)]

#[cfg(feature = "autonat")]
mod autonat;
#[cfg(feature = "dcutr")]
#[cfg(not(target_arch = "wasm32"))]
mod dcutr;
#[cfg(feature = "floodsub")]
mod floodsub;
#[cfg(feature = "gossipsub")]
mod gossipsub;
#[cfg(feature = "identify")]
mod identify;
#[cfg(feature = "kad")]
mod kad;
#[cfg(feature = "mdns")]
#[cfg(not(target_arch = "wasm32"))]
mod mdns;
#[cfg(feature = "ping")]
mod ping;
#[cfg(feature = "relay")]
mod relay;

#[cfg(feature = "rendezvous")]
mod rendezvous;
#[cfg(feature = "request-response")]
mod request_response;
#[cfg(feature = "stream")]
mod stream;
mod swarm;
#[cfg(feature = "upnp")]
#[cfg(not(target_arch = "wasm32"))]
mod upnp;

use crate::behaviour::BehaviourEvent;
use crate::error::{ConnexaResult, Error, Protocol};
use crate::keystore::{Keychain, store::memory::MemoryKeystore};
use crate::types::{Command, ConnexaSwarmEvent, SwarmCommand};
use crate::{TEventCallback, TPollableCallback, TSwarmEventCallback, TTaskCallback, behaviour};

#[cfg(feature = "kad")]
use crate::types::DHTEvent;
#[cfg(feature = "floodsub")]
use crate::types::FloodsubEvent;

use crate::behaviour::peer_store::store::Store;
use crate::handle::swarm::ConnectionTarget;
use crate::prelude::PeerstoreCommand;
#[cfg(feature = "gossipsub")]
use crate::types::GossipsubEvent;
use crate::types::{BlacklistCommand, ConnectionLimitsCommand, WhitelistCommand};
use futures::channel::{mpsc, oneshot};
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use futures_timer::Delay;
use indexmap::{IndexMap, IndexSet};
#[cfg(feature = "gossipsub")]
use libp2p::gossipsub::{MessageAcceptance, MessageId};
#[cfg(feature = "kad")]
use libp2p::kad::store::RecordStore;
#[cfg(feature = "kad")]
use libp2p::kad::{PeerInfo, PeerRecord, ProviderRecord, QueryId, Record, RecordKey as Key};
#[cfg(feature = "rendezvous")]
use libp2p::rendezvous::Namespace;
use libp2p::swarm::derive_prelude::ListenerId;
use libp2p::swarm::{ConnectionId, NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, PeerId, Swarm};
use pollable_map::futures::FutureMap;
use pollable_map::futures::set::FutureSet;
use pollable_map::optional::Optional;
use pollable_map::stream::StreamMap;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

#[cfg(feature = "rendezvous")]
type RendezvousDiscoverResponse =
    ConnexaResult<(libp2p::rendezvous::Cookie, Vec<(PeerId, Vec<Multiaddr>)>)>;

#[cfg(feature = "rendezvous")]
type RendezvousDiscoverRequest = (
    Option<libp2p::rendezvous::Cookie>,
    Option<u64>,
    oneshot::Sender<RendezvousDiscoverResponse>,
);

#[cfg(feature = "rendezvous")]
type RendezvousRegisterRequest = (Option<u64>, oneshot::Sender<ConnexaResult<()>>);

pub struct ConnexaTask<X, C: NetworkBehaviour, S, T = (), K = MemoryKeystore>
where
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub swarm: Optional<Swarm<behaviour::Behaviour<C, S>>>,
    pub command_receiver: Optional<mpsc::Receiver<Command<T>>>,
    pub context: X,
    pub custom_task_callback: TTaskCallback<C, X, T, S, K>,
    pub custom_event_callback: TEventCallback<C, X, S, K>,
    pub swarm_event_callback: TSwarmEventCallback<C, X, S, K>,
    pub custom_pollable_callback: TPollableCallback<C, X, S, K>,
    pub keychain: Keychain<K>,

    pub connection_listeners: Vec<mpsc::Sender<ConnexaSwarmEvent>>,

    pub listener_addresses: HashMap<ListenerId, Vec<Multiaddr>>,

    /// A listener for sending dht events
    #[cfg(feature = "kad")]
    pub dht_event_sender: IndexMap<Key, Vec<mpsc::Sender<DHTEvent>>>,
    #[cfg(feature = "kad")]
    pub dht_event_global_sender: Vec<mpsc::Sender<DHTEvent>>,

    #[cfg(feature = "kad")]
    pub dht_put_record_receiver: StreamMap<Key, FutureSet<oneshot::Receiver<Record>>>,
    #[cfg(feature = "kad")]
    pub dht_put_record_global_receiver: FutureSet<oneshot::Receiver<Record>>,
    #[cfg(feature = "kad")]
    pub dht_provider_record_receiver: StreamMap<Key, FutureSet<oneshot::Receiver<ProviderRecord>>>,
    #[cfg(feature = "kad")]
    pub dht_provider_record_global_receiver: FutureSet<oneshot::Receiver<ProviderRecord>>,
    #[cfg(feature = "kad")]
    pub pending_dht_put_record: IndexMap<QueryId, oneshot::Sender<ConnexaResult<()>>>,
    #[cfg(feature = "kad")]
    pub pending_dht_put_provider_record: IndexMap<QueryId, oneshot::Sender<ConnexaResult<()>>>,
    #[cfg(feature = "kad")]
    pub pending_dht_get_record: IndexMap<QueryId, mpsc::Sender<ConnexaResult<PeerRecord>>>,
    #[cfg(feature = "kad")]
    pub pending_dht_get_provider_record:
        IndexMap<QueryId, mpsc::Sender<ConnexaResult<HashSet<PeerId>>>>,
    #[cfg(feature = "kad")]
    pub pending_dht_find_closest_peer:
        IndexMap<QueryId, oneshot::Sender<ConnexaResult<Vec<PeerInfo>>>>,

    #[cfg(feature = "kad")]
    pub pending_dht_bootstrap: IndexMap<QueryId, oneshot::Sender<ConnexaResult<()>>>,

    pub pending_connection: IndexMap<ConnectionId, oneshot::Sender<ConnexaResult<ConnectionId>>>,
    pub pending_disconnection_by_connection_id:
        IndexMap<ConnectionId, oneshot::Sender<ConnexaResult<()>>>,
    pub pending_disconnection_by_peer_id: IndexMap<PeerId, oneshot::Sender<ConnexaResult<()>>>,

    pub pending_listen_on: IndexMap<ListenerId, oneshot::Sender<ConnexaResult<ListenerId>>>,
    pub pending_remove_listener: IndexMap<ListenerId, oneshot::Sender<ConnexaResult<()>>>,

    #[cfg(feature = "gossipsub")]
    pub gossipsub_listener:
        IndexMap<libp2p::gossipsub::TopicHash, Vec<mpsc::Sender<GossipsubEvent>>>,
    #[cfg(feature = "floodsub")]
    pub floodsub_listener: IndexMap<libp2p::floodsub::Topic, Vec<mpsc::Sender<FloodsubEvent>>>,

    #[cfg(feature = "rendezvous")]
    pub pending_rendezvous_register:
        IndexMap<(PeerId, Namespace), VecDeque<RendezvousRegisterRequest>>,

    #[cfg(feature = "rendezvous")]
    pub pending_rendezvous_discover:
        IndexMap<(PeerId, Option<Namespace>), VecDeque<RendezvousDiscoverRequest>>,

    #[cfg(feature = "gossipsub")]
    pub gossipsub_can_propagate:
        FutureMap<(PeerId, MessageId), oneshot::Receiver<std::io::Result<MessageAcceptance>>>,

    pub peerstore_ops: FutureSet<BoxFuture<'static, ()>>,

    pub cleanup_timer: Delay,
    pub cleanup_interval: Duration,
}

impl<X, C: NetworkBehaviour, S, T, K> ConnexaTask<X, C, S, T, K>
where
    X: Default + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub fn new(swarm: Swarm<behaviour::Behaviour<C, S>>, keychain: Keychain<K>) -> Self {
        let duration = Duration::from_secs(10);
        Self {
            swarm: Optional::new(swarm),
            context: X::default(),
            command_receiver: Optional::default(),
            custom_event_callback: Box::new(|_, _, _, _| ()),
            custom_task_callback: Box::new(|_, _, _, _| ()),
            custom_pollable_callback: Box::new(|_, _, _, _| Poll::Pending),
            swarm_event_callback: Box::new(|_, _, _, _| ()),
            keychain,
            connection_listeners: Vec::new(),
            listener_addresses: HashMap::new(),
            #[cfg(feature = "kad")]
            dht_event_sender: Default::default(),
            #[cfg(feature = "kad")]
            dht_event_global_sender: vec![],
            #[cfg(feature = "kad")]
            dht_put_record_receiver: StreamMap::new(),
            #[cfg(feature = "kad")]
            dht_put_record_global_receiver: Default::default(),
            #[cfg(feature = "kad")]
            dht_provider_record_receiver: StreamMap::new(),
            #[cfg(feature = "kad")]
            dht_provider_record_global_receiver: Default::default(),
            #[cfg(feature = "kad")]
            pending_dht_put_record: Default::default(),
            #[cfg(feature = "kad")]
            pending_dht_put_provider_record: IndexMap::new(),
            #[cfg(feature = "kad")]
            pending_dht_get_record: Default::default(),
            #[cfg(feature = "kad")]
            pending_dht_get_provider_record: Default::default(),
            #[cfg(feature = "kad")]
            pending_dht_find_closest_peer: Default::default(),
            #[cfg(feature = "kad")]
            pending_dht_bootstrap: Default::default(),
            peerstore_ops: Default::default(),
            cleanup_timer: Delay::new(duration),
            cleanup_interval: duration,
            pending_connection: IndexMap::new(),
            pending_disconnection_by_peer_id: IndexMap::new(),
            pending_disconnection_by_connection_id: IndexMap::new(),
            pending_listen_on: IndexMap::new(),
            pending_remove_listener: IndexMap::new(),
            #[cfg(feature = "floodsub")]
            floodsub_listener: Default::default(),
            #[cfg(feature = "gossipsub")]
            gossipsub_listener: Default::default(),
            #[cfg(feature = "gossipsub")]
            gossipsub_can_propagate: Default::default(),
            #[cfg(feature = "rendezvous")]
            pending_rendezvous_discover: Default::default(),
            #[cfg(feature = "rendezvous")]
            pending_rendezvous_register: Default::default(),
        }
    }

    pub fn set_context(&mut self, context: X) {
        self.context = context;
    }

    pub fn set_command_receiver(&mut self, command_receiver: mpsc::Receiver<Command<T>>) {
        self.command_receiver.replace(command_receiver);
    }

    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(&mut Swarm<behaviour::Behaviour<C, S>>, &Keychain<K>, &mut X, C::ToSwarm)
            + Send
            + 'static,
    {
        self.custom_event_callback = Box::new(callback);
    }

    pub fn set_task_callback<F>(&mut self, callback: F)
    where
        F: Fn(&mut Swarm<behaviour::Behaviour<C, S>>, &Keychain<K>, &mut X, T) + Send + 'static,
    {
        self.custom_task_callback = Box::new(callback);
    }

    pub fn set_swarm_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(
                &mut Swarm<behaviour::Behaviour<C, S>>,
                &Keychain<K>,
                &SwarmEvent<BehaviourEvent<C, S>>,
                &mut X,
            )
            + 'static
            + Send,
    {
        self.swarm_event_callback = Box::new(callback);
    }

    pub fn set_pollable_callback<F>(&mut self, callback: F)
    where
        F: Fn(
                &mut Context<'_>,
                &mut Swarm<behaviour::Behaviour<C, S>>,
                &Keychain<K>,
                &mut X,
            ) -> Poll<()>
            + Send
            + 'static,
    {
        self.custom_pollable_callback = Box::new(callback);
    }

    pub fn process_command(&mut self, command: Command<T>) {
        let Some(swarm) = self.swarm.as_mut() else {
            return;
        };
        match command {
            Command::Swarm(swarm_command) => match swarm_command {
                SwarmCommand::Listener { resp } => {
                    // TODO: Make buffer adjustable via config
                    let (tx, rx) = mpsc::channel(50);
                    self.connection_listeners.push(tx);
                    let _ = resp.send(rx);
                }
                SwarmCommand::Dial { opt, resp } => {
                    let connection_id = opt.connection_id();
                    if let Err(e) = swarm.dial(opt) {
                        let _ = resp.send(Err(Error::Dial(other_error::ArcError::new(e))));
                        return;
                    }
                    self.pending_connection.insert(connection_id, resp);
                }
                SwarmCommand::IsConnected { peer_id, resp } => {
                    let is_connected = swarm.is_connected(&peer_id);
                    let _ = resp.send(is_connected);
                }
                SwarmCommand::Disconnect { target_type, resp } => match target_type {
                    ConnectionTarget::PeerId(peer_id) => {
                        if swarm.disconnect_peer_id(peer_id).is_err() {
                            let _ = resp.send(Err(Error::NotConnected(peer_id)));
                            return;
                        }
                        self.pending_disconnection_by_peer_id.insert(peer_id, resp);
                    }
                    ConnectionTarget::ConnectionId(connection_id) => {
                        if !swarm.close_connection(connection_id) {
                            let _ = resp.send(Err(Error::NotFound("connection".into())));
                            return;
                        }
                        self.pending_disconnection_by_connection_id
                            .insert(connection_id, resp);
                    }
                },
                SwarmCommand::ConnectedPeers { resp } => {
                    let connected_peers = swarm.connected_peers();
                    let _ = resp.send(connected_peers.copied().collect());
                }
                SwarmCommand::ListenOn { address, resp } => {
                    let id = match swarm.listen_on(address) {
                        Ok(id) => id,
                        Err(e) => {
                            let _ = resp.send(Err(std::io::Error::other(e).into()));
                            return;
                        }
                    };
                    self.pending_listen_on.insert(id, resp);
                }
                SwarmCommand::GetListeningAddress { id, resp } => {
                    let Some(addrs) = self.listener_addresses.get(&id) else {
                        let _ = resp.send(Err(Error::NotFound("listener".into())));
                        return;
                    };

                    let _ = resp.send(Ok(addrs.clone()));
                }
                SwarmCommand::RemoveListener { listener_id, resp } => {
                    if !swarm.remove_listener(listener_id) {
                        let _ = resp.send(Err(Error::NotFound("listener".into())));
                        return;
                    }
                    self.listener_addresses.remove(&listener_id);
                    self.pending_remove_listener.insert(listener_id, resp);
                }
                SwarmCommand::AddExternalAddress { address, resp } => {
                    swarm.add_external_address(address);
                    let _ = resp.send(Ok(()));
                }
                SwarmCommand::RemoveExternalAddress { address, resp } => {
                    swarm.remove_external_address(&address);
                    let _ = resp.send(Ok(()));
                }
                SwarmCommand::ListExternalAddresses { resp } => {
                    let addresses = swarm.external_addresses().cloned().collect();
                    let _ = resp.send(addresses);
                }
                SwarmCommand::ListListeningAddresses { resp } => {
                    let addresses = swarm.listeners().cloned().collect();
                    let _ = resp.send(addresses);
                }
                SwarmCommand::AddPeerAddress {
                    peer_id,
                    address,
                    resp,
                } => {
                    swarm.add_peer_address(peer_id, address);
                    let _ = resp.send(Ok(()));
                }
            },
            Command::Whitelist(command) => match command {
                WhitelistCommand::Add { peer_id, resp } => {
                    let Some(whitelist) = swarm.behaviour_mut().allow_list.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("whitelist"),
                        }));
                        return;
                    };

                    if !whitelist.allow_peer(peer_id) {
                        let _ = resp.send(Err(Error::AlreadyExists("peer in allow list".into())));
                        return;
                    }

                    let _ = resp.send(Ok(()));
                }
                WhitelistCommand::Remove { peer_id, resp } => {
                    let Some(whitelist) = swarm.behaviour_mut().allow_list.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("whitelist"),
                        }));
                        return;
                    };

                    if !whitelist.disallow_peer(peer_id) {
                        let _ = resp.send(Err(Error::NotFound("peer in allow list".into())));
                        return;
                    }

                    let _ = resp.send(Ok(()));
                }
                WhitelistCommand::List { resp } => {
                    let Some(whitelist) = swarm.behaviour_mut().allow_list.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("whitelist"),
                        }));
                        return;
                    };

                    let list = whitelist.allowed_peers();
                    let list = list.iter().cloned().collect();

                    let _ = resp.send(Ok(list));
                }
            },
            Command::Blacklist(command) => match command {
                BlacklistCommand::Add { peer_id, resp } => {
                    let Some(blacklist) = swarm.behaviour_mut().deny_list.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("blacklist"),
                        }));
                        return;
                    };

                    if !blacklist.block_peer(peer_id) {
                        let _ = resp.send(Err(Error::AlreadyExists("peer in deny list".into())));
                        return;
                    }

                    let _ = resp.send(Ok(()));
                }
                BlacklistCommand::Remove { peer_id, resp } => {
                    let Some(blacklist) = swarm.behaviour_mut().deny_list.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("blacklist"),
                        }));
                        return;
                    };

                    if !blacklist.unblock_peer(peer_id) {
                        let _ = resp.send(Err(Error::NotFound("peer in deny list".into())));
                        return;
                    }

                    let _ = resp.send(Ok(()));
                }
                BlacklistCommand::List { resp } => {
                    let Some(blacklist) = swarm.behaviour_mut().deny_list.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("blacklist"),
                        }));
                        return;
                    };

                    let list = blacklist.blocked_peers();
                    let list = list.iter().cloned().collect();

                    let _ = resp.send(Ok(list));
                }
            },
            Command::ConnectionLimits(command) => match command {
                ConnectionLimitsCommand::Get { resp } => {
                    let Some(connection_limits) = swarm.behaviour_mut().connection_limits.as_mut()
                    else {
                        let _ =
                            resp.send(Err(std::io::Error::other("connection limits not enabled")));
                        return;
                    };

                    let limits = connection_limits.limits_mut();
                    let _ = resp.send(Ok(limits.clone()));
                }
                ConnectionLimitsCommand::Set { limits, resp } => {
                    let Some(connection_limits) = swarm.behaviour_mut().connection_limits.as_mut()
                    else {
                        let _ =
                            resp.send(Err(std::io::Error::other("connection limits not enabled")));
                        return;
                    };

                    *connection_limits.limits_mut() = limits;

                    let _ = resp.send(Ok(()));
                }
            },
            Command::Peerstore(command) => match command {
                PeerstoreCommand::Add {
                    peer_id,
                    addr,
                    resp,
                } => {
                    let Some(store) = swarm.behaviour_mut().peer_store.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("peerstore"),
                        }));
                        return;
                    };

                    let fut = store.insert(peer_id, addr);
                    self.peerstore_ops.insert(Box::pin(async move {
                        let _ = resp.send(fut.await.map_err(Error::from));
                    }));
                }
                PeerstoreCommand::RemoveAddress {
                    peer_id,
                    addr,
                    resp,
                } => {
                    let Some(store) = swarm.behaviour_mut().peer_store.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("peerstore"),
                        }));
                        return;
                    };
                    let fut = store.remove_address(&peer_id, &addr);
                    self.peerstore_ops.insert(Box::pin(async move {
                        let _ = resp.send(fut.await.map_err(Error::from));
                    }));
                }
                PeerstoreCommand::Remove { peer_id, resp } => {
                    let Some(store) = swarm.behaviour_mut().peer_store.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("peerstore"),
                        }));
                        return;
                    };
                    let fut = store.remove(&peer_id);
                    self.peerstore_ops.insert(Box::pin(async move {
                        let _ = resp.send(fut.await.map_err(Error::from));
                    }));
                }
                PeerstoreCommand::List { peer_id, resp } => {
                    let Some(store) = swarm.behaviour_mut().peer_store.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("peerstore"),
                        }));
                        return;
                    };
                    let fut = store.address(&peer_id);
                    self.peerstore_ops.insert(Box::pin(async move {
                        let _ = resp.send(fut.await.map_err(Error::from));
                    }));
                }
                PeerstoreCommand::ListAll { resp } => {
                    let Some(store) = swarm.behaviour_mut().peer_store.as_mut() else {
                        let _ = resp.send(Err(Error::Disabled {
                            protocol: Protocol::Other("peerstore"),
                        }));
                        return;
                    };
                    let fut = store.list_all();
                    self.peerstore_ops.insert(Box::pin(async move {
                        let _ = resp.send(fut.await.map_err(Error::from));
                    }));
                }
            },
            #[cfg(feature = "gossipsub")]
            Command::Gossipsub(command) => self.process_gossipsub_command(command),
            #[cfg(feature = "floodsub")]
            Command::Floodsub(command) => self.process_floodsub_command(command),
            #[cfg(feature = "autonat")]
            Command::Autonat(autonat_command) => self.process_autonat_v1_command(autonat_command),
            #[cfg(feature = "kad")]
            Command::Dht(dht_command) => self.process_kademlia_command(dht_command),
            #[cfg(feature = "relay")]
            Command::AutoRelay(autorelay_command) => {
                self.process_autorelay_commands(autorelay_command)
            }
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(feature = "relay")]
            Command::RelayServer(relay_server_command) => {
                self.process_relay_server_command(relay_server_command)
            }

            #[cfg(feature = "stream")]
            Command::Stream(stream_command) => self.process_stream_command(stream_command),
            #[cfg(feature = "request-response")]
            Command::RequestResponse(request_response_command) => {
                self.process_request_response_command(request_response_command)
            }
            #[cfg(feature = "rendezvous")]
            Command::Rendezvous(rendezvous_command) => {
                self.process_rendezvous_command(rendezvous_command)
            }
            Command::Custom(custom_command) => {
                (self.custom_task_callback)(
                    swarm,
                    &self.keychain,
                    &mut self.context,
                    custom_command,
                );
            }
        }
    }
}

impl<X, C: NetworkBehaviour, S, T, K> Future for ConnexaTask<X, C, S, T, K>
where
    X: Default + Unpin + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    T: 'static,
    S: Store,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // We should terminate the future if swarm stream is done being polled
        if self.swarm.is_none() {
            return Poll::Ready(());
        }

        if self.cleanup_timer.poll_unpin(cx).is_ready() {
            let interval = self.cleanup_interval;
            self.cleanup_timer.reset(interval);
            #[cfg(feature = "gossipsub")]
            self.gossipsub_listener.retain(|_, v| {
                v.retain(|ch| !ch.is_closed());
                !v.is_empty()
            });

            #[cfg(feature = "floodsub")]
            self.floodsub_listener.retain(|_, v| {
                v.retain(|ch| !ch.is_closed());
                !v.is_empty()
            });

            #[cfg(feature = "kad")]
            self.dht_event_sender.retain(|_, v| {
                v.retain(|ch| !ch.is_closed());
                !v.is_empty()
            });

            #[cfg(feature = "kad")]
            self.dht_event_global_sender.retain(|ch| !ch.is_closed());
            self.connection_listeners.retain(|ch| !ch.is_closed());
        }

        while let Poll::Ready(Some(command)) = self.command_receiver.poll_next_unpin(cx) {
            self.process_command(command);
        }

        // Note: could probably poll in burst instead so we could continue making progress in this future
        //       while reducing long activities
        loop {
            match self.swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => self.process_swarm_event(event),
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => break,
            }
        }

        {
            let this = &mut *self;
            if let Some(swarm) = this.swarm.as_mut() {
                let _ =
                    (this.custom_pollable_callback)(cx, swarm, &this.keychain, &mut this.context);
            }
        }

        while let Poll::Ready(Some(())) = self.peerstore_ops.poll_next_unpin(cx) {}

        #[cfg(feature = "gossipsub")]
        while let Poll::Ready(Some(((propagation_source, message_id), result))) =
            self.gossipsub_can_propagate.poll_next_unpin(cx)
        {
            match result {
                Ok(Ok(acceptance)) => {
                    tracing::trace!(?acceptance, "received message acceptance");
                    let gossipsub = self
                        .swarm
                        .as_mut()
                        .unwrap()
                        .behaviour_mut()
                        .gossipsub
                        .as_mut()
                        .unwrap();

                    if !gossipsub.report_message_validation_result(
                        &message_id,
                        &propagation_source,
                        acceptance,
                    ) {
                        tracing::warn!(
                            ?propagation_source,
                            ?message_id,
                            "message not in cache. ignoring..."
                        );
                        continue;
                    }
                    tracing::trace!(
                        ?propagation_source,
                        ?message_id,
                        "reported message validation"
                    );
                }
                Ok(Err(e)) => {
                    tracing::error!(
                        ?propagation_source,
                        ?message_id,
                        ?e,
                        "failed to report message validation"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::error!(
                        ?propagation_source,
                        ?message_id,
                        ?e,
                        "failed to report message validation"
                    );
                }
            }
        }

        #[cfg(feature = "kad")]
        while let Poll::Ready(Some((key, result))) =
            self.dht_put_record_receiver.poll_next_unpin(cx)
        {
            let record = match result {
                Ok(record) => record,
                Err(_) => {
                    tracing::debug!(?key, "dht put record was not accepted; skipping");
                    continue;
                }
            };

            tracing::trace!(?key, ?record, "dht put record result");
            let swarm = self.swarm.as_mut().expect("swarm is available");

            let kad = swarm
                .behaviour_mut()
                .kademlia
                .as_mut()
                .expect("kad is available");

            match kad.store_mut().put(record) {
                Ok(_) => tracing::info!(?key, "dht put record success"),
                Err(e) => tracing::error!(?key, ?e, "dht put record failed"),
            }
        }

        #[cfg(feature = "kad")]
        while let Poll::Ready(Some(result)) =
            self.dht_put_record_global_receiver.poll_next_unpin(cx)
        {
            let record = match result {
                Ok(record) => record,
                Err(_) => {
                    tracing::debug!("dht put record was not accepted; skipping");
                    continue;
                }
            };

            let key = record.key.clone();
            tracing::trace!(?key, ?record, "dht put record result");
            let swarm = self.swarm.as_mut().expect("swarm is available");

            let kad = swarm
                .behaviour_mut()
                .kademlia
                .as_mut()
                .expect("kad is available");

            match kad.store_mut().put(record) {
                Ok(_) => tracing::info!(?key, "dht put record success"),
                Err(e) => tracing::error!(?key, ?e, "dht put record failed"),
            }
        }

        #[cfg(feature = "kad")]
        while let Poll::Ready(Some((key, result))) =
            self.dht_provider_record_receiver.poll_next_unpin(cx)
        {
            let record = match result {
                Ok(record) => record,
                Err(_) => {
                    tracing::debug!(?key, "dht provider record was not accepted; skipping");
                    continue;
                }
            };

            tracing::trace!(?key, ?record, "dht provider record result");

            let swarm = self.swarm.as_mut().expect("swarm is available");

            let kad = swarm
                .behaviour_mut()
                .kademlia
                .as_mut()
                .expect("kad is available");

            match kad.store_mut().add_provider(record) {
                Ok(_) => tracing::info!(?key, "dht add provider record success"),
                Err(e) => tracing::error!(?key, ?e, "dht add provider record failed"),
            }
        }

        #[cfg(feature = "kad")]
        while let Poll::Ready(Some(result)) =
            self.dht_provider_record_global_receiver.poll_next_unpin(cx)
        {
            let record = match result {
                Ok(record) => record,
                Err(_) => {
                    tracing::debug!("dht provider record was not accepted; skipping");
                    continue;
                }
            };

            let key = record.key.clone();

            tracing::trace!(?key, ?record, "dht provider record result");

            let swarm = self.swarm.as_mut().expect("swarm is available");

            let kad = swarm
                .behaviour_mut()
                .kademlia
                .as_mut()
                .expect("kad is available");

            match kad.store_mut().add_provider(record) {
                Ok(_) => tracing::info!(?key, "dht add provider record success"),
                Err(e) => tracing::error!(?key, ?e, "dht add provider record failed"),
            }
        }

        Poll::Pending
    }
}
