use std::convert::Infallible;
use std::task::{Context, Poll};

use indexmap::IndexMap;
use libp2p::{
    StreamProtocol,
    swarm::{NetworkBehaviour, THandlerInEvent, ToSwarm, dummy},
};

// Used to manage the index of request-response protocols.
// TODO: Implement logic to handle request-response behaviour directly
//       based on protocol name.
// Note: This may require some map-like behaviour to mimic StreamMap
#[derive(Debug)]
pub struct Behaviour {
    rr_list: IndexMap<StreamProtocol, usize>,
}

impl Behaviour {
    pub fn new(list: impl IntoIterator<Item = (StreamProtocol, usize)>) -> Self {
        let rr_list = IndexMap::from_iter(list);
        Self { rr_list }
    }

    pub fn get_protocol(&self, protocol: StreamProtocol) -> Option<usize> {
        self.rr_list.get(&protocol).copied()
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = dummy::ConnectionHandler;
    type ToSwarm = Infallible;

    fn handle_pending_inbound_connection(
        &mut self,
        _: libp2p::swarm::ConnectionId,
        _: &libp2p::Multiaddr,
        _: &libp2p::Multiaddr,
    ) -> Result<(), libp2p::swarm::ConnectionDenied> {
        Ok(())
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _: libp2p::swarm::ConnectionId,
        _: Option<libp2p::PeerId>,
        _: &[libp2p::Multiaddr],
        _: libp2p::core::Endpoint,
    ) -> Result<Vec<libp2p::Multiaddr>, libp2p::swarm::ConnectionDenied> {
        Ok(std::vec![])
    }

    fn handle_established_inbound_connection(
        &mut self,
        _: libp2p::swarm::ConnectionId,
        _: libp2p::PeerId,
        _: &libp2p::Multiaddr,
        _: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(dummy::ConnectionHandler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _: libp2p::swarm::ConnectionId,
        _: libp2p::PeerId,
        _: &libp2p::Multiaddr,
        _: libp2p::core::Endpoint,
        _: libp2p::core::transport::PortUse,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(dummy::ConnectionHandler)
    }

    fn on_swarm_event(&mut self, _: libp2p::swarm::FromSwarm) {}

    fn on_connection_handler_event(
        &mut self,
        _: libp2p::PeerId,
        _: libp2p::swarm::ConnectionId,
        _: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
    }

    fn poll(&mut self, _: &mut Context) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        Poll::Pending
    }
}
