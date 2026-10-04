use futures::{FutureExt, StreamExt};
mod codec;

use crate::behaviour::request_response::codec::Codec;
use crate::error::{ConnexaResult, Error};
use bytes::Bytes;
use futures::channel::mpsc::Sender as MpscSender;
use futures::channel::oneshot::Sender as OneshotSender;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures_timer::Delay;
use libp2p::core::Endpoint;
use libp2p::core::transport::PortUse;
use libp2p::request_response::{
    InboundFailure, InboundRequestId, OutboundFailure, OutboundRequestId, ProtocolSupport,
    ResponseChannel,
};
use libp2p::swarm::{
    ConnectionDenied, ConnectionId, FromSwarm, NetworkBehaviour, THandler, THandlerInEvent,
    THandlerOutEvent, ToSwarm,
};
use libp2p::{Multiaddr, PeerId, StreamProtocol, request_response};
use pollable_map::futures::FutureMap;
use pollable_map::optional::Optional;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::task::{Context, Poll};
use std::time::Duration;

const CHANNEL_LIMIT: usize = 256;
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct RequestResponseConfig {
    pub protocol: String,
    pub timeout: Option<Duration>,
    pub max_request_size: usize,
    pub max_response_size: usize,
    pub concurrent_streams: Option<usize>,
    pub channel_buffer: usize,
    pub protocol_direction: RequestResponseDirection,
}

#[derive(Debug, Clone, Default)]
pub enum RequestResponseDirection {
    /// Support inbound requests
    In,
    /// Support outbound requests
    Out,
    /// Support both inbound and outbound requests
    #[default]
    Both,
}

impl From<RequestResponseDirection> for ProtocolSupport {
    fn from(direction: RequestResponseDirection) -> Self {
        match direction {
            RequestResponseDirection::In => ProtocolSupport::Inbound,
            RequestResponseDirection::Out => ProtocolSupport::Outbound,
            RequestResponseDirection::Both => ProtocolSupport::Full,
        }
    }
}

impl Default for RequestResponseConfig {
    fn default() -> Self {
        Self {
            protocol: "/libp2p/request-response".into(),
            timeout: None,
            max_request_size: 512 * 1024,
            max_response_size: 2 * 1024 * 1024,
            concurrent_streams: None,
            channel_buffer: 128,
            protocol_direction: RequestResponseDirection::default(),
        }
    }
}

impl RequestResponseConfig {
    pub fn new(protocol: impl Into<String>) -> Self {
        let protocol = protocol.into();
        RequestResponseConfig {
            protocol,
            ..Default::default()
        }
    }

    /// Set a duration which would indicate on how long it should take for a response before the request times out.
    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum size of a request.
    pub fn set_max_request_size(mut self, size: usize) -> Self {
        self.max_request_size = size;
        self
    }

    /// Set the maximum size of a response.
    pub fn set_max_response_size(mut self, size: usize) -> Self {
        self.max_response_size = size;
        self
    }

    /// Set the upper bound of inbound and outbound streams.
    pub fn set_concurrent_streams(mut self, size: usize) -> Self {
        self.concurrent_streams = Some(size);
        self
    }

    /// Set the max slot available to send requests over channels.
    pub fn set_channel_buffer(mut self, size: usize) -> Self {
        self.channel_buffer = size;
        self
    }

    pub fn set_protocol_direction(mut self, direction: RequestResponseDirection) -> Self {
        self.protocol_direction = direction;
        self
    }
}

pub struct Behaviour {
    pending_request: HashMap<PeerId, HashMap<InboundRequestId, ResponseChannel<Bytes>>>,

    pending_response:
        HashMap<PeerId, HashMap<OutboundRequestId, OneshotSender<ConnexaResult<Bytes>>>>,
    broadcast_request: Vec<MpscSender<(PeerId, InboundRequestId, Bytes)>>,
    rr_behaviour: request_response::Behaviour<Codec>,

    gc_timer: Optional<Delay>,

    channel_buffer: usize,
}

impl Behaviour {
    pub fn new(config: RequestResponseConfig) -> Self {
        let mut cfg = request_response::Config::default()
            .with_request_timeout(config.timeout.unwrap_or(Duration::from_secs(120)));
        if let Some(size) = config.concurrent_streams {
            cfg = cfg.with_max_concurrent_streams(size);
        }

        let protocol = config.protocol;

        let st_protocol = StreamProtocol::try_from_owned(protocol).expect("valid protocol");

        let protocol = vec![(st_protocol, config.protocol_direction.into())];

        let codec = Codec::new(config.max_request_size, config.max_response_size);

        let rr_behaviour = request_response::Behaviour::with_codec(codec, protocol, cfg);

        Self {
            pending_response: HashMap::new(),
            pending_request: HashMap::new(),
            broadcast_request: Vec::new(),
            rr_behaviour,
            gc_timer: Optional::default(),
            channel_buffer: config.channel_buffer,
        }
    }

    pub fn subscribe(
        &mut self,
    ) -> futures::channel::mpsc::Receiver<(PeerId, InboundRequestId, Bytes)> {
        let (tx, rx) = futures::channel::mpsc::channel(self.channel_buffer);
        self.broadcast_request.push(tx);
        rx
    }

    pub fn send_request(
        &mut self,
        peer_id: PeerId,
        request: impl Into<Bytes>,
    ) -> BoxFuture<'static, ConnexaResult<Bytes>> {
        // Since we are only requesting from a single peer, we will only accept one response, if any, from the stream
        let mut st = self.send_requests([peer_id], request);
        Box::pin(async move {
            match st.next().await {
                // Since we are accepting from a single peer, thus would be tracking the peer,
                // we can exclude the peer id from the result.
                Some((_, result)) => result,
                None => Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe).into()),
            }
        })
    }

    pub fn send_response(
        &mut self,
        peer_id: PeerId,
        id: InboundRequestId,
        response: impl Into<Bytes>,
    ) -> ConnexaResult<()> {
        let pending_list = self
            .pending_request
            .get_mut(&peer_id)
            .ok_or_else(|| Error::NotFound("pending request".into()))?;

        let ch = pending_list
            .remove(&id)
            .ok_or_else(|| Error::NotFound("pending request".into()))?;

        let response = response.into();

        if self.rr_behaviour.send_response(ch, response).is_err() {
            return Err(IoError::new(
                IoErrorKind::BrokenPipe,
                "unable to send response. request either timed out, connection dropped, or unexpected behaviour occurred",
            )
            .into());
        }

        Ok(())
    }

    pub fn send_requests(
        &mut self,
        peers: impl IntoIterator<Item = PeerId>,
        request: impl Into<Bytes>,
    ) -> BoxStream<'static, (PeerId, ConnexaResult<Bytes>)> {
        let request = request.into();
        let mut oneshots = FutureMap::new();
        for peer_id in peers {
            let id = self.rr_behaviour.send_request(&peer_id, request.clone());
            let (tx, rx) = futures::channel::oneshot::channel();
            self.pending_response
                .entry(peer_id)
                .or_default()
                .insert(id, tx);
            oneshots.insert(
                id,
                rx.map(move |r| {
                    let result = match r {
                        Ok(Ok(bytes)) => Ok(bytes),
                        Ok(Err(e)) => Err(e),
                        Err(e) => {
                            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, e).into())
                        }
                    };
                    (peer_id, result)
                }),
            );
        }
        oneshots.map(|(_id, item)| item).boxed()
    }

    fn process_request(
        &mut self,
        id: InboundRequestId,
        peer_id: PeerId,
        request: Bytes,
        response_channel: ResponseChannel<Bytes>,
    ) {
        if self.broadcast_request.is_empty() {
            // If the node is not listening to the requests, then we should drop the response so it would time out
            tracing::warn!(%peer_id, request_id=%id, "no subscribers listening to request. dropping request");
            return;
        }

        //
        match self.broadcast_request.len() > CHANNEL_LIMIT {
            true => {
                // Because the channels exceed a specific threshold, we will begin do cleanup with a timer instead
                if self.gc_timer.is_none() {
                    self.gc_timer.replace(Delay::new(TIMEOUT));
                }
            }
            false => {
                self.gc_timer.take();
                self.broadcast_request.retain(|ch| !ch.is_closed());
            }
        }

        self.pending_request
            .entry(peer_id)
            .or_default()
            .insert(id, response_channel);

        for tx in self.broadcast_request.iter_mut() {
            if let Err(e) = tx.try_send((peer_id, id, request.clone())) {
                tracing::warn!(%peer_id, request_id=%id, error=%e, "unable to send request to subscriber");
                continue;
            }
        }
    }

    fn process_response(&mut self, id: OutboundRequestId, peer_id: PeerId, response: Bytes) {
        let Some(list) = self.pending_response.get_mut(&peer_id) else {
            return;
        };

        let Some(ch) = list.remove(&id) else {
            tracing::warn!(%peer_id, request_id=%id, "no pending request available that is awaiting for a response. dropping response");
            return;
        };

        if list.is_empty() {
            self.pending_response.remove(&peer_id);
        }

        if ch.send(Ok(response)).is_err() {
            tracing::warn!(%peer_id, request_id=%id, "unable to send response");
        }
    }

    fn process_outbound_failure(
        &mut self,
        id: OutboundRequestId,
        peer_id: PeerId,
        error: OutboundFailure,
    ) {
        tracing::error!(%peer_id, request_id=%id, %error, "outbound failure");

        if let Entry::Occupied(mut entry) = self.pending_response.entry(peer_id) {
            let list = entry.get_mut();

            if let Some(tx) = list.remove(&id) {
                _ = tx.send(Err(Error::RequestResponse(error)));
            }

            if list.is_empty() {
                entry.remove();
            }
        }
    }

    fn process_inbound_failure(
        &mut self,
        id: InboundRequestId,
        peer_id: PeerId,
        inbound_failure: InboundFailure,
    ) {
        tracing::error!(%peer_id, request_id=%id, error=%inbound_failure, "inbound failure");
        if let Entry::Occupied(mut entry) = self.pending_request.entry(peer_id) {
            let list = entry.get_mut();
            list.remove(&id);
            if list.is_empty() {
                entry.remove();
            }
        }
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler =
        <request_response::Behaviour<Codec> as NetworkBehaviour>::ConnectionHandler;
    type ToSwarm = ();

    fn handle_pending_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        self.rr_behaviour
            .handle_pending_inbound_connection(connection_id, local_addr, remote_addr)
    }

    fn handle_pending_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        maybe_peer: Option<PeerId>,
        addresses: &[Multiaddr],
        effective_role: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        self.rr_behaviour.handle_pending_outbound_connection(
            connection_id,
            maybe_peer,
            addresses,
            effective_role,
        )
    }

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.rr_behaviour.handle_established_inbound_connection(
            connection_id,
            peer,
            local_addr,
            remote_addr,
        )
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: Endpoint,
        reuse: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.rr_behaviour.handle_established_outbound_connection(
            connection_id,
            peer,
            addr,
            role_override,
            reuse,
        )
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        self.rr_behaviour
            .on_connection_handler_event(peer_id, connection_id, event)
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        self.rr_behaviour.on_swarm_event(event);
    }

    fn poll(&mut self, cx: &mut Context) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if self.gc_timer.poll_unpin(cx).is_ready() {
            self.broadcast_request.retain(|ch| !ch.is_closed());
            if self.broadcast_request.len() > CHANNEL_LIMIT {
                self.gc_timer.replace(Delay::new(TIMEOUT));
            }
        }

        while let Poll::Ready(event) = self.rr_behaviour.poll(cx) {
            match event {
                ToSwarm::GenerateEvent(request_response::Event::Message {
                    connection_id: _,
                    peer: peer_id,
                    message,
                }) => match message {
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => self.process_response(request_id, peer_id, response),
                    request_response::Message::Request {
                        request_id,
                        request,
                        channel,
                    } => {
                        self.process_request(request_id, peer_id, request, channel);
                    }
                },
                ToSwarm::GenerateEvent(request_response::Event::ResponseSent {
                    connection_id: _,
                    peer: peer_id,
                    request_id,
                }) => {
                    tracing::trace!(%peer_id, %request_id, "response sent");
                }
                ToSwarm::GenerateEvent(request_response::Event::OutboundFailure {
                    connection_id: _,
                    peer,
                    request_id,
                    error,
                }) => {
                    tracing::error!(peer_id = %peer, %request_id, ?error, direction="outbound");
                    self.process_outbound_failure(request_id, peer, error);
                }
                ToSwarm::GenerateEvent(request_response::Event::InboundFailure {
                    connection_id: _,
                    peer,
                    request_id,
                    error,
                }) => {
                    tracing::error!(peer_id = %peer, %request_id, ?error, direction="inbound");
                    self.process_inbound_failure(request_id, peer, error);
                }
                other => {
                    let new_to_swarm =
                        other.map_out(|_| unreachable!("we manually map `GenerateEvent` variants"));
                    return Poll::Ready(new_to_swarm);
                }
            };
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::core::muxing::StreamMuxerBox;
    use libp2p::core::transport::MemoryTransport;
    use libp2p::core::upgrade::Version;
    use libp2p::multiaddr::Protocol;
    use libp2p::swarm::SwarmEvent;
    use libp2p::{Swarm, SwarmBuilder};
    use libp2p::{Transport, noise, yamux};

    #[tokio::test]
    async fn send_single_request() {
        let mut node_a = swarm_node().await;
        let mut node_b = swarm_node().await;

        let peer_id_a = *node_a.local_peer_id();

        let peer_id_b = *node_b.local_peer_id();
        let addr_b = node_b.listeners().collect::<Vec<_>>()[0];

        node_a.add_peer_address(peer_id_b, addr_b.clone());

        let mut response_fut = node_a.behaviour_mut().send_request(peer_id_b, "ping");
        let mut node_b_request_listener = node_b.behaviour_mut().subscribe();

        let mut received_request = false;
        let mut received_response = false;

        loop {
            tokio::select! {
                _event = node_a.select_next_some() => {},
                _event = node_b.select_next_some() => {},
                Some((sender_peer_id, id, request)) = node_b_request_listener.next() => {
                    assert_eq!(sender_peer_id, peer_id_a);
                    assert_eq!(request, "ping");
                    received_request = true;
                    node_b.behaviour_mut().send_response(peer_id_a, id, "pong").expect("channel still active");
                }
                response = &mut response_fut => {
                    let response = response.expect("valid response");
                    assert_eq!(response, "pong");
                    received_response = true;
                }
            }

            if received_request && received_response {
                break;
            }
        }
    }

    #[tokio::test]
    async fn send_single_request_to_multiple_peers() {
        let mut node_a = swarm_node().await;
        let mut node_b = swarm_node().await;
        let mut node_c = swarm_node().await;

        let peer_id_a = *node_a.local_peer_id();

        let peer_id_b = *node_b.local_peer_id();
        let addr_b = node_b.listeners().collect::<Vec<_>>()[0];

        let peer_id_c = *node_c.local_peer_id();
        let addr_c = node_c.listeners().collect::<Vec<_>>()[0];

        node_a.add_peer_address(peer_id_b, addr_b.clone());
        node_a.add_peer_address(peer_id_c, addr_c.clone());

        let mut response_st = node_a
            .behaviour_mut()
            .send_requests([peer_id_b, peer_id_c], "ping");
        let mut node_b_request_listener = node_b.behaviour_mut().subscribe();
        let mut node_c_request_listener = node_c.behaviour_mut().subscribe();

        let mut received_request_for_b = false;
        let mut received_request_for_c = false;
        let mut received_response_from_b = false;
        let mut received_response_from_c = false;

        loop {
            tokio::select! {
                _event = node_a.select_next_some() => {},
                _event = node_b.select_next_some() => {},
                _event = node_c.select_next_some() => {},
                Some((sender_peer_id, id, request)) = node_b_request_listener.next() => {
                    assert_eq!(sender_peer_id, peer_id_a);
                    assert_eq!(request, "ping");
                    received_request_for_b = true;
                    node_b.behaviour_mut().send_response(peer_id_a, id, "pong_b").expect("channel still active");
                }
                Some((sender_peer_id, id, request)) = node_c_request_listener.next() => {
                    assert_eq!(sender_peer_id, peer_id_a);
                    assert_eq!(request, "ping");
                    received_request_for_c = true;
                    node_c.behaviour_mut().send_response(peer_id_a, id, "pong_c").expect("channel still active");
                }
                Some((peer_id, response)) = response_st.next() => {
                    match response {
                        Ok(response) if response == "pong_b" => {
                            assert_eq!(peer_id, peer_id_b);
                            received_response_from_b = true;
                        },
                        Ok(response) if response == "pong_c" => {
                            assert_eq!(peer_id, peer_id_c);
                            received_response_from_c = true;
                        }
                        _ => unreachable!(),
                    }
                }
            }

            if received_request_for_b
                && received_request_for_c
                && received_response_from_b
                && received_response_from_c
            {
                break;
            }
        }
    }

    async fn swarm_node() -> Swarm<Behaviour> {
        let mut swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_other_transport(|k| {
                let auth_upgrade = noise::Config::new(k)?;
                let multiplex_upgrade = yamux::Config::default();
                let memory_transport = MemoryTransport::new();
                let transport = memory_transport
                    .upgrade(Version::V1)
                    .authenticate(auth_upgrade)
                    .multiplex(multiplex_upgrade)
                    .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
                    .boxed();
                Ok(transport)
            })
            .unwrap()
            .with_behaviour(|_| Ok(Behaviour::new(Default::default())))
            .unwrap()
            .build();

        let id = swarm
            .listen_on(Multiaddr::empty().with(Protocol::Memory(0)))
            .expect("valid listener");

        if let SwarmEvent::NewListenAddr {
            listener_id,
            address: _,
        } = swarm.next().await.expect("swarm havent dropped")
        {
            assert_eq!(listener_id, id);
        }

        swarm
    }
}
