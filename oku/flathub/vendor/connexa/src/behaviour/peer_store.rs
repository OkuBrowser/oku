pub mod store;

use crate::behaviour::peer_store::store::Store;
use crate::behaviour::peer_store::store::memory::MemoryStore;
use crate::prelude::swarm::derive_prelude::PortUse;
use crate::prelude::swarm::{
    ConnectionDenied, ConnectionId, FromSwarm, NetworkBehaviour, THandler, THandlerInEvent,
    THandlerOutEvent, ToSwarm,
};
use crate::prelude::transport::Endpoint;
use crate::prelude::{Multiaddr, PeerId};
use futures::FutureExt;
use futures::future::BoxFuture;
use std::task::{Context, Poll, Waker};

type ListAllFuture = BoxFuture<'static, std::io::Result<Vec<(PeerId, Vec<Multiaddr>)>>>;

pub struct Behaviour<S = MemoryStore>
where
    S: 'static,
{
    store: S,
    waker: Option<Waker>,
}

impl Default for Behaviour<MemoryStore> {
    fn default() -> Self {
        Self {
            waker: None,
            store: MemoryStore::default(),
        }
    }
}

impl<S> Behaviour<S>
where
    S: Store,
{
    pub fn new(store: S) -> Self {
        Self { waker: None, store }
    }

    pub fn insert(
        &mut self,
        peer_id: PeerId,
        address: Multiaddr,
    ) -> BoxFuture<'static, std::io::Result<()>> {
        self.store.insert(peer_id, address).boxed()
    }
    pub fn remove(
        &mut self,
        peer_id: &PeerId,
    ) -> BoxFuture<'static, std::io::Result<Vec<Multiaddr>>> {
        self.store.remove(peer_id).boxed()
    }
    pub fn remove_address(
        &mut self,
        peer_id: &PeerId,
        address: &Multiaddr,
    ) -> BoxFuture<'static, std::io::Result<()>> {
        self.store.remove_address(peer_id, address).boxed()
    }
    pub fn address(&self, peer_id: &PeerId) -> BoxFuture<'static, std::io::Result<Vec<Multiaddr>>> {
        self.store.address(peer_id).boxed()
    }

    pub fn list_all(&self) -> ListAllFuture {
        self.store.list_all().boxed()
    }
}

impl<S> NetworkBehaviour for Behaviour<S>
where
    S: Store + 'static,
{
    type ConnectionHandler = super::dummy::DummyHandler;
    type ToSwarm = S::Event;

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(super::dummy::DummyHandler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &Multiaddr,
        _role_override: Endpoint,
        _port_use: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(super::dummy::DummyHandler)
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _maybe_peer: Option<PeerId>,
        _addresses: &[Multiaddr],
        _effective_role: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        let Some(peer_id) = _maybe_peer else {
            return Ok(vec![]);
        };

        let addrs = self.store.in_memory_address(&peer_id);
        Ok(addrs)
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        self.store.on_swarm_event(&event)
    }

    fn on_connection_handler_event(
        &mut self,
        _: PeerId,
        _: ConnectionId,
        _: THandlerOutEvent<Self>,
    ) {
        unreachable!("does not handle events")
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        match self.store.poll(cx) {
            Poll::Ready(ev) => Poll::Ready(ToSwarm::GenerateEvent(ev)),
            Poll::Pending => {
                self.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
