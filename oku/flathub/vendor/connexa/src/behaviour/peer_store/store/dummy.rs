use crate::behaviour::peer_store::store::Store;
use crate::prelude::swarm::FromSwarm;
use crate::prelude::{Multiaddr, PeerId};
use std::task::{Context, Poll};

#[derive(Debug, Copy, Clone)]
pub struct Dummy;

impl Store for Dummy {
    type Event = ();

    fn insert(
        &mut self,
        _: PeerId,
        _: Multiaddr,
    ) -> impl Future<Output = std::io::Result<()>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn remove(
        &mut self,
        _: &PeerId,
    ) -> impl Future<Output = std::io::Result<Vec<Multiaddr>>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn remove_address(
        &mut self,
        _: &PeerId,
        _: &Multiaddr,
    ) -> impl Future<Output = std::io::Result<()>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn address(
        &self,
        _: &PeerId,
    ) -> impl Future<Output = std::io::Result<Vec<Multiaddr>>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn list_all(
        &self,
    ) -> impl Future<Output = std::io::Result<Vec<(PeerId, Vec<Multiaddr>)>>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn in_memory_address(&self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn on_swarm_event(&mut self, _: &FromSwarm) {}

    fn poll(&mut self, _: &mut Context<'_>) -> Poll<Self::Event> {
        Poll::Pending
    }
}

impl Store for () {
    type Event = ();

    fn insert(
        &mut self,
        _: PeerId,
        _: Multiaddr,
    ) -> impl Future<Output = std::io::Result<()>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn remove(
        &mut self,
        _: &PeerId,
    ) -> impl Future<Output = std::io::Result<Vec<Multiaddr>>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn remove_address(
        &mut self,
        _: &PeerId,
        _: &Multiaddr,
    ) -> impl Future<Output = std::io::Result<()>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn address(
        &self,
        _: &PeerId,
    ) -> impl Future<Output = std::io::Result<Vec<Multiaddr>>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn list_all(
        &self,
    ) -> impl Future<Output = std::io::Result<Vec<(PeerId, Vec<Multiaddr>)>>> + Send + 'static {
        futures::future::ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
    }

    fn in_memory_address(&self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn on_swarm_event(&mut self, _: &FromSwarm) {}

    fn poll(&mut self, _: &mut Context<'_>) -> Poll<Self::Event> {
        Poll::Pending
    }
}
