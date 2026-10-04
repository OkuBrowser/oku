#[cfg(feature = "relay")]
pub mod autorelay;
pub mod dummy;
pub mod peer_store;
#[cfg(feature = "request-response")]
pub mod request_response;
#[cfg(feature = "request-response")]
mod rr_man;

#[cfg(feature = "autonat")]
use libp2p::autonat;
#[cfg(feature = "dcutr")]
#[cfg(not(target_arch = "wasm32"))]
use libp2p::dcutr::Behaviour as Dcutr;
#[cfg(feature = "identify")]
use libp2p::identify::Behaviour as Identify;
#[cfg(feature = "kad")]
use libp2p::kad::Behaviour as Kademlia;
#[cfg(feature = "kad")]
use libp2p::kad::store::MemoryStore;
#[cfg(feature = "mdns")]
#[cfg(not(target_arch = "wasm32"))]
use libp2p::mdns::tokio::Behaviour as Mdns;
#[cfg(feature = "ping")]
use libp2p::ping::Behaviour as Ping;
#[cfg(feature = "relay")]
use libp2p::relay::client::Behaviour as RelayClient;
#[cfg(feature = "relay")]
use libp2p::relay::{Behaviour as RelayServer, client::Transport as ClientTransport};
#[cfg(not(feature = "relay"))]
type ClientTransport = ();

use libp2p::swarm::behaviour::toggle::Toggle;

use crate::behaviour::peer_store::store::Store;
use crate::builder::{Config, Protocols};
use crate::error::{ConnexaResult, Error};
use libp2p::identity::Keypair;
use libp2p::swarm::NetworkBehaviour;
use libp2p_allow_block_list::{AllowedPeers, BlockedPeers};
use std::fmt::Debug;

#[derive(NetworkBehaviour)]
pub struct Behaviour<C, S>
where
    S: Store,
{
    // connection management
    pub allow_list: Toggle<libp2p_allow_block_list::Behaviour<AllowedPeers>>,
    pub deny_list: Toggle<libp2p_allow_block_list::Behaviour<BlockedPeers>>,
    pub connection_limits: Toggle<libp2p_connection_limits::Behaviour>,

    pub peer_store: Toggle<peer_store::Behaviour<S>>,

    #[cfg(feature = "relay")]
    // networking
    pub relay: Toggle<RelayServer>,
    #[cfg(feature = "relay")]
    pub relay_client: Toggle<RelayClient>,

    #[cfg(feature = "relay")]
    pub autorelay: Toggle<autorelay::Behaviour>,

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "upnp")]
    pub upnp: Toggle<libp2p::upnp::tokio::Behaviour>,
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "dcutr")]
    pub dcutr: Toggle<Dcutr>,

    // discovery
    #[cfg(feature = "rendezvous")]
    pub rendezvous_client: Toggle<libp2p::rendezvous::client::Behaviour>,
    #[cfg(feature = "rendezvous")]
    pub rendezvous_server: Toggle<libp2p::rendezvous::server::Behaviour>,
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "mdns")]
    pub mdns: Toggle<Mdns>,
    #[cfg(feature = "kad")]
    pub kademlia: Toggle<Kademlia<MemoryStore>>,

    #[cfg(feature = "identify")]
    pub identify: Toggle<Identify>,

    #[cfg(feature = "gossipsub")]
    pub gossipsub: Toggle<libp2p::gossipsub::Behaviour>,
    #[cfg(feature = "floodsub")]
    pub floodsub: Toggle<libp2p::floodsub::Behaviour>,

    #[cfg(feature = "ping")]
    pub ping: Toggle<Ping>,
    #[cfg(feature = "stream")]
    pub stream: Toggle<libp2p_stream::Behaviour>,

    #[cfg(feature = "autonat")]
    pub autonat_v1: Toggle<autonat::v1::Behaviour>,
    #[cfg(feature = "autonat")]
    pub autonat_v2_client: Toggle<autonat::v2::client::Behaviour>,
    #[cfg(feature = "autonat")]
    pub autonat_v2_server: Toggle<autonat::v2::server::Behaviour>,

    // TODO: Write a macro or behaviour to support multiple request-response behaviour
    #[cfg(feature = "request-response")]
    pub rr_man: Toggle<rr_man::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_1: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_2: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_3: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_4: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_5: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_6: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_7: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_8: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_9: Toggle<request_response::Behaviour>,
    #[cfg(feature = "request-response")]
    pub rr_0: Toggle<request_response::Behaviour>,

    // custom behaviour
    pub custom: Toggle<C>,
}

impl<C, S> Behaviour<C, S>
where
    C: NetworkBehaviour,
    <C as NetworkBehaviour>::ToSwarm: Debug + Send,
    S: Store,
{
    pub(crate) fn new(
        keypair: &Keypair,
        custom_behaviour: Option<C>,
        config: Config<S>,
        protocols: Protocols,
    ) -> ConnexaResult<(Self, Option<ClientTransport>)> {
        if protocols.allow_list && protocols.deny_list {
            return Err(Error::InvalidConfig(
                "blocklist and whitelist cannot be enabled at the same time".into(),
            ));
        }

        let peer_id = keypair.public().to_peer_id();

        tracing::info!("net: starting with peer id {}", peer_id);

        #[cfg(feature = "mdns")]
        #[cfg(not(target_arch = "wasm32"))]
        let mdns = protocols
            .mdns
            .then(|| Mdns::new(Default::default(), peer_id))
            .transpose()?
            .into();

        #[cfg(feature = "kad")]
        let kademlia: Toggle<Kademlia<MemoryStore>> = match protocols.kad {
            true => {
                let (protocol, config_fn) = config.kademlia_config;
                let protocol = libp2p::StreamProtocol::try_from_owned(protocol)
                    .map_err(std::io::Error::other)?;
                let config = config_fn(libp2p::kad::Config::new(protocol));
                Toggle::from(Some(Kademlia::with_config(
                    peer_id,
                    MemoryStore::with_config(peer_id, Default::default()),
                    config,
                )))
            }
            false => Toggle::from(None),
        };

        #[cfg(feature = "autonat")]
        let autonat_v1 = protocols
            .autonat_v1
            .then(|| {
                let config_fn = config.autonat_v1_config;
                let config = config_fn(Default::default());
                autonat::Behaviour::new(peer_id, config)
            })
            .into();

        #[cfg(feature = "autonat")]
        let autonat_v2_client = protocols
            .autonat_v2_client
            .then(|| {
                let config_fn = config.autonat_v2_client_config;
                let config = config_fn(Default::default());
                autonat::v2::client::Behaviour::new(rand::make_rng(), config)
            })
            .into();

        #[cfg(feature = "autonat")]
        let autonat_v2_server = protocols
            .autonat_v2_server
            .then(autonat::v2::server::Behaviour::default)
            .into();

        #[cfg(feature = "ping")]
        let ping = protocols
            .ping
            .then(|| {
                let config_fn = config.ping_config;
                let config = config_fn(Default::default());
                Ping::new(config)
            })
            .into();

        #[cfg(feature = "identify")]
        let identify = protocols
            .identify
            .then(|| {
                let pubkey = keypair.public();
                let (protocol, config_fn) = config.identify_config;
                let config = (config_fn)(libp2p::identify::Config::new(protocol, pubkey));
                Identify::new(config)
            })
            .into();

        #[cfg(feature = "gossipsub")]
        let gossipsub = match protocols.gossipsub {
            true => {
                let config_fn = config.gossipsub_config;
                let (config_builder, auth) =
                    config_fn(keypair, libp2p::gossipsub::ConfigBuilder::default());
                let config = config_builder.build().map_err(std::io::Error::other)?;
                let behaviour = libp2p::gossipsub::Behaviour::new(auth, config)
                    .map_err(std::io::Error::other)?;
                Toggle::from(Some(behaviour))
            }
            false => Toggle::from(None),
        };

        #[cfg(feature = "floodsub")]
        let floodsub = protocols
            .floodsub
            .then(|| {
                let config_fn = config.floodsub_config;
                let config = config_fn(libp2p::floodsub::Config::new(peer_id));

                libp2p::floodsub::Behaviour::from_config(config)
            })
            .into();

        #[cfg(not(target_arch = "wasm32"))]
        #[cfg(feature = "dcutr")]
        let dcutr = protocols.dcutr.then(|| Dcutr::new(peer_id)).into();

        #[cfg(feature = "relay")]
        let relay = protocols
            .relay_server
            .then(|| {
                let config_fn = config.relay_server_config;
                let config = config_fn(Default::default());
                RelayServer::new(peer_id, config)
            })
            .into();

        #[cfg(not(target_arch = "wasm32"))]
        #[cfg(feature = "upnp")]
        let upnp = protocols
            .upnp
            .then(libp2p::upnp::tokio::Behaviour::default)
            .into();

        #[cfg(feature = "relay")]
        let (transport, relay_client) = match protocols.relay_client {
            true => {
                let (transport, client) = libp2p::relay::client::new(peer_id);
                (Some(transport), Some(client).into())
            }
            false => (None, None.into()),
        };
        #[cfg(feature = "relay")]
        let autorelay = protocols
            .autorelay
            .then(|| {
                let config_fn = config.autorelay_config;
                let config = config_fn(autorelay::Config::default());
                autorelay::Behaviour::new_with_config(config)
            })
            .into();

        #[cfg(not(feature = "relay"))]
        let transport = None::<()>;

        let custom = Toggle::from(custom_behaviour);

        #[cfg(feature = "rendezvous")]
        let rendezvous_client = protocols
            .rendezvous_client
            .then(|| libp2p::rendezvous::client::Behaviour::new(keypair.clone()))
            .into();

        #[cfg(feature = "rendezvous")]
        let rendezvous_server = protocols
            .rendezvous_server
            .then(|| libp2p::rendezvous::server::Behaviour::new(Default::default()))
            .into();

        #[cfg(feature = "stream")]
        let stream = protocols.streams.then(libp2p_stream::Behaviour::new).into();

        let connection_limits = protocols
            .connection_limits
            .then(|| {
                let config_fn = config.connection_limits;
                config_fn(Default::default())
            })
            .map(libp2p_connection_limits::Behaviour::new)
            .into();

        let allow_list = protocols
            .allow_list
            .then(|| {
                let mut behaviour = libp2p_allow_block_list::Behaviour::<AllowedPeers>::default();
                for peer_id in config.allow_list {
                    behaviour.allow_peer(peer_id);
                }
                behaviour
            })
            .into();

        let deny_list = protocols
            .deny_list
            .then(|| {
                let mut behaviour = libp2p_allow_block_list::Behaviour::<BlockedPeers>::default();
                for peer_id in config.deny_list {
                    behaviour.block_peer(peer_id);
                }
                behaviour
            })
            .into();

        let peer_store = protocols
            .peer_store
            .then_some(config.peer_store)
            .map(Option::unwrap)
            .map(peer_store::Behaviour::new)
            .into();

        #[allow(unused_mut)]
        let mut behaviour = Behaviour {
            allow_list,
            deny_list,
            connection_limits,
            peer_store,
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(feature = "mdns")]
            mdns,
            #[cfg(feature = "kad")]
            kademlia,
            #[cfg(feature = "ping")]
            ping,
            #[cfg(feature = "identify")]
            identify,
            #[cfg(feature = "autonat")]
            autonat_v1,
            #[cfg(feature = "autonat")]
            autonat_v2_client,
            #[cfg(feature = "autonat")]
            autonat_v2_server,
            #[cfg(feature = "gossipsub")]
            gossipsub,
            #[cfg(feature = "floodsub")]
            floodsub,
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(feature = "dcutr")]
            dcutr,
            #[cfg(feature = "relay")]
            relay,
            #[cfg(feature = "relay")]
            relay_client,
            #[cfg(feature = "relay")]
            autorelay,
            #[cfg(feature = "stream")]
            stream,
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(feature = "upnp")]
            upnp,
            custom,
            #[cfg(feature = "rendezvous")]
            rendezvous_client,
            #[cfg(feature = "rendezvous")]
            rendezvous_server,
            #[cfg(feature = "request-response")]
            rr_man: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_0: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_1: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_2: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_3: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_4: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_5: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_6: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_7: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_8: Toggle::from(None),
            #[cfg(feature = "request-response")]
            rr_9: Toggle::from(None),
        };

        #[cfg(feature = "request-response")]
        {
            let mut existing_protocol: indexmap::IndexMap<libp2p::StreamProtocol, _> =
                indexmap::IndexMap::new();

            for (index, config) in config.request_response_config.iter().enumerate() {
                let protocol = libp2p::StreamProtocol::try_from_owned(config.protocol.clone())
                    .map_err(std::io::Error::other)?;
                if existing_protocol.contains_key(&protocol) {
                    tracing::warn!(%protocol, "request-response protocol is already registered");
                    continue;
                };

                match index {
                    0 => {
                        if behaviour.rr_0.is_enabled() {
                            continue;
                        }
                        behaviour.rr_0 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    1 => {
                        if behaviour.rr_1.is_enabled() {
                            continue;
                        }
                        behaviour.rr_1 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    2 => {
                        if behaviour.rr_2.is_enabled() {
                            continue;
                        }
                        behaviour.rr_2 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    3 => {
                        if behaviour.rr_3.is_enabled() {
                            continue;
                        }
                        behaviour.rr_3 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    4 => {
                        if behaviour.rr_4.is_enabled() {
                            continue;
                        }
                        behaviour.rr_4 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    5 => {
                        if behaviour.rr_5.is_enabled() {
                            continue;
                        }
                        behaviour.rr_5 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    6 => {
                        if behaviour.rr_6.is_enabled() {
                            continue;
                        }
                        behaviour.rr_6 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    7 => {
                        if behaviour.rr_7.is_enabled() {
                            continue;
                        }
                        behaviour.rr_7 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    8 => {
                        if behaviour.rr_8.is_enabled() {
                            continue;
                        }
                        behaviour.rr_8 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    9 => {
                        if behaviour.rr_9.is_enabled() {
                            continue;
                        }
                        behaviour.rr_9 = protocols
                            .request_response
                            .then(|| request_response::Behaviour::new(config.clone()))
                            .into();
                    }
                    _ => {
                        tracing::warn!(
                            "local node can only support up to 10 request-response protocols at this time."
                        );
                        break;
                    }
                }

                existing_protocol.insert(protocol, index);
            }

            if !existing_protocol.is_empty() {
                behaviour.rr_man = Toggle::from(Some(rr_man::Behaviour::new(existing_protocol)))
            }
        }

        Ok((behaviour, transport))
    }

    #[cfg(feature = "request-response")]
    pub(crate) fn request_response(
        &mut self,
        protocol: Option<libp2p::StreamProtocol>,
    ) -> Option<&mut request_response::Behaviour> {
        let Some(protocol) = protocol else {
            return self.rr_0.as_mut();
        };

        let manager = self.rr_man.as_ref()?;
        let index = manager.get_protocol(protocol)?;
        match index {
            0 => self.rr_0.as_mut(),
            1 => self.rr_1.as_mut(),
            2 => self.rr_2.as_mut(),
            3 => self.rr_3.as_mut(),
            4 => self.rr_4.as_mut(),
            5 => self.rr_5.as_mut(),
            6 => self.rr_6.as_mut(),
            7 => self.rr_7.as_mut(),
            8 => self.rr_8.as_mut(),
            9 => self.rr_9.as_mut(),
            _ => None,
        }
    }
}
