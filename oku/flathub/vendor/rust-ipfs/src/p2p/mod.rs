//! P2P handling for IPFS nodes.

use crate::IpfsOptions;
use crate::repo::DefaultStorage;
use crate::repo::Repo;
use connexa::behaviour::peer_store::store::memory::MemoryStore;

pub use behaviour::Behaviour;
pub use behaviour::IdentifyConfiguration;
pub use behaviour::{RateLimit, RelayConfig};
use connexa::prelude::Multiaddr;
use connexa::prelude::PeerId;
use connexa::prelude::StreamProtocol;
use connexa::prelude::gossipsub::ValidationMode;
use connexa::prelude::identify::Info as IdentifyInfo;
use connexa::prelude::identity::Keypair;
use connexa::prelude::identity::PublicKey;
use connexa::prelude::swarm::NetworkBehaviour;
use connexa::prelude::swarm::Swarm;

pub(crate) mod addr;
pub(crate) mod addressbook;
pub mod bitswap;
pub(crate) mod peerbook;
pub mod protocol;

mod behaviour;
pub use self::addressbook::Config as AddressBookConfig;
pub use self::behaviour::BehaviourEvent;

pub use addr::MultiaddrExt;
pub use behaviour::KadResult;

pub(crate) type TSwarm<C> =
    Swarm<connexa::behaviour::Behaviour<behaviour::Behaviour<C>, MemoryStore>>;

/// Abstraction of IdentifyInfo but includes PeerId
#[derive(Clone, Debug, Eq)]
pub struct PeerInfo {
    /// The peer id of the user
    pub peer_id: PeerId,

    /// The public key of the local peer.
    pub public_key: PublicKey,

    /// Application-specific version of the protocol family used by the peer,
    /// e.g. `ipfs/1.0.0` or `polkadot/1.0.0`.
    pub protocol_version: String,

    /// Name and version of the peer, similar to the `User-Agent` header in
    /// the HTTP protocol.
    pub agent_version: String,

    /// The addresses that the peer is listening on.
    pub listen_addrs: Vec<Multiaddr>,

    /// The list of protocols supported by the peer, e.g. `/ipfs/ping/1.0.0`.
    pub protocols: Vec<StreamProtocol>,

    /// Address observed by or for the remote.
    pub observed_addr: Option<Multiaddr>,
}

impl core::hash::Hash for PeerInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.peer_id.hash(state);
        self.public_key.hash(state);
    }
}

impl PartialEq for PeerInfo {
    fn eq(&self, other: &Self) -> bool {
        self.peer_id == other.peer_id && self.public_key == other.public_key
    }
}

impl From<IdentifyInfo> for PeerInfo {
    fn from(info: IdentifyInfo) -> Self {
        let IdentifyInfo {
            public_key,
            protocol_version,
            agent_version,
            listen_addrs,
            protocols,
            observed_addr,
            ..
        } = info;
        let peer_id = public_key.to_peer_id();
        let observed_addr = Some(observed_addr);
        Self {
            peer_id,
            public_key,
            protocol_version,
            agent_version,
            listen_addrs,
            protocols,
            observed_addr,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubsubConfig {
    /// Custom protocol name
    pub custom_protocol_id: Option<String>,

    /// Max size that can be transmitted over gossipsub
    pub max_transmit_size: usize,

    /// Floodsub compatibility
    pub floodsub_compat: bool,

    /// Validation
    pub validate: PubsubValidation,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PubsubValidation {
    /// See [`ValidationMode::Strict`]
    Strict,

    /// See [`ValidationMode::Permissive`]
    Permissive,

    /// See [`ValidationMode::Anonymous`]
    Anonymous,

    /// See [`ValidationMode::None`]
    Relaxed,
}

impl From<PubsubValidation> for ValidationMode {
    fn from(validation: PubsubValidation) -> Self {
        match validation {
            PubsubValidation::Strict => ValidationMode::Strict,
            PubsubValidation::Permissive => ValidationMode::Permissive,
            PubsubValidation::Anonymous => ValidationMode::Anonymous,
            PubsubValidation::Relaxed => ValidationMode::None,
        }
    }
}

impl Default for PubsubConfig {
    fn default() -> Self {
        Self {
            custom_protocol_id: None,
            max_transmit_size: 2 * 1024 * 1024,
            validate: PubsubValidation::Strict,
            floodsub_compat: false,
        }
    }
}

/// Construct new behaviour
pub(crate) fn create_create_behaviour<C>(
    keypair: &Keypair,
    options: &IpfsOptions,
    repo: &Repo<DefaultStorage>,
    custom: Option<C>,
) -> behaviour::Behaviour<C>
where
    C: NetworkBehaviour,
    C: Send + Sync + 'static,
    <C as NetworkBehaviour>::ToSwarm: std::fmt::Debug + Send + Sync + 'static,
{
    behaviour::Behaviour::new(keypair, options, repo, custom)
}
