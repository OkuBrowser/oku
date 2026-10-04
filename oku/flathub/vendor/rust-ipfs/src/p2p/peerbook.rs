use core::task::{Context, Poll};
use std::collections::hash_map::Entry;
use std::time::Duration;

use connexa::prelude::{
    Multiaddr, PeerId,
    identify::Info,
    swarm::{
        self, ConnectionClosed, ConnectionDenied, ConnectionId, FromSwarm, NetworkBehaviour,
        THandler, THandlerInEvent, ToSwarm, behaviour::ConnectionEstablished,
    },
    transport::{Endpoint, transport::PortUse},
};

use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;

#[derive(Default, Debug)]
pub struct Behaviour {
    events: VecDeque<ToSwarm<<Self as NetworkBehaviour>::ToSwarm, THandlerInEvent<Self>>>,
    peer_info: HashMap<PeerId, Info>,
    peer_rtt: HashMap<PeerId, [Duration; 3]>,
    peer_connections: HashMap<PeerId, Vec<(ConnectionId, Multiaddr)>>,
}

impl Behaviour {
    pub fn inject_peer_info(&mut self, info: Info) {
        let peer_id = info.public_key.to_peer_id();
        self.peer_info.insert(peer_id, info);
    }

    pub fn peers(&self) -> impl Iterator<Item = &PeerId> {
        self.peer_connections.keys()
    }

    pub fn connected_peers_addrs(&self) -> impl Iterator<Item = (PeerId, Vec<Multiaddr>)> + '_ {
        self.peer_connections.iter().map(|(peer_id, list)| {
            let list = list
                .iter()
                .map(|(_, addr)| addr)
                .cloned()
                .collect::<Vec<_>>();
            (*peer_id, list)
        })
    }

    pub fn set_peer_rtt(&mut self, peer_id: PeerId, rtt: Duration) {
        self.peer_rtt
            .entry(peer_id)
            .and_modify(|r| {
                r.rotate_left(1);
                r[2] = rtt;
            })
            .or_insert([Duration::from_millis(0), Duration::from_millis(0), rtt]);
    }

    pub fn get_peer_rtt(&self, peer_id: PeerId) -> Option<[Duration; 3]> {
        self.peer_rtt.get(&peer_id).copied()
    }

    pub fn get_peer_latest_rtt(&self, peer_id: PeerId) -> Option<Duration> {
        self.get_peer_rtt(peer_id).map(|rtt| rtt[2])
    }

    pub fn get_peer_info(&self, peer_id: PeerId) -> Option<&Info> {
        self.peer_info.get(&peer_id)
    }

    pub fn remove_peer_info(&mut self, peer_id: PeerId) {
        self.peer_info.remove(&peer_id);
    }

    pub fn peer_connections(&self, peer_id: PeerId) -> Option<Vec<Multiaddr>> {
        self.peer_connections
            .get(&peer_id)
            .map(|list| list.iter().map(|(_, addr)| addr).cloned().collect())
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = connexa::dummy::DummyHandler;
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
        Ok(connexa::dummy::DummyHandler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _: ConnectionId,
        _: PeerId,
        _: &Multiaddr,
        _: Endpoint,
        _: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        Ok(connexa::dummy::DummyHandler)
    }

    fn on_connection_handler_event(
        &mut self,
        _: PeerId,
        _: ConnectionId,
        _: swarm::THandlerOutEvent<Self>,
    ) {
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        match event {
            FromSwarm::ConnectionEstablished(ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                ..
            }) => {
                let multiaddr = endpoint.get_remote_address().clone();
                self.peer_connections
                    .entry(peer_id)
                    .or_default()
                    .push((connection_id, multiaddr));
            }
            FromSwarm::ConnectionClosed(ConnectionClosed {
                peer_id,
                connection_id,
                remaining_established,
                ..
            }) => {
                if let Entry::Occupied(mut entry) = self.peer_connections.entry(peer_id) {
                    let list = entry.get_mut();

                    list.retain(|(id, _)| *id != connection_id);

                    if list.is_empty() {
                        entry.remove();
                    }
                }

                if remaining_established == 0 {
                    self.peer_rtt.remove(&(peer_id));
                    self.peer_info.remove(&peer_id);
                }
            }

            _ => {}
        }
    }

    fn poll(&mut self, _: &mut Context) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
