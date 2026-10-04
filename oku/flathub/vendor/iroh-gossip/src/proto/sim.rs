//! Simulation framework for testing the protocol implementation

use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque},
    fmt,
    str::FromStr,
};

use bytes::Bytes;
use n0_future::time::{Duration, Instant};
use rand::{rngs::ChaCha12Rng, seq::IteratorRandom, Rng, RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use tracing::{debug, debug_span, info, info_span, trace, warn};

use super::{Command, Config, Event, InEvent, OutEvent, PeerIdentity, State, TopicId};
use crate::proto::{PeerData, Scope};

const DEFAULT_LATENCY_STATIC: Duration = Duration::from_millis(50);
const DEFAULT_LATENCY_MIN: Duration = Duration::from_millis(10);
const DEFAULT_LATENCY_MAX: Duration = Duration::from_millis(100);

/// Configuration for a [`Network`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Configures the latency between peers.
    #[serde(default)]
    pub latency: LatencyConfig,
    /// Default protocol config for all peers.
    #[serde(default)]
    pub proto: Config,
}

impl From<Config> for NetworkConfig {
    fn from(config: Config) -> Self {
        Self {
            latency: Default::default(),
            proto: config,
        }
    }
}

/// Configures the latency between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatencyConfig {
    /// Use the same latency, always.
    Static(#[serde(with = "humantime_serde")] Duration),
    /// Chose a random latency for each connection within the specified bounds.
    Dynamic {
        /// The lower bound for the latency between two peers.
        #[serde(with = "humantime_serde")]
        min: Duration,
        /// The upper bound for the latency between two peers.
        #[serde(with = "humantime_serde")]
        max: Duration,
    },
}

impl LatencyConfig {
    /// Returns a default latency config with a static latency.
    pub fn default_static() -> Self {
        Self::Static(DEFAULT_LATENCY_STATIC)
    }

    /// Returns a default latency config with a dynamic latency.
    pub fn default_dynamic() -> Self {
        Self::Dynamic {
            min: DEFAULT_LATENCY_MIN,
            max: DEFAULT_LATENCY_MAX,
        }
    }

    /// Creates a new latency config with the provided min and max values in milliseconds.
    pub fn random_ms(min: u64, max: u64) -> Self {
        Self::Dynamic {
            min: Duration::from_millis(min),
            max: Duration::from_millis(max),
        }
    }

    /// Returns the maximum latency possible.
    pub fn max(&self) -> Duration {
        match self {
            Self::Static(dur) => *dur,
            Self::Dynamic { max, .. } => *max,
        }
    }

    /// Returns a new latency value to use for a peer connection.
    pub fn r#gen(&self, mut rng: impl Rng) -> Duration {
        match self {
            Self::Static(d) => *d,
            // TODO(frando): use uniform distribution?
            Self::Dynamic { min, max } => rng.random_range(*min..*max),
        }
    }
}

impl Default for LatencyConfig {
    fn default() -> Self {
        Self::default_dynamic()
    }
}

/// Test network implementation.
///
/// A discrete event simulation of a gossip swarm.
#[derive(Debug)]
pub struct Network<PI, R> {
    start: Instant,
    time: Instant,
    tick: usize,
    peers: BTreeMap<PI, State<PI, R>>,
    conns: BTreeSet<ConnId<PI>>,
    events: VecDeque<(PI, TopicId, Event<PI>)>,
    latencies: BTreeMap<ConnId<PI>, Duration>,
    rng: R,
    config: NetworkConfig,
    queue: TimedEventQueue<PI>,
}

impl<PI, R> Network<PI, R> {
    /// Creates a new network.
    pub fn new(config: NetworkConfig, rng: R) -> Self {
        let time = Instant::now();
        Self {
            tick: 0,
            start: time,
            time,
            config,
            queue: Default::default(),
            peers: Default::default(),
            conns: Default::default(),
            events: Default::default(),
            latencies: BTreeMap::new(),
            rng,
        }
    }
}

impl<PI: PeerIdentity + fmt::Display, R: Rng + SeedableRng> Network<PI, R> {
    /// Inserts a new peer.
    ///
    /// Panics if the peer already exists.
    pub fn insert(&mut self, peer_id: PI) {
        let config = self.config.proto.clone();
        self.insert_with_config(peer_id, config);
    }

    /// Inserts a new peer with the specified protocol config.
    ///
    /// Panics if the peer already exists.
    pub fn insert_with_config(&mut self, peer_id: PI, config: Config) {
        assert!(
            !self.peers.contains_key(&peer_id),
            "duplicate peer: {peer_id:?}"
        );
        let rng = R::from_rng(&mut self.rng);
        let state = State::new(peer_id, PeerData::default(), config, rng);
        self.peers.insert(peer_id, state);
    }

    /// Inserts a new peer and joins a topic with a set of bootstrap nodes.
    ///
    /// Panics if the peer already exists.
    pub fn insert_and_join(&mut self, peer_id: PI, topic: TopicId, bootstrap: Vec<PI>) {
        self.insert(peer_id);
        self.command(peer_id, topic, Command::Join(bootstrap));
    }
}

impl<PI: PeerIdentity + fmt::Display, R: Rng + SeedableRng> Network<PI, R> {
    /// Drains all queued events.
    pub fn events(&mut self) -> impl Iterator<Item = (PI, TopicId, Event<PI>)> + '_ {
        self.events.drain(..)
    }

    /// Drains all queued events and returns them in a sorted vector.
    pub fn events_sorted(&mut self) -> Vec<(PI, TopicId, Event<PI>)> {
        sort(self.events().collect())
    }

    /// Returns all active connections.
    pub fn conns(&self) -> Vec<(PI, PI)> {
        sort(self.conns.iter().cloned().map(Into::into).collect())
    }

    /// Queues and performs a command.
    pub fn command(&mut self, peer: PI, topic: TopicId, command: Command<PI>) {
        debug!(?peer, "~~ COMMAND {command:?}");
        self.queue
            .insert(self.time, peer, InEvent::Command(topic, command));
        self.tick();
    }

    /// Returns an iterator over the [`State`] for each peer.
    pub fn peer_states(&self) -> impl Iterator<Item = &State<PI, R>> {
        self.peers.values()
    }

    /// Returns an iterator over the node ids of all peers.
    pub fn peer_ids(&self) -> impl Iterator<Item = PI> + '_ {
        self.peers.keys().cloned()
    }

    /// Returns the [`State`] for a peer, if it exists.
    pub fn peer(&self, peer: &PI) -> Option<&State<PI, R>> {
        self.peers.get(peer)
    }

    /// Returns the neighbors a peer has on the swarm membership layer.
    pub fn neighbors(&self, peer: &PI, topic: &TopicId) -> Option<Vec<PI>> {
        let peer = self.peer(peer)?;
        let state = peer.state(topic)?;
        Some(state.swarm.active_view.iter().cloned().collect::<Vec<_>>())
    }

    /// Removes a peer, breaking all connections to other peers.
    pub fn remove(&mut self, peer: &PI) {
        let remove_conns: Vec<_> = self
            .conns
            .iter()
            .filter(|&c| c.peers().contains(peer))
            .cloned()
            .collect();
        for conn in remove_conns.into_iter() {
            self.kill_connection(*peer, conn.other(*peer).unwrap());
        }
        self.peers.remove(peer);
    }

    /// Returns the time elapsed since starting the network.
    pub fn elapsed(&self) -> Duration {
        self.time.duration_since(self.start)
    }

    /// Returns the time elapsed since starting the network, formatted as seconds with limited decimals.
    pub fn elapsed_fmt(&self) -> String {
        format!("{:>2.4}s", self.elapsed().as_secs_f32())
    }

    /// Runs the simulation for `n` times the maximum latency between peers.
    pub fn run_trips(&mut self, n: usize) {
        let duration = self.config.latency.max() * n as u32;
        self.run_duration(duration)
    }

    /// Runs the simulation for `timeout`.
    pub fn run_duration(&mut self, timeout: Duration) {
        let end = self.time + timeout;
        while self.queue.next_before(end) {
            self.tick();
        }
        assert!(self.time <= end);
        self.time = end;
    }

    /// Runs the simulation while `f` returns `true`.
    ///
    /// The callback will be called for each emitted event.
    pub fn run_while(&mut self, mut f: impl FnMut(PI, TopicId, Event<PI>) -> bool) {
        loop {
            while let Some((peer, topic, event)) = self.events.pop_front() {
                if !f(peer, topic, event) {
                    return;
                }
            }
            self.tick();
        }
    }

    /// Runs the simulation while `f` returns `true`, aborting after `timeout`.
    ///
    /// The callback will be called for each emitted event.
    pub fn run_while_with_timeout(
        &mut self,
        timeout: Duration,
        mut f: impl FnMut(PI, TopicId, Event<PI>) -> bool,
    ) {
        let end = self.time + timeout;
        loop {
            while let Some((peer, topic, event)) = self.events.pop_front() {
                if !f(peer, topic, event) {
                    return;
                }
            }
            if self.queue.next_before(end) {
                self.tick();
            } else {
                break;
            }
        }
        assert!(self.time <= end);
        self.time = end;
    }

    fn tick(&mut self) {
        self.tick += 1;
        let Some((time, peer, event)) = self.queue.pop() else {
            warn!("tick on empty queue");
            return;
        };
        assert!(time >= self.time);
        self.time = time;
        let span = debug_span!("tick", %peer, tick = %self.tick, t = %self.elapsed_fmt());
        let _guard = span.enter();
        debug!("~~ TICK ");

        let Some(state) = self.peers.get_mut(&peer) else {
            // TODO: queue PeerDisconnected for sender?
            warn!(?time, ?peer, ?event, "event for dead peer");
            return;
        };
        if let InEvent::RecvMessage(from, _message) = &event {
            self.conns.insert((*from, peer).into());
        }
        let out = state.handle(event, self.time, None);
        let mut kill = vec![];
        for event in out {
            match event {
                OutEvent::SendMessage(to, message) => {
                    let latency = latency_between(
                        &self.config.latency,
                        &mut self.latencies,
                        &peer,
                        &to,
                        &mut self.rng,
                    );
                    self.queue
                        .insert(self.time + latency, to, InEvent::RecvMessage(peer, message));
                }
                OutEvent::ScheduleTimer(time, timer) => {
                    self.queue
                        .insert(self.time + time, peer, InEvent::TimerExpired(timer));
                }
                OutEvent::DisconnectPeer(to) => {
                    debug!(peer = ?peer, other = ?to, "disconnect");
                    kill.push((peer, to));
                }
                OutEvent::EmitEvent(topic, event) => {
                    debug!(peer = ?peer, "emit {event:?}");
                    self.events.push_back((peer, topic, event));
                }
                OutEvent::PeerData(_peer, _data) => {}
            }
        }
        for (from, to) in kill {
            self.kill_connection(from, to);
        }
    }

    /// Breaks the connection between two peers.
    ///
    /// The `to` peer will received a [`InEvent::PeerDisconnected`] after a latency interval.
    fn kill_connection(&mut self, from: PI, to: PI) {
        let conn = ConnId::from((from, to));
        if self.conns.remove(&conn) {
            // We add the event a microsecond after the regular latency between the two peers,
            // so that any messages queued from the current time arrive before the disconnected event.
            let latency = latency_between(
                &self.config.latency,
                &mut self.latencies,
                &from,
                &to,
                &mut self.rng,
            ) + Duration::from_micros(1);
            self.queue
                .insert(self.time + latency, to, InEvent::PeerDisconnected(from));
        }
    }

    /// Checks if all neighbor and eager relations are synchronous.
    ///
    /// Iterates over all peers, and checks for each peer X:
    /// - that all active view members (neighbors) have X listed as a neighbor as well
    /// - that all eager peers have X listed as eager as well
    ///
    /// Returns `true` if this is holds, otherwise returns `false`.
    ///
    /// Logs, at debug level, the cases where the above doesn't hold.
    pub fn check_synchronicity(&self) -> bool {
        let mut ok = true;
        for state in self.peers.values() {
            let peer = *state.me();
            for (topic, state) in state.states() {
                for other in state.swarm.active_view.iter() {
                    let other_state = &self
                        .peers
                        .get(other)
                        .unwrap()
                        .state(topic)
                        .unwrap()
                        .swarm
                        .active_view;
                    if !other_state.contains(&peer) {
                        debug!(node = %peer, other = ?other, "missing active_view peer in other");
                        ok = false;
                    }
                }
                for other in state.gossip.eager_push_peers.iter() {
                    let other_state = &self
                        .peers
                        .get(other)
                        .unwrap()
                        .state(topic)
                        .unwrap()
                        .gossip
                        .eager_push_peers;
                    if !other_state.contains(&peer) {
                        debug!(node = %peer, other = ?other, "missing eager_push peer in other");
                        ok = false;
                    }
                }
            }
        }
        ok
    }

    /// Returns a report with histograms on active, passive, eager and lazy counts.
    pub fn report(&self) -> NetworkReport<PI> {
        let mut histograms = NetworkHistograms::default();
        let mut peers_without_neighbors = Vec::new();
        for (id, peer) in self.peers.iter() {
            let state = peer.state(&TOPIC).unwrap();
            add_one(&mut histograms.active, state.swarm.active_view.len());
            add_one(&mut histograms.passive, state.swarm.passive_view.len());
            add_one(&mut histograms.eager, state.gossip.eager_push_peers.len());
            add_one(&mut histograms.lazy, state.gossip.lazy_push_peers.len());
            if state.swarm.active_view.is_empty() {
                peers_without_neighbors.push(*id);
                trace!(node=%id, active = ?state.swarm.active_view.iter().collect::<Vec<_>>(), passive=?state.swarm.passive_view.iter().collect::<Vec<_>>(), "active view empty^");
            }
        }
        NetworkReport {
            histograms,
            peer_count: self.peers.len(),
            peers_without_neighbors,
        }
    }
}

fn latency_between<PI: PeerIdentity + Ord + PartialOrd, R: Rng>(
    latency_config: &LatencyConfig,
    latencies: &mut BTreeMap<ConnId<PI>, Duration>,
    a: &PI,
    b: &PI,
    rng: &mut R,
) -> Duration {
    let id: ConnId<PI> = (*a, *b).into();
    *latencies
        .entry(id)
        .or_insert_with(|| latency_config.r#gen(rng))
}

#[derive(Debug)]
struct TimedEventQueue<PI> {
    seq: i32,
    events: BinaryHeap<(TimedEvent<PeerEvent<PI>>, i32)>,
}

impl<PI> Default for TimedEventQueue<PI> {
    fn default() -> Self {
        Self {
            seq: 0,
            events: Default::default(),
        }
    }
}

impl<PI> TimedEventQueue<PI> {
    fn insert(&mut self, time: Instant, peer: PI, event: InEvent<PI>) {
        let seq = self.seq;
        self.seq += 1;
        self.events.push((
            TimedEvent {
                time,
                event: PeerEvent(peer, event),
            },
            -seq,
        ))
    }

    fn pop(&mut self) -> Option<(Instant, PI, InEvent<PI>)> {
        self.events
            .pop()
            .map(|(e, _)| (e.time, e.event.0, e.event.1))
    }

    fn peek_next(&self) -> Option<Instant> {
        self.events.peek().map(|(e, _)| e.time)
    }

    fn next_before(&self, before: Instant) -> bool {
        match self.peek_next() {
            None => false,
            Some(at) => at <= before,
        }
    }
}

#[derive(Debug)]
struct TimedEvent<E> {
    time: Instant,
    event: E,
}

#[derive(Debug)]
struct PeerEvent<PI>(PI, InEvent<PI>);

impl<E> Eq for TimedEvent<E> {}

impl<E> PartialEq for TimedEvent<E> {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl<E> PartialOrd for TimedEvent<E> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<E> Ord for TimedEvent<E> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time).reverse()
    }
}

/// The peer id type used in the simulator.
type PeerId = u64;

/// Configuration for the [`Simulator`].
#[derive(Debug, Serialize, Deserialize)]
pub struct SimulatorConfig {
    /// Seed for the random number generator used in the nodes
    pub rng_seed: u64,
    /// Number of nodes to create
    pub peers: usize,
    /// Timeout after which a gossip round is aborted
    pub gossip_round_timeout: Duration,
}

/// Variants how to bootstrap the swarm.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum BootstrapMode {
    /// All peers join a single peer.
    #[default]
    Single,
    /// First `count` bootstrap peers are created and join each other,
    /// then the remaining peers join the swarm by joining one of these first `count` peers.
    Set {
        /// Number of bootstrap peers to join first
        count: u64,
    },
}

impl SimulatorConfig {
    /// Creates a [`SimulatorConfig`] by reading from environment variables.
    ///
    /// [`Self::peers`] is read from `PEERS`, defaulting to `100` if unset.
    /// [`Self::rng_seed`] is read from `SEED`, defaulting to `0` if unset.
    /// [`Self::gossip_round_timeout`] is read, as seconds, from `GOSSIP_ROUND_TIMEOUT`, defaulting to `5` if unset.
    pub fn from_env() -> Self {
        let peer = read_var("PEERS", 100);
        Self {
            rng_seed: read_var("SEED", 0),
            peers: peer,
            gossip_round_timeout: Duration::from_secs(read_var("GOSSIP_ROUND_TIMEOUT", 5)),
        }
    }
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            rng_seed: 0,
            peers: 100,
            gossip_round_timeout: Duration::from_secs(5),
        }
    }
}

/// Statistics for a gossip round.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RoundStats {
    /// The (simulated) time this round took in total.
    pub duration: Duration,
    /// The relative message redundancy in this round.
    pub rmr: f32,
    /// The maximum last delivery hop in this round.
    pub ldh: f32,
    /// The number of undelivered messages in this round.
    pub missed: f32,
}

/// Difference (as factors) between two [`RoundStats`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RoundStatsDiff {
    /// The difference in [`RoundStats::duration`], as a factor.
    pub duration: f32,
    /// The difference in [`RoundStats::rmr`], as a factor.
    pub rmr: f32,
    /// The difference in [`RoundStats::ldh`], as a factor.
    pub ldh: f32,
    /// The difference in [`RoundStats::missed`], as a factor.
    pub missed: f32,
}

impl fmt::Display for RoundStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RMR {:>6.2} LDH {:>6.2} duration {:>6.2}ms missed {:>10.2}",
            self.rmr,
            self.ldh,
            self.duration.as_millis(),
            self.missed
        )
    }
}

impl RoundStats {
    fn new_max() -> Self {
        Self {
            duration: Duration::MAX,
            rmr: f32::MAX,
            ldh: f32::MAX,
            missed: f32::MAX,
        }
    }

    /// Calculates the mean for each value in a list of [`RoundStats`].
    pub fn mean<'a>(rounds: impl IntoIterator<Item = &'a RoundStats>) -> RoundStats {
        let (len, mut avg) =
            rounds
                .into_iter()
                .fold((0., RoundStats::default()), |(len, mut agg), round| {
                    agg.rmr += round.rmr;
                    agg.ldh += round.ldh;
                    agg.duration += round.duration;
                    agg.missed += round.missed;
                    (len + 1., agg)
                });
        avg.rmr /= len;
        avg.ldh /= len;
        avg.missed /= len;
        avg.duration /= len as u32;
        avg
    }

    /// Calculates the minimum for each value in a list of [`RoundStats`].
    pub fn min<'a>(rounds: impl IntoIterator<Item = &'a RoundStats>) -> RoundStats {
        rounds
            .into_iter()
            .fold(RoundStats::new_max(), |mut agg, round| {
                agg.rmr = agg.rmr.min(round.rmr);
                agg.ldh = agg.ldh.min(round.ldh);
                agg.duration = agg.duration.min(round.duration);
                agg.missed = agg.missed.min(round.missed);
                agg
            })
    }

    /// Calculates the maximum for each value in a list of [`RoundStats`].
    pub fn max<'a>(rounds: impl IntoIterator<Item = &'a RoundStats>) -> RoundStats {
        rounds
            .into_iter()
            .fold(RoundStats::default(), |mut agg, round| {
                agg.rmr = agg.rmr.max(round.rmr);
                agg.ldh = agg.ldh.max(round.ldh);
                agg.duration = agg.duration.max(round.duration);
                agg.missed = agg.missed.max(round.missed);
                agg
            })
    }

    /// Calculates the minimum, maximum, and mean for each value in a list of [`RoundStats`].
    pub fn avg(rounds: &[RoundStats]) -> RoundStatsAvg {
        let len = rounds.len();
        let min = Self::min(rounds);
        let max = Self::max(rounds);
        let mean = Self::mean(rounds);
        RoundStatsAvg {
            len,
            min,
            max,
            mean,
        }
    }

    /// Calculates the difference factors for each value between `self` and `other`.
    pub fn diff(&self, other: &Self) -> RoundStatsDiff {
        RoundStatsDiff {
            duration: diff_percent(self.duration.as_secs_f32(), other.duration.as_secs_f32()),
            rmr: diff_percent(self.rmr, other.rmr),
            ldh: diff_percent(self.ldh, other.ldh),
            missed: diff_percent(self.missed, other.missed),
        }
    }
}

fn diff_percent(a: f32, b: f32) -> f32 {
    if a == 0.0 && b == 0.0 {
        0.0
    } else if b == 0.0 {
        -1.0
    } else if a == 0.0 {
        1.0
    } else {
        (b - a) / a
    }
}

/// Summary values for a list of [`RoundStats`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RoundStatsAvg {
    /// The number of rounds for which this average is calculated.
    pub len: usize,
    /// The minimum values of the list.
    pub min: RoundStats,
    /// The maximum values of the list.
    pub max: RoundStats,
    /// The mean values of the list.
    pub mean: RoundStats,
}

/// Difference, in factors, between two [`RoundStatsAvg`]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RoundStatsAvgDiff {
    /// The difference, as factors, in the minimums.
    pub min: RoundStatsDiff,
    /// The difference, as factors, in the maximumx.
    pub max: RoundStatsDiff,
    /// The difference, as factors, in the mean values.
    pub mean: RoundStatsDiff,
}

impl RoundStatsAvg {
    /// Calculates the difference, as factors, between `self` and `other`.
    pub fn diff(&self, other: &Self) -> RoundStatsAvgDiff {
        RoundStatsAvgDiff {
            min: self.min.diff(&other.min),
            max: self.max.diff(&other.max),
            mean: self.mean.diff(&other.mean),
        }
    }

    /// Calculates the average between a list of [`RoundStatsAvg`].
    pub fn avg(rows: &[Self]) -> Self {
        let len = rows.iter().map(|row| row.len).sum();
        let min = RoundStats::min(rows.iter().map(|x| &x.min));
        let max = RoundStats::min(rows.iter().map(|x| &x.max));
        let mean = RoundStats::min(rows.iter().map(|x| &x.mean));
        Self {
            min,
            max,
            mean,
            len,
        }
    }
}

/// Histograms on the distribution of peers in the network.
///
/// For each field, the map's key is the bucket value, and the map's value is
/// the number of peers that fall into that bucket.
#[derive(Debug, Default)]
pub struct NetworkHistograms {
    /// Distribution of active view (neighbor) counts.
    pub active: BTreeMap<usize, usize>,
    /// Distribution of passive view counts.
    pub passive: BTreeMap<usize, usize>,
    /// Distribution of eager peer counts.
    pub eager: BTreeMap<usize, usize>,
    /// Distribution of lazy peer counts.
    pub lazy: BTreeMap<usize, usize>,
}

fn avg(map: &BTreeMap<usize, usize>) -> f32 {
    let (sum, count) = map
        .iter()
        .fold((0, 0), |(sum, count), (k, v)| (sum + k * v, count + v));
    if count != 0 {
        sum as f32 / count as f32
    } else {
        0.
    }
}

fn min(map: &BTreeMap<usize, usize>) -> usize {
    map.first_key_value().map(|(k, _v)| *k).unwrap_or_default()
}

fn max(map: &BTreeMap<usize, usize>) -> usize {
    map.last_key_value().map(|(k, _v)| *k).unwrap_or_default()
}

impl fmt::Display for NetworkHistograms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "    eager {:?}\n    lazy {:?}\n    active {:?}\n    passive {:?}",
            self.eager, self.lazy, self.active, self.passive
        )
    }
}

/// A report on the state of a [`Network`].
#[derive(Debug)]
pub struct NetworkReport<PI> {
    /// The number of peers in the network.
    pub peer_count: usize,
    /// List of peers that don't have any neighbors.
    pub peers_without_neighbors: Vec<PI>,
    /// Histograms of peer distribution metrics.
    pub histograms: NetworkHistograms,
}

impl<PI> NetworkReport<PI> {
    /// Returns `true` if the network contains peers that have no active neighbors.
    pub fn has_peers_with_no_neighbors(&self) -> bool {
        *self.histograms.active.get(&0).unwrap_or(&0) > 0
    }
}

impl<PI> fmt::Display for NetworkReport<PI> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "peers: {}\n{}", self.peer_count, self.histograms)?;
        if self.peers_without_neighbors.is_empty() {
            writeln!(f, "(all have neighbors)")
        } else {
            writeln!(
                f,
                "({} peers have no neighbors)",
                self.peers_without_neighbors.len()
            )
        }
    }
}

const TOPIC: TopicId = TopicId::from_bytes([0u8; 32]);

/// A simulator for the gossip protocol
#[derive(Debug)]
pub struct Simulator {
    /// Configuration of the simulator.
    pub config: SimulatorConfig,
    /// The [`Network`]
    pub network: Network<PeerId, rand::rngs::ChaCha12Rng>,
    /// List of [`RoundStats`] of all previous rounds.
    round_stats: Vec<RoundStats>,
}

impl Simulator {
    /// Creates a new simulator.
    pub fn new(
        simulator_config: SimulatorConfig,
        network_config: impl Into<NetworkConfig>,
    ) -> Self {
        let network_config = network_config.into();
        info!("start {simulator_config:?} {network_config:?}");
        let rng = rand::rngs::ChaCha12Rng::seed_from_u64(simulator_config.rng_seed);
        Self {
            network: Network::new(network_config, rng),
            config: simulator_config,
            round_stats: Default::default(),
        }
    }

    /// Creates a new random number generator, derived from the simuator's RNG.
    pub fn rng(&mut self) -> ChaCha12Rng {
        ChaCha12Rng::from_rng(&mut self.network.rng)
    }

    /// Returns the peer id of a random peer.
    pub fn random_peer(&mut self) -> PeerId {
        *self
            .network
            .peers
            .keys()
            .choose(&mut self.network.rng)
            .unwrap()
    }

    /// Returns the number of peers.
    pub fn peer_count(&self) -> usize {
        self.network.peers.len()
    }

    /// Removes `n` peers from the network.
    pub fn remove_peers(&mut self, n: usize) {
        for _i in 0..n {
            let key = self.random_peer();
            self.network.remove(&key);
        }
    }

    /// Returns a report on the current state of the network.
    pub fn report(&mut self) -> NetworkReport<PeerId> {
        let report = self.network.report();
        let min_active_len = min(&report.histograms.active);
        let max_active_len = max(&report.histograms.active);
        let avg = avg(&report.histograms.active);
        let len = report.peer_count;
        debug!(
            "nodes {len} active: avg {avg:2.2} min {min_active_len} max {max_active_len} empty {}",
            report.peers_without_neighbors.len()
        );
        report
    }

    /// Bootstraps the network.
    ///
    /// See [`BootstrapMode`] for details.
    ///
    /// Returns the [`NetworkReport`] after finishing the bootstrap.
    pub fn bootstrap(&mut self, bootstrap_mode: BootstrapMode) -> NetworkReport<PeerId> {
        let span = info_span!("bootstrap");
        let _guard = span.enter();
        info!("bootstrap {bootstrap_mode:?}");

        let node_count = self.config.peers as u64;

        match bootstrap_mode {
            BootstrapMode::Single => {
                self.network.insert_and_join(0, TOPIC, vec![]);
                for i in 1..node_count {
                    self.network.insert_and_join(i, TOPIC, vec![0]);
                }
                self.network.run_trips(20);
            }
            BootstrapMode::Set { count } => {
                self.network.insert_and_join(0, TOPIC, vec![]);
                for i in 1..count {
                    self.network.insert_and_join(i, TOPIC, vec![0]);
                }

                self.network.run_trips(7);

                for i in count..node_count {
                    let contact = self.network.rng.random_range(0..count);
                    self.network.insert_and_join(i, TOPIC, vec![contact]);
                }

                self.network.run_trips(20);
            }
        }

        let report = self.report();
        if report.has_peers_with_no_neighbors() {
            warn!("failed to keep all nodes active after warmup: {report:?}");
        } else {
            info!("bootstrap complete, all nodes active");
        }
        report
    }

    /// Runs a round of gossiping.
    ///
    /// `messages` is a list of `(sender, message)` pairs. All messages will be sent simultaneously.
    /// The round will run until all peers received all messages, or until [`SimulatorConfig::gossip_round_timeout`]
    /// is elapsed.
    ///
    /// Returns the number of undelivered messages.
    pub fn gossip_round(&mut self, messages: Vec<(PeerId, Bytes)>) -> usize {
        let span = debug_span!("g", r = self.round_stats.len());
        let _guard = span.enter();
        self.reset_stats();
        let start = self.network.time;
        let expected_count: usize = messages.len() * (self.network.peers.len() - 1);
        info!(
            time=%self.network.elapsed_fmt(),
            "round {i}: send {len} messages / recv {expected_count} total",
            len = messages.len(),
            i = self.round_stats.len()
        );

        // Send messages and keep track of expected receives
        let mut missing: BTreeMap<PeerId, BTreeSet<Bytes>> = BTreeMap::new();
        for (from, message) in messages {
            for peer in self.network.peer_ids().filter(|p| *p != from) {
                missing.entry(peer).or_default().insert(message.clone());
            }
            self.network
                .command(from, TOPIC, Command::Broadcast(message, Scope::Swarm));
        }

        let timeout = self.config.gossip_round_timeout;
        let mut received_count = 0;
        self.network
            .run_while_with_timeout(timeout, |peer, _topic, event| {
                if let Event::Received(message) = event {
                    let set = missing.get_mut(&peer).unwrap();
                    if !set.remove(&message.content) {
                        panic!("received duplicate message event");
                    } else if set.is_empty() {
                        missing.remove(&peer);
                    }
                    received_count += 1;
                }
                received_count != expected_count
            });

        let missing_count: usize = expected_count - received_count;
        if missing_count == 0 {
            info!("break: all messages received by all peers");
        } else {
            warn!("break: max ticks for round exceeded (still missing {missing_count})");
            debug!("missing: {missing:?}");
        }
        let elapsed = self.network.time.duration_since(start);
        self.report_gossip_round(expected_count, missing_count, elapsed);
        missing_count
    }

    fn report_gossip_round(
        &mut self,
        expected_recv_count: usize,
        missed: usize,
        duration: Duration,
    ) {
        let payloud_msg_count = self.total_payload_messages();
        let ctrl_msg_count = self.total_control_messages();
        let rmr_expected_count = expected_recv_count - missed;
        let rmr = (payloud_msg_count as f32 / (rmr_expected_count as f32 - 1.)) - 1.;
        let ldh = self.max_ldh();

        let round_stats = RoundStats {
            duration,
            rmr,
            ldh: ldh as f32,
            missed: missed as f32,
        };
        let histograms = self.network.report().histograms;
        info!(
            "round {}: pay {} ctrl {} {round_stats} \n{histograms}",
            self.round_stats.len(),
            payloud_msg_count,
            ctrl_msg_count,
        );
        self.round_stats.push(round_stats);
    }

    /// Calculates the [`RoundStatsAvg`] of all gossip rounds.
    pub fn round_stats_average(&self) -> RoundStatsAvg {
        RoundStats::avg(&self.round_stats)
    }

    fn reset_stats(&mut self) {
        for state in self.network.peers.values_mut() {
            state.reset_stats(&TOPIC);
        }
    }

    fn max_ldh(&self) -> u16 {
        let mut max = 0;
        for state in self.network.peers.values() {
            let state = state.state(&TOPIC).unwrap();
            let stats = state.gossip.stats();
            max = max.max(stats.max_last_delivery_hop);
        }
        max
    }

    fn total_payload_messages(&self) -> u64 {
        let mut sum = 0;
        for state in self.network.peers.values() {
            let state = state.state(&TOPIC).unwrap();
            let stats = state.gossip.stats();
            sum += stats.payload_messages_received;
        }
        sum
    }

    fn total_control_messages(&self) -> u64 {
        let mut sum = 0;
        for state in self.network.peers.values() {
            let state = state.state(&TOPIC).unwrap();
            let stats = state.gossip.stats();
            sum += stats.control_messages_received;
        }
        sum
    }
}

fn add_one(map: &mut BTreeMap<usize, usize>, key: usize) {
    let entry = map.entry(key).or_default();
    *entry += 1;
}

/// Helper struct for active connections. A sorted tuple.
#[derive(Debug, Clone, PartialOrd, Ord, Eq, PartialEq, Hash)]
struct ConnId<PI>([PI; 2]);
impl<PI: Ord + Copy> ConnId<PI> {
    fn new(a: PI, b: PI) -> Self {
        let mut conn = [a, b];
        conn.sort();
        Self(conn)
    }
    fn peers(&self) -> [PI; 2] {
        self.0
    }

    fn other(&self, other: PI) -> Option<PI> {
        if self.0[0] == other {
            Some(self.0[1])
        } else if self.0[1] == other {
            Some(self.0[0])
        } else {
            None
        }
    }
}
impl<PI: Ord + Copy> From<(PI, PI)> for ConnId<PI> {
    fn from((a, b): (PI, PI)) -> Self {
        Self::new(a, b)
    }
}
impl<PI: Copy> From<ConnId<PI>> for (PI, PI) {
    fn from(conn: ConnId<PI>) -> (PI, PI) {
        (conn.0[0], conn.0[1])
    }
}

fn sort<T: Ord + Clone>(items: Vec<T>) -> Vec<T> {
    let mut sorted = items;
    sorted.sort();
    sorted
}

fn read_var<T: FromStr<Err: fmt::Display + fmt::Debug>>(name: &str, default: T) -> T {
    std::env::var(name)
        .map(|x| {
            x.parse()
                .unwrap_or_else(|_| panic!("Failed to parse environment variable {name}"))
        })
        .unwrap_or(default)
}
