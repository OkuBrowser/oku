use connexa::prelude::{
    StreamProtocol,
    swarm::{
        ConnectionHandler, ConnectionHandlerEvent, SubstreamProtocol, SupportedProtocols,
        handler::ConnectionEvent,
    },
    transport::upgrade::DeniedUpgrade,
};

use std::{
    collections::VecDeque,
    task::{Context, Poll},
};

#[allow(clippy::type_complexity)]
#[allow(deprecated)]
#[derive(Default, Debug)]
pub struct Handler {
    events: VecDeque<
        ConnectionHandlerEvent<
            <Self as ConnectionHandler>::OutboundProtocol,
            <Self as ConnectionHandler>::OutboundOpenInfo,
            <Self as ConnectionHandler>::ToBehaviour,
        >,
    >,
    supported_protocol: SupportedProtocols,
}

#[derive(Debug, Clone)]
pub enum Out {
    Protocol(Vec<StreamProtocol>),
}

#[allow(deprecated)]
impl ConnectionHandler for Handler {
    type FromBehaviour = ();
    type ToBehaviour = Out;
    type InboundProtocol = DeniedUpgrade;
    type OutboundProtocol = DeniedUpgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(DeniedUpgrade, ())
    }

    fn connection_keep_alive(&self) -> bool {
        false
    }

    fn on_behaviour_event(&mut self, _: Self::FromBehaviour) {}

    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        if let ConnectionEvent::LocalProtocolsChange(protocol) = event {
            let change = self.supported_protocol.on_protocols_change(protocol);
            if change {
                self.events
                    .push_back(ConnectionHandlerEvent::NotifyBehaviour(Out::Protocol(
                        self.supported_protocol.iter().cloned().collect(),
                    )));
            }
        }
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::ToBehaviour>,
    > {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }
        Poll::Pending
    }
}
