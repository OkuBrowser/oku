pub mod dummy;
pub mod memory;

use libp2p::swarm::FromSwarm;
use libp2p::{Multiaddr, PeerId};
use std::fmt::Debug;
use std::task::{Context, Poll};

pub trait Store: Send + Sync + 'static {
    type Event: Debug + Send;

    /// Insert a new peer and associated address into the store
    fn insert(
        &mut self,
        peer_id: PeerId,
        address: Multiaddr,
    ) -> impl Future<Output = std::io::Result<()>> + Send + 'static;

    /// Remove a peer and all its associated addresses from the store
    fn remove(
        &mut self,
        peer_id: &PeerId,
    ) -> impl Future<Output = std::io::Result<Vec<Multiaddr>>> + Send + 'static;

    /// Remove a specific address associated with a peer from the store
    fn remove_address(
        &mut self,
        peer_id: &PeerId,
        address: &Multiaddr,
    ) -> impl Future<Output = std::io::Result<()>> + Send + 'static;

    /// Get all addresses associated with a peer
    fn address(
        &self,
        peer_id: &PeerId,
    ) -> impl Future<Output = std::io::Result<Vec<Multiaddr>>> + Send + 'static;

    /// Get all addresses in the peer store
    fn list_all(
        &self,
    ) -> impl Future<Output = std::io::Result<Vec<(PeerId, Vec<Multiaddr>)>>> + Send + 'static;

    /// Get all in-memory address associated with a peer.
    /// Note that this function assumes that the addresses are stored in memory
    fn in_memory_address(&self, peer_id: &PeerId) -> Vec<Multiaddr>;

    /// Handle swarm events to update the store state
    fn on_swarm_event(&mut self, event: &FromSwarm);

    /// Poll the store for events
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::Event>;
}
