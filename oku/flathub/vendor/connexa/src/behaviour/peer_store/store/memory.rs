use crate::behaviour::peer_store::store::Store;
use crate::prelude::swarm::derive_prelude::ConnectionEstablished;
use crate::prelude::swarm::{ConnectionClosed, FromSwarm, NewExternalAddrOfPeer};
use futures::future::{Ready, ready};
use futures::{FutureExt, StreamExt};
use futures_timer::Delay;
use indexmap::{IndexMap, IndexSet};
use libp2p::swarm::ConnectionId;
use libp2p::{Multiaddr, PeerId};
use pollable_map::futures::FutureMap;
use std::task::{Context, Poll};

const CAPACITY_CAP: usize = 256;

#[derive(Default)]
pub struct MemoryStore {
    peers: IndexMap<PeerId, IndexSet<Multiaddr>>,
    // Note: we do this to act as a "reference counter" to the same address connected to the peer
    //       before we proceed to mark the address for removal.
    connections: IndexMap<(PeerId, ConnectionId), Multiaddr>,
    persistent: IndexSet<PeerId>,
    timer: FutureMap<(PeerId, Multiaddr), Delay>,
}

impl FromIterator<(PeerId, Multiaddr)> for MemoryStore {
    fn from_iter<T: IntoIterator<Item = (PeerId, Multiaddr)>>(iter: T) -> Self {
        let mut store = Self::default();
        for (peer_id, addr) in iter {
            store.persistent.insert(peer_id);
            store.peers.entry(peer_id).or_default().insert(addr);
        }
        store
    }
}

// Note that we use this because the trait returns a future, which would allow the implementation to either use `async` to desugar to `fn -> impl Future`
// or allow custom futures (ie here we use `Ready` since this is in-memory and is expected to be ready). This, however, may change in the future.
// See https://github.com/rust-lang/rust/issues/121718 for more information
#[allow(refining_impl_trait)]
impl Store for MemoryStore {
    type Event = ();

    fn insert(&mut self, peer_id: PeerId, address: Multiaddr) -> Ready<std::io::Result<()>> {
        self.persistent.insert(peer_id);
        // remove cleanup timer since the address is manually stored.
        self.timer.remove(&(peer_id, address.clone()));

        let result = match self.peers.entry(peer_id).or_default().insert(address) {
            true => Ok(()),
            false => Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "address already exists",
            )),
        };
        ready(result)
    }

    fn remove(&mut self, peer_id: &PeerId) -> Ready<std::io::Result<Vec<Multiaddr>>> {
        let result = {
            let list = self.peers.shift_remove(peer_id);
            match list {
                Some(list) => {
                    self.persistent.shift_remove(peer_id);
                    Ok(Vec::from_iter(list))
                }
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "peer not found",
                )),
            }
        };
        ready(result)
    }

    fn remove_address(
        &mut self,
        peer_id: &PeerId,
        address: &Multiaddr,
    ) -> Ready<std::io::Result<()>> {
        let Some(list) = self.peers.get_mut(peer_id) else {
            return ready(Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "peer not found",
            )));
        };

        if !list.shift_remove(address) {
            return ready(Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "address not found",
            )));
        }

        if list.is_empty() {
            self.peers.shift_remove(peer_id);
            self.persistent.shift_remove(peer_id);
        }

        ready(Ok(()))
    }

    fn address(&self, peer_id: &PeerId) -> Ready<std::io::Result<Vec<Multiaddr>>> {
        let Some(addrs) = self.peers.get(peer_id).cloned() else {
            return ready(Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "peer not found",
            )));
        };
        ready(Ok(Vec::from_iter(addrs)))
    }

    fn list_all(&self) -> Ready<std::io::Result<Vec<(PeerId, Vec<Multiaddr>)>>> {
        let list = self
            .peers
            .iter()
            .map(|(peer_id, list)| {
                let list = Vec::from_iter(list.clone());
                (*peer_id, list)
            })
            .collect::<Vec<_>>();
        ready(Ok(list))
    }

    fn in_memory_address(&self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.peers
            .get(peer_id)
            .cloned()
            .map(Vec::from_iter)
            .unwrap_or_default()
    }

    fn on_swarm_event(&mut self, event: &FromSwarm) {
        match event {
            FromSwarm::NewExternalAddrOfPeer(NewExternalAddrOfPeer { peer_id, addr }) => {
                self.peers
                    .entry(*peer_id)
                    .or_default()
                    .insert(Multiaddr::clone(addr));
                self.persistent.insert(*peer_id);
            }
            FromSwarm::ConnectionEstablished(ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                ..
            }) => {
                // Note: because we are adding the addresses from an established connection, we will not be persisting the address unless
                //       the address is added manually.
                let remote_addr = endpoint.get_remote_address().clone();
                self.connections
                    .insert((*peer_id, *connection_id), remote_addr.clone());

                self.peers
                    .entry(*peer_id)
                    .or_default()
                    .insert(remote_addr.clone());

                self.timer.remove(&(*peer_id, remote_addr));
                // TODO: determine if we should remove any failed addresses from the store to keep the entry up to date?
            }
            FromSwarm::ConnectionClosed(ConnectionClosed {
                connection_id,
                peer_id,
                endpoint,
                ..
            }) => {
                let remote_addr = endpoint.get_remote_address();

                self.connections.shift_remove(&(*peer_id, *connection_id));

                let still_connected = self
                    .connections
                    .iter()
                    .any(|((id, _), addr)| id == peer_id && addr == remote_addr);

                if !still_connected && !self.persistent.contains(peer_id) {
                    self.timer.insert(
                        (*peer_id, remote_addr.clone()),
                        Delay::new(std::time::Duration::from_secs(60)),
                    );
                }

                if self.connections.is_empty()
                    || (self.connections.capacity() > CAPACITY_CAP
                        && self.connections.len() < CAPACITY_CAP)
                {
                    self.connections.shrink_to_fit();
                }
            }
            _ => {}
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::Event> {
        while let Poll::Ready(Some(((peer_id, addr), _))) = self.timer.poll_next_unpin(cx) {
            if let Err(e) = self
                .remove_address(&peer_id, &addr)
                .now_or_never()
                .expect("future ready")
            {
                tracing::error!(%peer_id, %addr, error = %e, "failed to remove address from store");
            }
        }
        Poll::Pending
    }
}
