use super::{addressbook, protocol};

use super::peerbook::{self};
use serde::{Deserialize, Serialize};

use crate::repo::DefaultStorage;
use crate::{IntoAddPeerOpt, IpfsOptions};

use crate::repo::Repo;

use ipld_core::cid::Cid;

use connexa::prelude::dht::Record;
use connexa::prelude::identity::{Keypair, PublicKey};
#[cfg(not(target_arch = "wasm32"))]
use connexa::prelude::relay;
use connexa::prelude::swarm::NetworkBehaviour;
use connexa::prelude::swarm::behaviour::toggle::Toggle;
use connexa::prelude::{Multiaddr, PeerId, identify};

use std::fmt::Debug;
use std::num::NonZeroU32;
use std::time::Duration;

/// Behaviour type.
#[derive(NetworkBehaviour)]
#[behaviour(prelude = "connexa::prelude::swarm::derive_prelude")]
pub struct Behaviour<C>
where
    C: NetworkBehaviour,
    <C as NetworkBehaviour>::ToSwarm: Debug + Send,
{
    pub addressbook: addressbook::Behaviour,

    pub bitswap: Toggle<super::bitswap::Behaviour>,

    // custom behaviours
    pub custom: Toggle<C>,

    // misc
    pub peerbook: peerbook::Behaviour,
    pub protocol: protocol::Behaviour,
}

/// Represents the result of a Kademlia query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KadResult {
    /// The query has been exhausted.
    Complete,
    /// The query successfully returns `GetClosestPeers` or `GetProviders` results.
    Peers(Vec<PeerId>),
    /// The query successfully returns a `GetRecord` result.
    Records(Vec<Record>),
    Record(Record),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelayConfig {
    pub max_reservations: usize,
    pub max_reservations_per_peer: usize,
    pub reservation_duration: Duration,
    pub reservation_rate_limiters: Vec<RateLimit>,

    pub max_circuits: usize,
    pub max_circuits_per_peer: usize,
    pub max_circuit_duration: Duration,
    pub max_circuit_bytes: u64,
    pub circuit_src_rate_limiters: Vec<RateLimit>,
}

impl Default for RelayConfig {
    fn default() -> Self {
        let limiters = vec![
            RateLimit::PerPeer {
                limit: NonZeroU32::new(30).expect("30 > 0"),
                interval: Duration::from_secs(60 * 2),
            },
            RateLimit::PerIp {
                limit: NonZeroU32::new(60).expect("60 > 0"),
                interval: Duration::from_secs(60),
            },
        ];
        Self {
            max_reservations: 128,
            max_reservations_per_peer: 4,
            reservation_duration: Duration::from_secs(60 * 60),
            reservation_rate_limiters: limiters.clone(),
            max_circuits: 16,
            max_circuits_per_peer: 4,
            max_circuit_duration: Duration::from_secs(2 * 60),
            max_circuit_bytes: 1 << 17,
            circuit_src_rate_limiters: limiters,
        }
    }
}

impl RelayConfig {
    /// Configuration to allow a connection to the relay without limits
    pub fn unbounded() -> Self {
        Self {
            max_circuits: usize::MAX,
            max_circuit_bytes: u64::MAX,
            max_circuit_duration: Duration::MAX,
            max_circuits_per_peer: usize::MAX,
            max_reservations: usize::MAX,
            reservation_duration: Duration::MAX,
            max_reservations_per_peer: usize::MAX,
            reservation_rate_limiters: vec![],
            circuit_src_rate_limiters: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IdentifyConfiguration {
    pub protocol_version: String,
    pub agent_version: String,
    pub interval: Duration,
    pub push_update: bool,
    pub cache: usize,
}

impl Default for IdentifyConfiguration {
    fn default() -> Self {
        Self {
            protocol_version: "/ipfs/0.1.0".into(),
            agent_version: "rust-ipfs".into(),
            interval: Duration::from_secs(5 * 60),
            push_update: true,
            cache: 100,
        }
    }
}

impl IdentifyConfiguration {
    pub fn into(self, publuc_key: PublicKey) -> identify::Config {
        identify::Config::new(self.protocol_version, publuc_key)
            .with_agent_version(self.agent_version)
            .with_interval(self.interval)
            .with_push_listen_addr_updates(self.push_update)
            .with_cache_size(self.cache)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<RelayConfig> for relay::server::Config {
    fn from(
        RelayConfig {
            max_reservations,
            max_reservations_per_peer,
            reservation_duration,
            max_circuits,
            max_circuits_per_peer,
            max_circuit_duration,
            max_circuit_bytes,
            reservation_rate_limiters,
            circuit_src_rate_limiters,
        }: RelayConfig,
    ) -> Self {
        let reservation_duration = max_duration(reservation_duration);
        let max_circuit_duration = max_duration(max_circuit_duration);

        let mut config = relay::server::Config {
            max_reservations,
            max_reservations_per_peer,
            reservation_duration,
            max_circuits,
            max_circuits_per_peer,
            max_circuit_duration,
            max_circuit_bytes,
            ..Default::default()
        };

        for rate in circuit_src_rate_limiters {
            match rate {
                RateLimit::PerPeer { limit, interval } => {
                    config = config.circuit_src_per_peer(limit, interval);
                }
                RateLimit::PerIp { limit, interval } => {
                    config = config.circuit_src_per_ip(limit, interval);
                }
            }
        }

        for rate in reservation_rate_limiters {
            match rate {
                RateLimit::PerPeer { limit, interval } => {
                    config = config.reservation_rate_per_peer(limit, interval);
                }
                RateLimit::PerIp { limit, interval } => {
                    config = config.reservation_rate_per_ip(limit, interval);
                }
            }
        }

        config
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn max_duration(duration: Duration) -> Duration {
    let start = web_time::Instant::now();
    if start.checked_add(duration).is_none() {
        return Duration::from_secs(u32::MAX as _);
    }
    duration
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RateLimit {
    PerPeer {
        limit: NonZeroU32,
        interval: Duration,
    },
    PerIp {
        limit: NonZeroU32,
        interval: Duration,
    },
}
impl<C> Behaviour<C>
where
    C: NetworkBehaviour,
    <C as NetworkBehaviour>::ToSwarm: Debug + Send,
{
    pub(crate) fn new(
        keypair: &Keypair,
        options: &IpfsOptions,
        repo: &Repo<DefaultStorage>,
        custom: Option<C>,
    ) -> Self {
        let bootstrap = options.bootstrap.clone();

        let protocols = options.protocols;

        let peer_id = keypair.public().to_peer_id();

        info!("net: starting with peer id {}", peer_id);

        let bitswap = protocols
            .bitswap
            .then(|| {
                let config = (options.bitswap_config)(super::bitswap::Config::default());
                super::bitswap::Behaviour::new(repo, config)
            })
            .into();

        let peerbook = peerbook::Behaviour::default();

        let addressbook = addressbook::Behaviour::with_config(options.addr_config);

        let protocol = protocol::Behaviour::default();
        let custom = Toggle::from(custom);

        let mut behaviour = Behaviour {
            bitswap,
            peerbook,
            addressbook,
            protocol,
            custom,
        };

        for addr in bootstrap {
            let Ok(mut opt) = IntoAddPeerOpt::into_opt(addr) else {
                continue;
            };

            // explicitly dial the bootstrap peer. If the peer will be bootstrapped via kad, the additional dial will be cancelled
            opt = opt.set_dial(true);

            _ = behaviour.add_peer(opt);
        }

        behaviour
    }

    pub fn add_peer<I: IntoAddPeerOpt>(&mut self, opt: I) -> bool {
        let opt = opt.into_opt().expect("valid entries");

        self.addressbook.add_address(opt);

        true
    }

    pub fn remove_peer(&mut self, peer: &PeerId) {
        self.addressbook.remove_peer(peer);
    }

    pub fn addrs(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        self.peerbook.connected_peers_addrs().collect()
    }

    // TODO
    pub fn stop_providing_block(&mut self, cid: &Cid) {
        info!("Finished providing block {}", cid.to_string());
        let _key = cid.hash().to_bytes();
        // if let Some(kad) = self.kademlia.as_mut() {
        //     kad.stop_providing(&key.into());
        // }
    }

    pub fn supported_protocols(&self) -> Vec<String> {
        self.protocol.iter().collect::<Vec<_>>()
    }
}
