// TODO: Replace with builtin autorelay behaviour from libp2p. See https://github.com/libp2p/rust-libp2p/pull/6156
use crate::behaviour::autorelay::handler::Out;
use crate::multiaddr_ext::MultiaddrExt;
use crate::prelude::swarm::derive_prelude::{ListenerId, PortUse};
use crate::prelude::swarm::{
    ExternalAddresses, ListenOpts, NewListenAddr,
    derive_prelude::{
        AddressChange, ConnectionClosed, ConnectionDenied, ConnectionEstablished, ConnectionId,
        DialFailure, FromSwarm, ListenerClosed, ListenerError, Multiaddr, NetworkBehaviour,
        THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
    },
    dial_opts::DialOpts,
    dummy,
};
use crate::prelude::transport::Endpoint;
use crate::prelude::{PeerId, Protocol};
use either::Either;
use futures::FutureExt;
use futures_timer::Delay;
use libp2p::core::ConnectedPoint;
use libp2p::swarm::dial_opts::PeerCondition;
use std::collections::BTreeMap;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroU8,
    task::{Context, Poll, Waker},
    time::Duration,
};
use web_time::{Instant, SystemTime};

mod handler;

#[derive(Debug)]
pub struct Behaviour {
    config: Config,
    status: Status,
    auto_status_change: bool,
    external_addresses: ExternalAddresses,
    events: VecDeque<ToSwarm<<Self as NetworkBehaviour>::ToSwarm, THandlerInEvent<Self>>>,

    connections: HashMap<(PeerId, ConnectionId), Connection>,

    reservations: HashMap<ListenerId, Reservation>,

    pending_eviction: Option<ListenerId>,

    external_reservations: HashMap<ListenerId, PeerId>,

    static_relays: HashMap<PeerId, Vec<Multiaddr>>,

    pending_static_dials: HashMap<PeerId, ConnectionId>,

    static_dial_cooldowns: HashMap<PeerId, Instant>,

    failure_counts: HashMap<PeerId, u32>,

    reservation_cooldowns: HashMap<PeerId, Instant>,

    previous_relays: VecDeque<(PeerId, Multiaddr, SystemTime)>,

    relays_available: bool,

    cooldown_wakeup: Option<(Instant, Delay)>,

    waker: Option<Waker>,
}

impl Default for Behaviour {
    fn default() -> Self {
        Self {
            config: Config::default(),
            status: Status::Enable,
            auto_status_change: true,
            external_addresses: ExternalAddresses::default(),
            events: VecDeque::new(),
            connections: HashMap::new(),
            reservations: HashMap::new(),
            pending_eviction: None,
            external_reservations: HashMap::new(),
            static_relays: HashMap::new(),
            pending_static_dials: HashMap::new(),
            static_dial_cooldowns: HashMap::new(),
            failure_counts: HashMap::new(),
            reservation_cooldowns: HashMap::new(),
            previous_relays: VecDeque::new(),
            relays_available: false,
            cooldown_wakeup: None,
            waker: None,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    #[default]
    Enable,
    Disable,
}

#[derive(Debug)]
struct Connection {
    endpoint: ConnectedPoint,
    relay_status: RelayStatus,
}

impl Connection {
    /// Mark relayed connection as not supported
    pub(crate) fn disqualify_connection_if_relayed(&mut self) {
        if self.endpoint.is_relayed() {
            self.relay_status = RelayStatus::NotSupported;
        }
    }

    pub(crate) fn is_relayed(&self) -> bool {
        self.endpoint.is_relayed()
    }

    pub(crate) fn addr(&self) -> &Multiaddr {
        self.endpoint.get_remote_address()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelayStatus {
    Supported,
    NotSupported,
    Pending,
}

#[derive(Debug)]
struct Reservation {
    peer_id: PeerId,
    selected_connection_id: ConnectionId,
    address: Multiaddr,
    status: ReservationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReservationStatus {
    Pending,
    Active,
}

#[derive(Debug)]
pub struct Config {
    max_reservations: NonZeroU8,
    failure_cooldown: Duration,
    failure_cooldown_max: Duration,
    max_previous_relays: usize,
    static_relays: HashMap<PeerId, Vec<Multiaddr>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_reservations: NonZeroU8::new(2).unwrap(),
            failure_cooldown: Duration::from_secs(30),
            failure_cooldown_max: Duration::from_secs(10 * 60),
            max_previous_relays: 16,
            static_relays: HashMap::new(),
        }
    }
}

impl Config {
    pub fn set_max_reservations(mut self, max_reservations: NonZeroU8) -> Self {
        self.max_reservations = max_reservations;
        self
    }

    pub fn set_failure_cooldown(mut self, duration: Duration) -> Self {
        self.failure_cooldown = duration;
        self
    }

    pub fn set_failure_cooldown_max(mut self, duration: Duration) -> Self {
        self.failure_cooldown_max = duration;
        self
    }

    pub fn set_max_previous_relays(mut self, max: usize) -> Self {
        self.max_previous_relays = max;
        self
    }

    pub fn add_static_relay(mut self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        let entry = self.static_relays.entry(peer_id).or_default();
        for addr in addresses {
            if !entry.contains(&addr) {
                entry.push(addr);
            }
        }
        self
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Event {
    /// The status of the local node has changed.
    StatusChanged { status: Status },
    /// No connected peer supports the HOP protocol.
    NoRelaysAvailable,
    /// At least one connected peer supports the HOP protocol.
    RelaysAvailable,
}

impl Behaviour {
    pub fn new_with_config(mut config: Config) -> Self {
        let initial_static_relays = std::mem::take(&mut config.static_relays);
        let mut behaviour = Self {
            config,
            ..Default::default()
        };
        for (peer_id, addresses) in initial_static_relays {
            for address in addresses {
                behaviour.add_static_relay(peer_id, address);
            }
        }
        behaviour
    }

    /// Sets the autorelay status.
    pub fn set_status(&mut self, status: Option<Status>) {
        match status {
            Some(status) => {
                self.auto_status_change = false;
                if self.status != status {
                    self.status = status;
                    self.events
                        .push_back(ToSwarm::GenerateEvent(Event::StatusChanged { status }));
                    if status == Status::Enable {
                        self.meet_reservation_target();
                    }
                }
            }
            None => {
                self.auto_status_change = true;
                self.determine_status_from_external_addresses();
            }
        }

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    /// Register a peer as a static relay.
    ///
    /// This will dial and establish a connection to the peer if it doesn't already have a direct
    /// connection.
    /// Note that peers that are through a relay cannot be used as a static peer
    pub fn add_static_relay(&mut self, peer_id: PeerId, address: Multiaddr) -> bool {
        if address.is_relayed() {
            tracing::warn!(%peer_id, %address, "static relay address is relayed. ignoring.");
            return false;
        }

        let entry = self.static_relays.entry(peer_id).or_default();
        if entry.contains(&address) {
            tracing::warn!(%peer_id, %address, "static relay address already exist");
        } else {
            entry.push(address);
        }
        let combined = entry.clone();

        if !self.queue_static_dial(peer_id, combined) {
            self.meet_reservation_target();
        }

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        true
    }

    /// Remove peer as a static relay.
    /// This will not close any connections or terminate any existing reservation with the relay
    pub fn remove_static_relay(&mut self, peer_id: &PeerId) -> bool {
        self.static_dial_cooldowns.remove(peer_id);
        self.static_relays.remove(peer_id).is_some()
    }

    pub fn static_relays(&self) -> impl Iterator<Item = (&PeerId, &[Multiaddr])> {
        self.static_relays
            .iter()
            .map(|(peer, addrs)| (peer, addrs.as_slice()))
    }

    pub fn previous_relays(&self) -> impl Iterator<Item = (&PeerId, &Multiaddr, &SystemTime)> {
        self.previous_relays
            .iter()
            .map(|(peer, addr, ts)| (peer, addr, ts))
    }

    fn static_dial_in_cooldown(&self, peer_id: &PeerId) -> bool {
        self.static_dial_cooldowns
            .get(peer_id)
            .is_some_and(|deadline| *deadline > Instant::now())
    }

    fn queue_static_dial(&mut self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> bool {
        if !self.static_relays.contains_key(&peer_id)
            || addresses.is_empty()
            || self.has_direct_connection(&peer_id)
            || self.static_dial_in_cooldown(&peer_id)
            || self.pending_static_dials.contains_key(&peer_id)
        {
            return false;
        }
        let opts = DialOpts::peer_id(peer_id)
            .condition(PeerCondition::NotDialing)
            .addresses(addresses)
            .build();

        self.pending_static_dials
            .insert(peer_id, opts.connection_id());
        self.events.push_back(ToSwarm::Dial { opts });
        true
    }

    fn record_previous_relay(&mut self, peer_id: PeerId, address: Multiaddr) {
        let max = self.config.max_previous_relays;
        if max == 0 {
            return;
        }
        self.previous_relays.retain(|(p, _, _)| *p != peer_id);
        if self.previous_relays.len() >= max {
            self.previous_relays.pop_front();
        }
        self.previous_relays
            .push_back((peer_id, address, SystemTime::now()));
    }

    fn forget_previous_relay(&mut self, peer_id: &PeerId) {
        self.previous_relays.retain(|(p, _, _)| p != peer_id);
    }

    fn record_failure(&mut self, peer_id: PeerId) -> Duration {
        let attempts = self.failure_counts.entry(peer_id).or_insert(0);
        *attempts = attempts.saturating_add(1);
        let exponent = attempts.saturating_sub(1).min(20);
        let scale = 1u32 << exponent;
        self.config
            .failure_cooldown
            .saturating_mul(scale)
            .min(self.config.failure_cooldown_max)
    }

    fn clear_failure(&mut self, peer_id: &PeerId) {
        self.failure_counts.remove(peer_id);
        self.reservation_cooldowns.remove(peer_id);
    }

    fn reservation_in_cooldown(&self, peer_id: &PeerId) -> bool {
        self.reservation_cooldowns
            .get(peer_id)
            .is_some_and(|deadline| *deadline > Instant::now())
    }

    fn poll_cooldowns(&mut self, cx: &mut Context<'_>) -> bool {
        let now = Instant::now();

        let reservations_expired = self
            .reservation_cooldowns
            .values()
            .any(|deadline| *deadline <= now);

        let due_dials: Vec<_> = self
            .static_dial_cooldowns
            .iter()
            .filter_map(|(peer, deadline)| (*deadline <= now).then_some(*peer))
            .collect();

        if reservations_expired || !due_dials.is_empty() {
            self.reservation_cooldowns
                .retain(|_, deadline| *deadline > now);
            self.failure_counts.retain(|peer_id, _| {
                self.reservation_cooldowns.contains_key(peer_id)
                    || self.connections.keys().any(|(pid, _)| pid == peer_id)
            });
            self.cooldown_wakeup = None;

            for peer_id in due_dials {
                self.static_dial_cooldowns.remove(&peer_id);

                if let Some(addresses) = self.static_relays.get(&peer_id).cloned() {
                    self.queue_static_dial(peer_id, addresses);
                }
            }

            if reservations_expired {
                self.meet_reservation_target();
            }

            return true;
        }

        let deadline = self
            .reservation_cooldowns
            .values()
            .chain(self.static_dial_cooldowns.values())
            .copied()
            .min();

        let Some(deadline) = deadline else {
            self.cooldown_wakeup = None;
            return false;
        };

        if self.cooldown_wakeup.as_ref().map(|(at, _)| *at) != Some(deadline) {
            self.cooldown_wakeup = Some((
                deadline,
                Delay::new(deadline.saturating_duration_since(now)),
            ));
        }

        self.cooldown_wakeup
            .as_mut()
            .is_some_and(|(_, timer)| timer.poll_unpin(cx).is_ready())
    }

    fn determine_status_from_external_addresses(&mut self) {
        let has_public_addr = self
            .external_addresses
            .iter()
            .any(|addr| !addr.is_relayed());

        let new_status = match has_public_addr {
            true => Status::Disable,
            false => Status::Enable,
        };
        if new_status != self.status {
            self.status = new_status;
            self.events
                .push_back(ToSwarm::GenerateEvent(Event::StatusChanged {
                    status: new_status,
                }));
            match new_status {
                Status::Enable => self.meet_reservation_target(),
                Status::Disable => self.remove_all_reservations(),
            }
        }
    }

    fn has_direct_connection(&self, peer_id: &PeerId) -> bool {
        self.connections
            .iter()
            .any(|((pid, _), info)| pid == peer_id && !info.is_relayed())
    }

    fn make_room_for_static_relays(&mut self, needed: usize) {
        if needed == 0 || self.pending_eviction.is_some() {
            return;
        }

        let listener_id = self.reservations.iter().find_map(|(id, reservation)| {
            let peer = &reservation.peer_id;
            let externally_reserved = self
                .external_reservations
                .values()
                .any(|other| other == peer);

            (!self.static_relays.contains_key(peer) && !externally_reserved).then_some(*id)
        });

        if let Some(id) = listener_id {
            self.pending_eviction = Some(id);
            self.events.push_back(ToSwarm::RemoveListener { id });
        }
    }

    fn remove_reservation(&mut self, id: ListenerId) -> Option<Reservation> {
        if self.pending_eviction == Some(id) {
            self.pending_eviction = None;
        }

        self.reservations.remove(&id)
    }

    fn select_connection_for_reservation(&mut self, peer_id: PeerId, connection_id: ConnectionId) {
        let info = self
            .connections
            .get_mut(&(peer_id, connection_id))
            .expect("connection is present");

        if info.relay_status != RelayStatus::Supported {
            return;
        }

        let address = info.addr().clone();
        let addr_with_peer_id = match address.clone().with_p2p(peer_id) {
            Ok(addr) => addr,
            Err(addr) => {
                tracing::warn!(%addr, "address unexpectedly contains a different peer id than the connection; marking relay connection ineligible");
                info.relay_status = RelayStatus::NotSupported;
                return;
            }
        };

        let opts = ListenOpts::new(addr_with_peer_id.with(Protocol::P2pCircuit));
        let id = opts.listener_id();

        self.reservations.insert(
            id,
            Reservation {
                peer_id,
                selected_connection_id: connection_id,
                address,
                status: ReservationStatus::Pending,
            },
        );
        self.events.push_back(ToSwarm::ListenOn { opts });
    }

    /// Removes all existing reservations.
    pub fn remove_all_reservations(&mut self) {
        for &id in self.reservations.keys() {
            self.events.push_back(ToSwarm::RemoveListener { id });
        }
    }

    fn disable_reservation(&mut self, id: ListenerId, failed: bool) {
        if self.external_reservations.remove(&id).is_some() {
            self.meet_reservation_target();
            return;
        }

        let Some(reservation) = self.remove_reservation(id) else {
            return;
        };

        tracing::trace!(
            listener_id = %id,
            peer_id = %reservation.peer_id,
            selected_connection_id = %reservation.selected_connection_id,
            failed,
            "reservation disabled"
        );

        if failed {
            let duration = self.record_failure(reservation.peer_id);
            self.reservation_cooldowns
                .insert(reservation.peer_id, Instant::now() + duration);
        }

        self.record_previous_relay(reservation.peer_id, reservation.address);
        self.meet_reservation_target();
    }

    fn covered_peers(&self) -> HashSet<PeerId> {
        self.reservations
            .values()
            .map(|reservation| reservation.peer_id)
            .chain(self.external_reservations.values().copied())
            .collect()
    }

    /// Meet the reservation target by selecting connections to establish a reservation.
    fn meet_reservation_target(&mut self) {
        if self.status == Status::Disable {
            return;
        }

        let max = self.config.max_reservations.get() as usize;
        let covered = self.covered_peers();
        let budget = max.saturating_sub(covered.len());

        let mut static_candidates = BTreeMap::new();
        let mut candidates: BTreeMap<_, ConnectionId> = BTreeMap::new();
        for ((peer_id, connection_id), info) in self.connections.iter() {
            if covered.contains(peer_id) {
                continue;
            }
            if self.reservation_in_cooldown(peer_id) {
                continue;
            }
            if info.relay_status != RelayStatus::Supported {
                continue;
            }
            let bucket = if self.static_relays.contains_key(peer_id) {
                &mut static_candidates
            } else {
                &mut candidates
            };
            bucket
                .entry(*peer_id)
                .and_modify(|existing| *existing = (*existing).min(*connection_id))
                .or_insert(*connection_id);
        }

        self.make_room_for_static_relays(static_candidates.len().saturating_sub(budget));

        if budget == 0 {
            return;
        }

        let selected_candidates: Vec<(PeerId, ConnectionId)> = static_candidates
            .into_iter()
            .chain(candidates)
            .take(budget)
            .collect();

        for (peer_id, connection_id) in selected_candidates {
            self.select_connection_for_reservation(peer_id, connection_id);
        }

        debug_assert!(self.covered_peers().len() <= max);
    }

    fn update_relay_availability(&mut self) {
        let has_hop_peer = self
            .connections
            .values()
            .any(|info| info.relay_status == RelayStatus::Supported);

        match (has_hop_peer, self.relays_available) {
            (true, false) => {
                self.relays_available = true;
                self.events
                    .push_back(ToSwarm::GenerateEvent(Event::RelaysAvailable));
            }
            (false, true) => {
                self.relays_available = false;
                self.events
                    .push_back(ToSwarm::GenerateEvent(Event::NoRelaysAvailable));
            }
            _ => {}
        }
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = Either<handler::Handler, dummy::ConnectionHandler>;
    type ToSwarm = Event;

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        if local_addr.is_relayed() {
            Ok(Either::Right(dummy::ConnectionHandler))
        } else {
            Ok(Either::Left(handler::Handler::default()))
        }
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        addr: &Multiaddr,
        _role_override: Endpoint,
        _port_use: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        if addr.is_relayed() {
            Ok(Either::Right(dummy::ConnectionHandler))
        } else {
            Ok(Either::Left(handler::Handler::default()))
        }
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        let change = self.external_addresses.on_swarm_event(&event);

        if self.auto_status_change && change {
            self.determine_status_from_external_addresses();
        }

        match event {
            FromSwarm::ConnectionEstablished(ConnectionEstablished {
                peer_id,
                endpoint,
                connection_id,
                ..
            }) => {
                let mut connection = Connection {
                    endpoint: endpoint.clone(),
                    relay_status: RelayStatus::Pending,
                };

                connection.disqualify_connection_if_relayed();

                self.connections
                    .insert((peer_id, connection_id), connection);

                if self.pending_static_dials.get(&peer_id) == Some(&connection_id) {
                    self.pending_static_dials.remove(&peer_id);
                }

                if self.has_direct_connection(&peer_id) {
                    self.static_dial_cooldowns.remove(&peer_id);
                }
            }
            FromSwarm::ConnectionClosed(ConnectionClosed {
                peer_id,
                connection_id,
                ..
            }) => {
                let Some(connection) = self.connections.remove(&(peer_id, connection_id)) else {
                    return;
                };

                if !self.connections.keys().any(|(pid, _)| *pid == peer_id)
                    && !self.reservation_in_cooldown(&peer_id)
                {
                    self.clear_failure(&peer_id);
                }

                if !connection.is_relayed()
                    && !self.has_direct_connection(&peer_id)
                    && let Some(address) = self
                        .reservations
                        .values()
                        .find(|reservation| reservation.peer_id == peer_id)
                        .map(|reservation| reservation.address.clone())
                {
                    self.record_previous_relay(peer_id, address);
                }

                if let Some(addresses) = self.static_relays.get(&peer_id).cloned() {
                    self.queue_static_dial(peer_id, addresses);
                }

                self.update_relay_availability();
            }
            FromSwarm::AddressChange(AddressChange {
                peer_id,
                connection_id,
                old: _,
                new,
            }) => {
                let Some(connection) = self.connections.get_mut(&(peer_id, connection_id)) else {
                    return;
                };

                connection.endpoint = new.clone();
            }
            FromSwarm::NewListenAddr(NewListenAddr { listener_id, addr }) => {
                if !addr.is_relayed() {
                    return;
                }

                if let Some(reservation) = self.reservations.get_mut(&listener_id) {
                    if reservation.status == ReservationStatus::Pending {
                        reservation.status = ReservationStatus::Active;
                        let peer_id = reservation.peer_id;
                        self.forget_previous_relay(&peer_id);
                        self.clear_failure(&peer_id);
                    }
                    return;
                }

                if let Some(relay_peer_id) = addr.relay_peer_id() {
                    self.external_reservations
                        .insert(listener_id, relay_peer_id);
                }
            }
            FromSwarm::ListenerError(ListenerError { listener_id, .. }) => {
                self.disable_reservation(listener_id, true);
            }
            FromSwarm::ListenerClosed(ListenerClosed {
                listener_id,
                reason,
                ..
            }) => {
                self.disable_reservation(listener_id, reason.is_err());
            }
            FromSwarm::DialFailure(DialFailure {
                peer_id: Some(peer_id),
                connection_id,
                error,
                ..
            }) if self.pending_static_dials.get(&peer_id) == Some(&connection_id) => {
                self.pending_static_dials.remove(&peer_id);

                if self.static_relays.contains_key(&peer_id)
                    && !self.has_direct_connection(&peer_id)
                {
                    tracing::warn!(%peer_id, %error, "dial to static relay failed");
                    self.static_dial_cooldowns
                        .insert(peer_id, Instant::now() + self.config.failure_cooldown);
                }
            }
            _ => {}
        }
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        let Either::Left(event) = event;

        let Some(connection) = self.connections.get_mut(&(peer_id, connection_id)) else {
            return;
        };

        match event {
            Out::Supported => {
                if matches!(
                    connection.relay_status,
                    RelayStatus::Pending | RelayStatus::NotSupported
                ) {
                    connection.relay_status = RelayStatus::Supported;
                    self.meet_reservation_target();
                    self.update_relay_availability();
                }
            }
            Out::Unsupported => {
                connection.relay_status = RelayStatus::NotSupported;

                let has_supported_connection = self.connections.iter().any(|((peer, _), info)| {
                    *peer == peer_id
                        && !info.is_relayed()
                        && info.relay_status == RelayStatus::Supported
                });

                if !has_supported_connection {
                    for (id, reservation) in &self.reservations {
                        if reservation.peer_id == peer_id {
                            self.events.push_back(ToSwarm::RemoveListener { id: *id });
                        }
                    }
                }
                self.update_relay_availability();
            }
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        loop {
            if let Some(event) = self.events.pop_front() {
                return Poll::Ready(event);
            }

            if self.poll_cooldowns(cx) {
                continue;
            }

            self.waker = Some(cx.waker().clone());

            return Poll::Pending;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::behaviour::autorelay;
    use futures::StreamExt;
    use futures::{AsyncRead, AsyncWrite};
    use libp2p::core::muxing::StreamMuxerBox;
    use libp2p::core::transport::{Boxed, MemoryTransport, OrTransport};
    use libp2p::core::upgrade;
    use libp2p::multiaddr::Protocol;
    use libp2p::swarm::{Config, ConnectionId, NetworkBehaviour, SwarmEvent};
    use libp2p::{Multiaddr, PeerId, Swarm, Transport, identify, identity, noise, relay, yamux};
    use std::collections::{HashMap, HashSet};
    use std::num::NonZeroU8;
    use std::time::Duration;

    #[tokio::test]
    async fn autorelay_respects_max_reservations() {
        init_tracing();

        let (relay_a_peer_id, relay_a_addr) = spawn_relay();
        let (relay_b_peer_id, relay_b_addr) = spawn_relay();

        let mut client = build_client(
            autorelay::Config::default().set_max_reservations(NonZeroU8::new(1).unwrap()),
        );
        client.dial(relay_a_addr).unwrap();
        client.dial(relay_b_addr).unwrap();

        let first = wait_until_some(&mut client, Duration::from_secs(20), |event| {
            if let SwarmEvent::Behaviour(ClientEvent::RelayClient(
                relay::client::Event::ReservationReqAccepted {
                    relay_peer_id,
                    renewal: false,
                    ..
                },
            )) = event
            {
                Some(*relay_peer_id)
            } else {
                None
            }
        })
        .await;
        assert!(first == relay_a_peer_id || first == relay_b_peer_id);

        let mut extra = 0usize;
        let mut settle = futures_timer::Delay::new(Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = &mut settle => break,
                ev = client.select_next_some() => {
                    if matches!(
                        ev,
                        SwarmEvent::Behaviour(ClientEvent::RelayClient(
                            relay::client::Event::ReservationReqAccepted { renewal: false, .. }
                        ))
                    ) {
                        extra += 1;
                    }
                }
            }
        }

        assert_eq!(
            extra, 0,
            "autorelay opened {extra} reservation(s) beyond max_reservations=1"
        );
    }

    #[tokio::test]
    async fn autorelay_with_two_reservations_among_five_relays() {
        init_tracing();

        let relay_addrs: Vec<(PeerId, Multiaddr)> = (0..5).map(|_| spawn_relay()).collect();
        let relay_peers: HashSet<PeerId> = relay_addrs.iter().map(|(p, _)| *p).collect();

        let mut client = build_client(
            autorelay::Config::default().set_max_reservations(NonZeroU8::new(2).unwrap()),
        );
        for (_, addr) in &relay_addrs {
            client.dial(addr.clone()).unwrap();
        }

        let mut direct_conns: HashMap<PeerId, ConnectionId> = HashMap::new();
        let mut reservations: HashSet<PeerId> = HashSet::new();

        let mut sleep = futures_timer::Delay::new(Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = &mut sleep => panic!(
                    "timeout: got {} reservations, expected 2",
                    reservations.len()
                ),
                ev = client.select_next_some() => match ev {
                    SwarmEvent::ConnectionEstablished {
                        peer_id, connection_id, endpoint, ..
                    } if !endpoint.is_relayed() && relay_peers.contains(&peer_id) => {
                        direct_conns.insert(peer_id, connection_id);
                    }
                    SwarmEvent::Behaviour(ClientEvent::RelayClient(
                        relay::client::Event::ReservationReqAccepted {
                            relay_peer_id,
                            renewal: false,
                            ..
                        },
                    )) => {
                        reservations.insert(relay_peer_id);
                    }
                    _ => {}
                }
            }
            if reservations.len() == 2 {
                break;
            }
        }

        let drop_peer = *reservations.iter().next().expect("two reservations held");
        let keep_peer = reservations
            .iter()
            .find(|p| **p != drop_peer)
            .copied()
            .expect("two reservations held");
        let drop_conn = *direct_conns
            .get(&drop_peer)
            .expect("direct connection observed");

        assert!(
            client.close_connection(drop_conn),
            "should close the relay connection holding a reservation"
        );

        let mut sleep = futures_timer::Delay::new(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = &mut sleep => panic!("timeout waiting for replacement reservation"),
                ev = client.select_next_some() => {
                    if let SwarmEvent::Behaviour(ClientEvent::RelayClient(
                        relay::client::Event::ReservationReqAccepted {
                            relay_peer_id,
                            renewal: false,
                            ..
                        },
                    )) = ev
                        && relay_peer_id != keep_peer
                        && relay_peer_id != drop_peer
                    {
                        return;
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn autorelay_drops_reservations_when_public_address_appears() {
        init_tracing();

        let (_, relay_a_addr) = spawn_relay();
        let (_, relay_b_addr) = spawn_relay();

        let mut client = build_client(
            autorelay::Config::default().set_max_reservations(NonZeroU8::new(2).unwrap()),
        );
        client.dial(relay_a_addr).unwrap();
        client.dial(relay_b_addr).unwrap();

        let mut confirmed: HashSet<Multiaddr> = HashSet::new();
        let mut sleep = futures_timer::Delay::new(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = &mut sleep => panic!(
                    "timeout: got {} confirmed external addresses, expected 2",
                    confirmed.len()
                ),
                ev = client.select_next_some() => {
                    if let SwarmEvent::ExternalAddrConfirmed { address } = ev
                        && address.iter().any(|p| p == Protocol::P2pCircuit)
                    {
                        confirmed.insert(address);
                    }
                }
            }
            if confirmed.len() == 2 {
                break;
            }
        }

        let public_addr = Multiaddr::empty().with(Protocol::Memory(rand::random::<u64>()));
        client.add_external_address(public_addr);

        let mut expired: HashSet<Multiaddr> = HashSet::new();
        let mut sleep = futures_timer::Delay::new(Duration::from_secs(15));

        loop {
            tokio::select! {
                _ = &mut sleep => panic!(
                    "timeout: only {}/{} relayed addresses expired",
                    expired.len(),
                    confirmed.len()
                ),
                ev = client.select_next_some() => {
                    if let SwarmEvent::ExternalAddrExpired { address } = ev
                        && confirmed.contains(&address)
                    {
                        expired.insert(address);
                    }
                }
            }
            if expired == confirmed {
                break;
            }
        }
    }

    #[tokio::test]
    async fn autorelay_expires_circuit_address_once_on_connection_close() {
        init_tracing();

        let (relay_peer, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client.dial(relay_addr).unwrap();

        let (conn, circuit) = wait_until_some(&mut client, Duration::from_secs(15), {
            let mut direct: Option<ConnectionId> = None;
            move |event| match event {
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    connection_id,
                    endpoint,
                    ..
                } if *peer_id == relay_peer && !endpoint.is_relayed() => {
                    direct = Some(*connection_id);
                    None
                }
                SwarmEvent::ExternalAddrConfirmed { address }
                    if address.iter().any(|p| p == Protocol::P2pCircuit) =>
                {
                    direct.map(|c| (c, address.clone()))
                }
                _ => None,
            }
        })
        .await;

        assert!(client.close_connection(conn));

        let mut expired = 0usize;
        let mut settle = futures_timer::Delay::new(Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = &mut settle => break,
                ev = client.select_next_some() => {
                    if let SwarmEvent::ExternalAddrExpired { address } = &ev
                        && *address == circuit
                    {
                        expired += 1;
                    }
                }
            }
        }

        assert_eq!(
            expired, 1,
            "expected exactly one expiry of the circuit address, got {expired}"
        );
    }

    #[tokio::test]
    async fn autorelay_expires_user_circuit_listen_on_removal() {
        init_tracing();

        let (relay_peer, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client
            .behaviour_mut()
            .autorelay
            .set_status(Some(autorelay::Status::Disable));

        let circuit_listen = relay_addr
            .with(Protocol::P2p(relay_peer))
            .with(Protocol::P2pCircuit);
        let listener_id = client.listen_on(circuit_listen).unwrap();

        let circuit = wait_until_some(&mut client, Duration::from_secs(15), |event| {
            if let SwarmEvent::ExternalAddrConfirmed { address } = event
                && address.iter().any(|p| p == Protocol::P2pCircuit)
            {
                Some(address.clone())
            } else {
                None
            }
        })
        .await;

        wait_until(&mut client, Duration::from_secs(10), |event| {
            matches!(
                event,
                SwarmEvent::NewListenAddr { address, .. } if *address == circuit
            )
        })
        .await;

        assert!(client.remove_listener(listener_id));

        wait_until(&mut client, Duration::from_secs(10), |event| {
            matches!(
                event,
                SwarmEvent::ExternalAddrExpired { address } if *address == circuit
            )
        })
        .await;
    }

    #[tokio::test]
    async fn autorelay_blacklists_failing_relay_and_retries_after_cooldown() {
        init_tracing();

        let (_, relay_addr) = spawn_rejecting_relay();

        let cooldown = Duration::from_secs(1);
        let mut client = build_client(
            autorelay::Config::default()
                .set_max_reservations(NonZeroU8::new(1).unwrap())
                .set_failure_cooldown(cooldown),
        );
        client.dial(relay_addr).unwrap();

        let first_failure_at =
            wait_for_listener_failure(&mut client, Duration::from_secs(10)).await;

        let early_retry = with_timeout(
            wait_for_listener_failure(&mut client, cooldown * 5),
            cooldown / 2,
        )
        .await;
        assert!(
            early_retry.is_none(),
            "autorelay retried during the cooldown window"
        );

        let second_failure_at = wait_for_listener_failure(&mut client, cooldown * 5).await;
        let elapsed = second_failure_at.duration_since(first_failure_at);
        assert!(
            elapsed >= cooldown,
            "retry should respect cooldown (elapsed {elapsed:?}, cooldown {cooldown:?})"
        );
    }

    async fn wait_for_listener_failure(
        client: &mut Swarm<Client>,
        timeout: Duration,
    ) -> std::time::Instant {
        let mut sleep = futures_timer::Delay::new(timeout);

        loop {
            tokio::select! {
                _ = &mut sleep => panic!("timeout waiting for listener failure"),
                ev = client.select_next_some() => {
                    if let SwarmEvent::ListenerClosed { reason: Err(_), .. } = ev {
                        return std::time::Instant::now();
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn autorelay_disabled_does_not_reserve() {
        init_tracing();

        let (_, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client
            .behaviour_mut()
            .autorelay
            .set_status(Some(autorelay::Status::Disable));
        client.dial(relay_addr).unwrap();

        let observed = with_timeout(
            wait_until(&mut client, Duration::from_secs(5), |event| {
                matches!(
                    event,
                    SwarmEvent::Behaviour(ClientEvent::RelayClient(
                        relay::client::Event::ReservationReqAccepted { .. }
                    ))
                )
            }),
            Duration::from_secs(3),
        )
        .await;

        assert!(
            observed.is_none(),
            "autorelay opened a reservation while disabled"
        );
    }

    #[tokio::test]
    async fn autorelay_re_enable_triggers_reservation() {
        init_tracing();

        let (_, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client
            .behaviour_mut()
            .autorelay
            .set_status(Some(autorelay::Status::Disable));
        client.dial(relay_addr).unwrap();

        let mut sleep = futures_timer::Delay::new(Duration::from_secs(3));

        loop {
            tokio::select! {
                _ = &mut sleep => break,
                ev = client.select_next_some() => {
                    if matches!(
                        ev,
                        SwarmEvent::Behaviour(ClientEvent::RelayClient(
                            relay::client::Event::ReservationReqAccepted { .. }
                        ))
                    ) {
                        panic!("autorelay reserved while disabled");
                    }
                }
            }
        }

        client
            .behaviour_mut()
            .autorelay
            .set_status(Some(autorelay::Status::Enable));

        wait_until(&mut client, Duration::from_secs(10), |event| {
            matches!(
                event,
                SwarmEvent::Behaviour(ClientEvent::RelayClient(
                    relay::client::Event::ReservationReqAccepted { .. }
                ))
            )
        })
        .await;
    }

    #[tokio::test]
    async fn autorelay_disable_preserves_active_reservation() {
        init_tracing();

        let (_, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client.dial(relay_addr).unwrap();

        wait_until(&mut client, Duration::from_secs(20), |event| {
            matches!(
                event,
                SwarmEvent::Behaviour(ClientEvent::RelayClient(
                    relay::client::Event::ReservationReqAccepted { .. }
                ))
            )
        })
        .await;

        client
            .behaviour_mut()
            .autorelay
            .set_status(Some(autorelay::Status::Disable));

        let mut sleep = futures_timer::Delay::new(Duration::from_secs(3));

        loop {
            tokio::select! {
                _ = &mut sleep => break,
                ev = client.select_next_some() => {
                    if let SwarmEvent::ListenerClosed { reason: Err(_), .. } = ev {
                        panic!("disabling autorelay dropped an active reservation");
                    }
                    if let SwarmEvent::ExternalAddrExpired { .. } = ev {
                        panic!("disabling autorelay expired an external address");
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn autorelay_prefers_static_relay() {
        init_tracing();

        let (opportunistic_peer, opportunistic_addr) = spawn_relay();
        let (static_peer, static_addr) = spawn_relay();

        let mut client = build_client(
            autorelay::Config::default().set_max_reservations(NonZeroU8::new(1).unwrap()),
        );
        client
            .behaviour_mut()
            .autorelay
            .set_status(Some(autorelay::Status::Disable));

        client.dial(opportunistic_addr).unwrap();
        client
            .behaviour_mut()
            .autorelay
            .add_static_relay(static_peer, static_addr);

        // Let both connections establish and identify exchanges complete.
        let mut warmup = futures_timer::Delay::new(Duration::from_secs(3));
        loop {
            tokio::select! {
                _ = &mut warmup => break,
                _ = client.select_next_some() => {}
            }
        }

        client
            .behaviour_mut()
            .autorelay
            .set_status(Some(autorelay::Status::Enable));

        let accepted_peer = wait_until_some(&mut client, Duration::from_secs(15), |event| {
            if let SwarmEvent::Behaviour(ClientEvent::RelayClient(
                relay::client::Event::ReservationReqAccepted { relay_peer_id, .. },
            )) = event
            {
                Some(*relay_peer_id)
            } else {
                None
            }
        })
        .await;

        assert_eq!(
            accepted_peer, static_peer,
            "autorelay should pick the static relay over the opportunistic one"
        );
        assert_ne!(accepted_peer, opportunistic_peer);
    }

    #[tokio::test]
    async fn remove_static_relay_preserves_active_reservation() {
        init_tracing();

        let (relay_peer, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client
            .behaviour_mut()
            .autorelay
            .add_static_relay(relay_peer, relay_addr);

        wait_until(&mut client, Duration::from_secs(15), |event| {
            matches!(
                event,
                SwarmEvent::Behaviour(ClientEvent::RelayClient(
                    relay::client::Event::ReservationReqAccepted { .. }
                ))
            )
        })
        .await;

        assert!(
            client
                .behaviour_mut()
                .autorelay
                .remove_static_relay(&relay_peer)
        );

        let mut sleep = futures_timer::Delay::new(Duration::from_secs(3));

        loop {
            tokio::select! {
                _ = &mut sleep => break,
                ev = client.select_next_some() => {
                    if let SwarmEvent::ListenerClosed { reason: Err(_), .. } = ev {
                        panic!("removing static relay dropped an active reservation");
                    }
                    if let SwarmEvent::ExternalAddrExpired { .. } = ev {
                        panic!("removing static relay expired an external address");
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn static_relay_redials_after_connection_drop() {
        init_tracing();

        let (relay_peer, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client
            .behaviour_mut()
            .autorelay
            .add_static_relay(relay_peer, relay_addr);

        let conn_id =
            wait_for_reservation_with_conn(&mut client, relay_peer, Duration::from_secs(15)).await;

        assert!(client.close_connection(conn_id));

        wait_until(&mut client, Duration::from_secs(20), {
            let mut redialed = false;
            let mut reserved_again = false;
            move |event| {
                match event {
                    SwarmEvent::ConnectionEstablished {
                        peer_id, endpoint, ..
                    } if *peer_id == relay_peer && !endpoint.is_relayed() => {
                        redialed = true;
                    }
                    SwarmEvent::Behaviour(ClientEvent::RelayClient(
                        relay::client::Event::ReservationReqAccepted { relay_peer_id, .. },
                    )) if *relay_peer_id == relay_peer => {
                        reserved_again = true;
                    }
                    _ => {}
                }
                redialed && reserved_again
            }
        })
        .await;
    }

    async fn wait_until_some<F, T>(
        client: &mut Swarm<Client>,
        timeout: Duration,
        mut extract: F,
    ) -> T
    where
        F: FnMut(&SwarmEvent<ClientEvent>) -> Option<T>,
    {
        let mut sleep = futures_timer::Delay::new(timeout);

        loop {
            tokio::select! {
                _ = &mut sleep => panic!("timeout waiting on predicate"),
                ev = client.select_next_some() => {
                    if let Some(value) = extract(&ev) {
                        return value;
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn autorelay_emits_relay_available_after_recovery() {
        init_tracing();

        let (relay_peer, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client.dial(relay_addr.clone()).unwrap();

        let conn_id =
            wait_for_reservation_with_conn(&mut client, relay_peer, Duration::from_secs(15)).await;

        assert!(client.close_connection(conn_id));

        wait_until(&mut client, Duration::from_secs(10), |event| {
            matches!(
                event,
                SwarmEvent::Behaviour(ClientEvent::Autorelay(autorelay::Event::NoRelaysAvailable))
            )
        })
        .await;

        client.dial(relay_addr).unwrap();

        wait_until(&mut client, Duration::from_secs(15), |event| {
            matches!(
                event,
                SwarmEvent::Behaviour(ClientEvent::Autorelay(autorelay::Event::RelaysAvailable))
            )
        })
        .await;
    }

    #[tokio::test]
    async fn autorelay_no_relays_available_is_edge_triggered() {
        init_tracing();

        let (relay_a_peer, relay_a_addr) = spawn_relay();
        let (relay_b_peer, relay_b_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client.dial(relay_a_addr).unwrap();
        client.dial(relay_b_addr).unwrap();

        let mut conns: HashMap<PeerId, ConnectionId> = HashMap::new();
        let mut reserved: HashSet<PeerId> = HashSet::new();
        let mut sleep = futures_timer::Delay::new(Duration::from_secs(20));

        loop {
            tokio::select! {
                _ = &mut sleep => panic!("did not get both reservations in time"),
                ev = client.select_next_some() => match ev {
                    SwarmEvent::ConnectionEstablished {
                        peer_id, connection_id, endpoint, ..
                    } if !endpoint.is_relayed()
                        && (peer_id == relay_a_peer || peer_id == relay_b_peer) =>
                    {
                        conns.insert(peer_id, connection_id);
                    }
                    SwarmEvent::Behaviour(ClientEvent::RelayClient(
                        relay::client::Event::ReservationReqAccepted { relay_peer_id, .. }
                    )) if relay_peer_id == relay_a_peer || relay_peer_id == relay_b_peer => {
                        reserved.insert(relay_peer_id);
                    }
                    _ => {}
                }
            }
            if reserved.len() == 2 {
                break;
            }
        }

        let conn_a = *conns.get(&relay_a_peer).unwrap();
        let conn_b = *conns.get(&relay_b_peer).unwrap();

        assert!(client.close_connection(conn_a));
        assert!(client.close_connection(conn_b));

        let mut starved_count = 0usize;
        let mut sleep = futures_timer::Delay::new(Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = &mut sleep => break,
                ev = client.select_next_some() => {
                    if matches!(
                        ev,
                        SwarmEvent::Behaviour(ClientEvent::Autorelay(
                            autorelay::Event::NoRelaysAvailable
                        ))
                    ) {
                        starved_count += 1;
                    }
                }
            }
        }

        assert_eq!(
            starved_count, 1,
            "NoRelaysAvailable should fire exactly once across multiple meet_reservation_target invocations"
        );
    }

    #[tokio::test]
    async fn autorelay_resumes_after_public_address_removed() {
        init_tracing();

        let (relay_peer, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client.dial(relay_addr).unwrap();

        wait_for_reservation_from(&mut client, relay_peer, Duration::from_secs(15)).await;

        let public_addr = memory_addr();
        client.add_external_address(public_addr.clone());

        wait_until(&mut client, Duration::from_secs(10), |event| {
            matches!(event, SwarmEvent::ExternalAddrExpired { .. })
        })
        .await;

        client.remove_external_address(&public_addr);

        wait_for_reservation_from(&mut client, relay_peer, Duration::from_secs(15)).await;
    }

    #[tokio::test]
    async fn autorelay_manual_enable_ignores_public_address() {
        init_tracing();

        let (relay_peer, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client
            .behaviour_mut()
            .autorelay
            .set_status(Some(autorelay::Status::Enable));
        client.dial(relay_addr).unwrap();

        wait_for_reservation_from(&mut client, relay_peer, Duration::from_secs(15)).await;

        client.add_external_address(memory_addr());

        let mut sleep = futures_timer::Delay::new(Duration::from_secs(3));

        loop {
            tokio::select! {
                _ = &mut sleep => break,
                ev = client.select_next_some() => {
                    if let SwarmEvent::ListenerClosed { reason: Err(_), .. } = ev {
                        panic!("manual-Enable autorelay dropped reservation after public addr appeared");
                    }
                    if let SwarmEvent::ExternalAddrExpired { address } = &ev
                        && address.iter().any(|p| p == Protocol::P2pCircuit)
                    {
                        panic!("manual-Enable autorelay expired the relayed external address");
                    }
                    if let SwarmEvent::Behaviour(ClientEvent::Autorelay(
                        autorelay::Event::StatusChanged { status: autorelay::Status::Disable },
                    )) = ev
                    {
                        panic!("manual-Enable autorelay flipped to Disable on public addr");
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn autorelay_forgets_previous_relay_on_reacquire() {
        init_tracing();

        let (relay_peer, relay_addr) = spawn_relay();

        let mut client = build_client(autorelay::Config::default());
        client.dial(relay_addr.clone()).unwrap();

        let conn_id =
            wait_for_reservation_with_conn(&mut client, relay_peer, Duration::from_secs(15)).await;

        assert!(client.close_connection(conn_id));

        wait_until(&mut client, Duration::from_secs(10), |event| {
            matches!(
                event,
                SwarmEvent::Behaviour(ClientEvent::Autorelay(autorelay::Event::NoRelaysAvailable))
            )
        })
        .await;

        assert!(
            client
                .behaviour()
                .autorelay
                .previous_relays()
                .any(|(p, _, _)| *p == relay_peer),
            "expected {relay_peer} in previous_relays after loss"
        );

        client.dial(relay_addr).unwrap();

        wait_until(&mut client, Duration::from_secs(15), |event| {
            matches!(
            event,
            SwarmEvent::NewListenAddr { address, .. } if address.iter().any(|p| p == Protocol::P2pCircuit)
        )
        })
            .await;

        let previous: Vec<PeerId> = client
            .behaviour()
            .autorelay
            .previous_relays()
            .map(|(p, _, _)| *p)
            .collect();
        assert!(
            !previous.contains(&relay_peer),
            "expected {relay_peer} to be removed from previous_relays after re-acquire, got {previous:?}"
        );
    }

    #[tokio::test]
    async fn autorelay_previous_relays_is_bounded() {
        init_tracing();

        let peers_and_addrs: Vec<(PeerId, Multiaddr)> = (0..3).map(|_| spawn_relay()).collect();

        let mut client = build_client(
            autorelay::Config::default()
                .set_max_reservations(NonZeroU8::new(1).unwrap())
                .set_max_previous_relays(2),
        );

        for (peer, addr) in &peers_and_addrs {
            client.dial(addr.clone()).unwrap();

            let conn_id =
                wait_for_reservation_with_conn(&mut client, *peer, Duration::from_secs(15)).await;

            assert!(client.close_connection(conn_id));

            wait_until(&mut client, Duration::from_secs(10), |event| {
                matches!(
                    event,
                    SwarmEvent::Behaviour(ClientEvent::Autorelay(
                        autorelay::Event::NoRelaysAvailable
                    ))
                )
            })
            .await;
        }

        let previous: Vec<PeerId> = client
            .behaviour()
            .autorelay
            .previous_relays()
            .map(|(p, _, _)| *p)
            .collect();

        assert_eq!(
            previous.len(),
            2,
            "expected previous_relays to be bounded to 2, got {previous:?}"
        );
        assert!(
            !previous.contains(&peers_and_addrs[0].0),
            "oldest relay should have been evicted: {previous:?}"
        );
        assert!(previous.contains(&peers_and_addrs[1].0));
        assert!(previous.contains(&peers_and_addrs[2].0));
    }

    #[tokio::test]
    async fn autorelay_static_relay_dial_cooldown_after_failure() {
        init_tracing();

        let cooldown = Duration::from_secs(2);
        let mut client = build_client(autorelay::Config::default().set_failure_cooldown(cooldown));

        let unreachable_peer = PeerId::random();
        let unreachable_addr = memory_addr();

        client
            .behaviour_mut()
            .autorelay
            .add_static_relay(unreachable_peer, unreachable_addr.clone());

        wait_until(&mut client, Duration::from_secs(5), |event| {
            matches!(
            event,
            SwarmEvent::OutgoingConnectionError { peer_id: Some(p), .. } if *p == unreachable_peer
        )
        })
            .await;

        let first_failure_at = std::time::Instant::now();

        client
            .behaviour_mut()
            .autorelay
            .add_static_relay(unreachable_peer, unreachable_addr.clone());

        let mut redialed = false;
        let mut watch = futures_timer::Delay::new(cooldown / 2);
        loop {
            tokio::select! {
                _ = &mut watch => break,
                ev = client.select_next_some() => {
                    if matches!(
                        ev,
                        SwarmEvent::OutgoingConnectionError { peer_id: Some(p), .. } if p == unreachable_peer
                    ) {
                        redialed = true;
                        break;
                    }
                }
            }
        }
        assert!(!redialed, "autorelay redialed within cooldown");

        let remaining = cooldown
            .checked_sub(first_failure_at.elapsed())
            .unwrap_or_default();
        if !remaining.is_zero() {
            futures_timer::Delay::new(remaining + Duration::from_millis(200)).await;
        }

        client
            .behaviour_mut()
            .autorelay
            .add_static_relay(unreachable_peer, unreachable_addr);

        wait_until(&mut client, Duration::from_secs(5), |event| {
            matches!(
            event,
            SwarmEvent::OutgoingConnectionError { peer_id: Some(p), .. } if *p == unreachable_peer
        )
        })
            .await;
    }

    #[tokio::test]
    async fn autorelay_evicts_discovered_peers_for_static() {
        init_tracing();

        let (opp_a_peer, opp_a_addr) = spawn_relay();
        let (opp_b_peer, opp_b_addr) = spawn_relay();
        let (static_peer, static_addr) = spawn_relay();

        let mut client = build_client(
            autorelay::Config::default().set_max_reservations(NonZeroU8::new(1).unwrap()),
        );

        client.dial(opp_a_addr).unwrap();
        client.dial(opp_b_addr).unwrap();

        wait_until_some(&mut client, Duration::from_secs(20), |event| {
            if let SwarmEvent::Behaviour(ClientEvent::RelayClient(
                relay::client::Event::ReservationReqAccepted { relay_peer_id, .. },
            )) = event
                && (*relay_peer_id == opp_a_peer || *relay_peer_id == opp_b_peer)
            {
                Some(*relay_peer_id)
            } else {
                None
            }
        })
        .await;

        client
            .behaviour_mut()
            .autorelay
            .add_static_relay(static_peer, static_addr);

        wait_for_reservation_from(&mut client, static_peer, Duration::from_secs(20)).await;
    }

    async fn wait_until<F>(client: &mut Swarm<Client>, timeout: Duration, mut predicate: F)
    where
        F: FnMut(&SwarmEvent<ClientEvent>) -> bool,
    {
        let mut sleep = futures_timer::Delay::new(timeout);
        loop {
            tokio::select! {
                _ = &mut sleep => panic!("timeout waiting on predicate"),
                ev = client.select_next_some() => {
                    if predicate(&ev) {
                        return;
                    }
                }
            }
        }
    }

    async fn with_timeout<F: Future>(future: F, timeout: Duration) -> Option<F::Output> {
        use futures::future::Either;
        let timer = futures_timer::Delay::new(timeout);
        futures::pin_mut!(future);
        match futures::future::select(future, timer).await {
            Either::Left((output, _)) => Some(output),
            Either::Right(_) => None,
        }
    }

    async fn wait_for_reservation_from(
        client: &mut Swarm<Client>,
        peer: PeerId,
        timeout: Duration,
    ) {
        wait_until(client, timeout, |event| {
            matches!(
                event,
                SwarmEvent::Behaviour(ClientEvent::RelayClient(
                    relay::client::Event::ReservationReqAccepted { relay_peer_id, .. }
                )) if *relay_peer_id == peer
            )
        })
        .await;
    }

    async fn wait_for_reservation_with_conn(
        client: &mut Swarm<Client>,
        peer: PeerId,
        timeout: Duration,
    ) -> ConnectionId {
        wait_until_some(client, timeout, {
            let mut established: Option<ConnectionId> = None;
            let mut reserved = false;
            move |event| {
                match event {
                    SwarmEvent::ConnectionEstablished {
                        peer_id,
                        connection_id,
                        endpoint,
                        ..
                    } if *peer_id == peer && !endpoint.is_relayed() => {
                        established = Some(*connection_id);
                    }
                    SwarmEvent::Behaviour(ClientEvent::RelayClient(
                        relay::client::Event::ReservationReqAccepted { relay_peer_id, .. },
                    )) if *relay_peer_id == peer => {
                        reserved = true;
                    }
                    _ => {}
                }
                if reserved { established } else { None }
            }
        })
        .await
    }

    fn init_tracing() {
        // let _ = tracing_subscriber::fmt()
        //     .with_env_filter(EnvFilter::from_default_env())
        //     .try_init();
    }

    fn memory_addr() -> Multiaddr {
        Multiaddr::empty().with(Protocol::Memory(rand::random::<u64>()))
    }

    fn spawn_relay() -> (PeerId, Multiaddr) {
        spawn_relay_swarm(build_relay())
    }

    fn spawn_rejecting_relay() -> (PeerId, Multiaddr) {
        spawn_relay_swarm(build_rejecting_relay())
    }

    fn spawn_relay_swarm(mut relay: Swarm<Relay>) -> (PeerId, Multiaddr) {
        let addr = memory_addr();
        let peer = *relay.local_peer_id();
        relay.listen_on(addr.clone()).unwrap();
        relay.add_external_address(addr.clone());
        tokio::spawn(relay.collect::<Vec<_>>());
        (peer, addr)
    }

    fn build_relay() -> Swarm<Relay> {
        build_relay_with_config(relay::Config {
            reservation_duration: Duration::from_secs(60),
            ..Default::default()
        })
    }

    fn build_rejecting_relay() -> Swarm<Relay> {
        build_relay_with_config(relay::Config {
            max_reservations: 0,
            ..Default::default()
        })
    }

    fn build_relay_with_config(config: relay::Config) -> Swarm<Relay> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = local_key.public().to_peer_id();
        let transport = upgrade_transport(MemoryTransport::default().boxed(), &local_key);

        Swarm::new(
            transport,
            Relay {
                relay: relay::Behaviour::new(local_peer_id, config),
                identify: identify::Behaviour::new(identify::Config::new(
                    "/autorelay-test/1.0.0".to_owned(),
                    local_key.public(),
                )),
            },
            local_peer_id,
            Config::with_tokio_executor(),
        )
    }

    fn build_client(autorelay_config: autorelay::Config) -> Swarm<Client> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = local_key.public().to_peer_id();
        let (relay_transport, relay_client) = relay::client::new(local_peer_id);

        let transport = upgrade_transport(
            OrTransport::new(relay_transport, MemoryTransport::default()).boxed(),
            &local_key,
        );

        Swarm::new(
            transport,
            Client {
                relay_client,
                autorelay: autorelay::Behaviour::new_with_config(autorelay_config),
                identify: identify::Behaviour::new(identify::Config::new(
                    "/autorelay-test/1.0.0".to_owned(),
                    local_key.public(),
                )),
            },
            local_peer_id,
            Config::with_tokio_executor(),
        )
    }

    fn upgrade_transport<StreamSink>(
        transport: Boxed<StreamSink>,
        identity: &identity::Keypair,
    ) -> Boxed<(PeerId, StreamMuxerBox)>
    where
        StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        transport
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(identity).unwrap())
            .multiplex(yamux::Config::default())
            .boxed()
    }

    #[derive(NetworkBehaviour)]
    struct Relay {
        relay: relay::Behaviour,
        identify: identify::Behaviour,
    }

    #[derive(NetworkBehaviour)]
    struct Client {
        relay_client: relay::client::Behaviour,
        autorelay: autorelay::Behaviour,
        identify: identify::Behaviour,
    }
}
