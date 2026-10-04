use std::collections::VecDeque;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use asynchronous_codec::Framed;
use connexa::prelude::swarm::handler::{
    ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
    StreamUpgradeError,
};
use connexa::prelude::swarm::{ConnectionHandler, ConnectionHandlerEvent, SubstreamProtocol};
use connexa::prelude::transport::upgrade::ReadyUpgrade;
use connexa::prelude::{Stream, StreamProtocol};
use futures::{SinkExt, StreamExt};

use super::{bitswap_pb, message::BitswapMessage};

const PROTOCOL: StreamProtocol = StreamProtocol::new("/ipfs/bitswap/1.2.0");
const MAX_BUF_SIZE: usize = 2_097_152;
const WRITE_ERROR_THRESHOLD: usize = 3;
const WRITE_ERROR_WINDOW: Duration = Duration::from_secs(30);

type Codec = prost_codec::Codec<bitswap_pb::Message>;
type Channel = Framed<Stream, Codec>;

#[derive(Debug)]
pub enum FromBehaviour {
    Send(BitswapMessage),
}

#[derive(Debug)]
pub enum ToBehaviour {
    Ready,
    Unsupported,
    Failed,
    Message(BitswapMessage),
}

enum Outbound {
    None,
    Requested,
    Ready(Box<Channel>),
    Unsupported,
    Failed,
}

pub struct Handler {
    outbound: Outbound,
    inbound: Option<Box<Channel>>,
    outbox: VecDeque<BitswapMessage>,
    write_errors: VecDeque<Instant>,
    pending: VecDeque<ToBehaviour>,
}

impl Default for Handler {
    fn default() -> Self {
        Self {
            outbound: Outbound::None,
            inbound: None,
            outbox: VecDeque::new(),
            write_errors: VecDeque::new(),
            pending: VecDeque::new(),
        }
    }
}

fn note_write_error(write_errors: &mut VecDeque<Instant>) -> bool {
    let now = Instant::now();
    while let Some(&front) = write_errors.front() {
        if now.duration_since(front) > WRITE_ERROR_WINDOW {
            write_errors.pop_front();
        } else {
            break;
        }
    }
    write_errors.push_back(now);
    write_errors.len() >= WRITE_ERROR_THRESHOLD
}

impl ConnectionHandler for Handler {
    type FromBehaviour = FromBehaviour;
    type ToBehaviour = ToBehaviour;
    type InboundProtocol = ReadyUpgrade<StreamProtocol>;
    type OutboundProtocol = ReadyUpgrade<StreamProtocol>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL), ())
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        match event {
            FromBehaviour::Send(message) => self.outbox.push_back(message),
        }
    }

    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol: stream,
                ..
            }) => {
                self.inbound = Some(Box::new(Framed::new(stream, Codec::new(MAX_BUF_SIZE))));
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol: stream,
                ..
            }) => {
                self.outbound =
                    Outbound::Ready(Box::new(Framed::new(stream, Codec::new(MAX_BUF_SIZE))));
                self.pending.push_back(ToBehaviour::Ready);
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError { error, .. }) => {
                if matches!(error, StreamUpgradeError::NegotiationFailed) {
                    self.outbound = Outbound::Unsupported;
                    self.pending.push_back(ToBehaviour::Unsupported);
                } else {
                    // transient; poll re-requests
                    self.outbound = Outbound::None;
                }
            }
            _ => {}
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::ToBehaviour>,
    > {
        if let Some(event) = self.pending.pop_front() {
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(event));
        }

        if matches!(self.outbound, Outbound::None) {
            self.outbound = Outbound::Requested;
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL), ()),
            });
        }

        let mut write_failed = false;
        if let Outbound::Ready(stream) = &mut self.outbound {
            while !self.outbox.is_empty() {
                match stream.poll_ready_unpin(cx) {
                    Poll::Ready(Ok(())) => {
                        let message = self.outbox.pop_front().expect("non-empty");
                        match message.into_proto() {
                            Ok(proto) => {
                                if stream.start_send_unpin(proto).is_err() {
                                    write_failed = note_write_error(&mut self.write_errors);
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::debug!(error=%e, "dropping unserializable bitswap message")
                            }
                        }
                    }
                    Poll::Ready(Err(_)) => {
                        write_failed = note_write_error(&mut self.write_errors);
                        break;
                    }
                    Poll::Pending => break,
                }
            }
            if !write_failed {
                let _ = stream.poll_flush_unpin(cx);
            }
        }

        if write_failed {
            self.outbound = Outbound::Failed;
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ToBehaviour::Failed));
        }

        let mut received = None;
        let mut inbound_closed = false;
        if let Some(stream) = &mut self.inbound {
            match stream.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(proto))) => match BitswapMessage::from_proto(proto) {
                    Ok(message) => received = Some(message),
                    Err(e) => tracing::debug!(error=%e, "failed to parse inbound bitswap message"),
                },
                Poll::Ready(Some(Err(_))) | Poll::Ready(None) => inbound_closed = true,
                Poll::Pending => {}
            }
        }

        if inbound_closed {
            self.inbound = None;
        }

        if let Some(message) = received {
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                ToBehaviour::Message(message),
            ));
        }

        Poll::Pending
    }
}
