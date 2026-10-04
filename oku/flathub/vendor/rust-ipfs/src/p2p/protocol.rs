use connexa::prelude::{
    Multiaddr, PeerId, StreamProtocol,
    swarm::{
        self, ConnectionDenied, ConnectionId, FromSwarm, NetworkBehaviour, THandler,
        THandlerInEvent, ToSwarm,
    },
    transport::{Endpoint, transport::PortUse},
};

use std::convert::Infallible;
use std::task::{Context, Poll};

mod handler;

#[derive(Default, Debug)]
pub struct Behaviour {
    protocol: Vec<StreamProtocol>,
}

impl Behaviour {
    pub fn iter(&self) -> impl Iterator<Item = String> + '_ {
        self.protocol.iter().map(|s| s.to_string())
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = handler::Handler;
    type ToSwarm = Infallible;

    fn handle_pending_inbound_connection(
        &mut self,
        _: ConnectionId,
        _: &Multiaddr,
        _: &Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        Ok(())
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _: ConnectionId,
        _: Option<PeerId>,
        _: &[Multiaddr],
        _: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        Ok(vec![])
    }

    fn handle_established_inbound_connection(
        &mut self,
        _: ConnectionId,
        _: PeerId,
        _: &Multiaddr,
        _: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(handler::Handler::default())
    }

    fn handle_established_outbound_connection(
        &mut self,
        _: ConnectionId,
        _: PeerId,
        _: &Multiaddr,
        _: Endpoint,
        _: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(handler::Handler::default())
    }

    fn on_connection_handler_event(
        &mut self,
        _: PeerId,
        _: ConnectionId,
        event: swarm::THandlerOutEvent<Self>,
    ) {
        match event {
            handler::Out::Protocol(protocol) => {
                if self.protocol.ne(&protocol) {
                    self.protocol = protocol;
                }
            }
        }
    }

    fn on_swarm_event(&mut self, _: FromSwarm) {}

    fn poll(&mut self, _: &mut Context) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        Poll::Pending
    }
}
