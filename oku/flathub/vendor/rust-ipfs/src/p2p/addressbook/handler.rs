use std::convert::Infallible;
use std::task::{Context, Poll};

use connexa::prelude::transport::upgrade::DeniedUpgrade;

use connexa::prelude::swarm::{
    ConnectionHandler, ConnectionHandlerEvent, SubstreamProtocol, handler::ConnectionEvent,
};

#[derive(Default, Debug)]
pub struct Handler {
    keep_alive: bool,
}

impl Handler {
    pub fn new(keep_alive: bool) -> Self {
        Self { keep_alive }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum In {
    Protect,
    Unprotect,
}

impl ConnectionHandler for Handler {
    type FromBehaviour = In;
    type ToBehaviour = Infallible;
    type InboundProtocol = DeniedUpgrade;
    type OutboundProtocol = DeniedUpgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol> {
        SubstreamProtocol::new(DeniedUpgrade, ())
    }

    fn connection_keep_alive(&self) -> bool {
        self.keep_alive
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        match event {
            In::Protect => {
                self.keep_alive = true;
            }
            In::Unprotect => {
                self.keep_alive = false;
            }
        }
    }

    fn on_connection_event(
        &mut self,
        _: ConnectionEvent<Self::InboundProtocol, Self::OutboundProtocol>,
    ) {
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<ConnectionHandlerEvent<Self::OutboundProtocol, (), Self::ToBehaviour>> {
        Poll::Pending
    }
}
