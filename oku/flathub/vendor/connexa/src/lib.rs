pub mod behaviour;
pub mod builder;
pub mod error;
pub mod handle;
pub mod keystore;
mod multiaddr_ext;
pub mod task;
pub(crate) mod types;

use crate::behaviour::BehaviourEvent;
use crate::keystore::Keychain;
use libp2p::identity::Keypair;
use libp2p::swarm::NetworkBehaviour;
use libp2p::swarm::{Swarm, SwarmEvent};
use std::task::{Context, Poll};

pub(crate) type TTaskCallback<B, Ctx, Cmd, Store, K> = Box<
    dyn Fn(&mut Swarm<behaviour::Behaviour<B, Store>>, &Keychain<K>, &mut Ctx, Cmd)
        + 'static
        + Send,
>;
pub(crate) type TEventCallback<B, Ctx, Store, K> = Box<
    dyn Fn(
            &mut Swarm<behaviour::Behaviour<B, Store>>,
            &Keychain<K>,
            &mut Ctx,
            <B as NetworkBehaviour>::ToSwarm,
        )
        + 'static
        + Send,
>;
pub(crate) type TPollableCallback<B, Ctx, Store, K> = Box<
    dyn Fn(
            &mut Context,
            &mut Swarm<behaviour::Behaviour<B, Store>>,
            &Keychain<K>,
            &mut Ctx,
        ) -> Poll<()>
        + 'static
        + Send,
>;
pub(crate) type TSwarmEventCallback<B, Ctx, Store, K> = Box<
    dyn Fn(
            &mut Swarm<behaviour::Behaviour<B, Store>>,
            &Keychain<K>,
            &SwarmEvent<BehaviourEvent<B, Store>>,
            &mut Ctx,
        )
        + 'static
        + Send,
>;

pub(crate) type TPreloadCallback<B, Ctx, Store, K> = Box<
    dyn FnOnce(&Keypair, &mut Swarm<behaviour::Behaviour<B, Store>>, &Keychain<K>, &mut Ctx)
        + 'static
        + Send,
>;

pub mod dummy {
    pub use crate::behaviour::dummy::{Behaviour, DummyHandler};
}

pub mod prelude {
    use crate::builder::ConnexaBuilder;
    pub use crate::multiaddr_ext::MultiaddrExt;
    pub use crate::types::*;
    pub use libp2p::{Multiaddr, PeerId, Stream, StreamProtocol, multiaddr::Protocol};

    pub use crate::handle::swarm::ConnectionTarget;

    use crate::prelude::peer_store::store::memory::MemoryStore;
    pub use libp2p::identity;

    pub mod swarm {
        pub use libp2p::SwarmBuilder;
        pub use libp2p::swarm::*;
    }

    pub mod peer_store {
        pub use crate::behaviour::peer_store::store;
    }

    #[cfg(feature = "kad")]
    pub mod dht {
        pub use crate::handle::dht::{ToOptionalRecordKey, ToRecordKey};
        pub use libp2p::kad::*;
    }

    #[cfg(feature = "request-response")]
    pub mod request_response {
        pub use crate::handle::request_response::{IntoRequest, OptionalStreamProtocol};
        pub use libp2p::request_response::{
            Config, Event, InboundFailure, InboundRequestId, Message, OutboundFailure,
            OutboundRequestId, ProtocolSupport,
        };
    }

    #[cfg(feature = "stream")]
    pub mod stream {
        pub use crate::handle::stream::IntoStreamProtocol;
        pub use libp2p_stream::{Control, IncomingStreams, OpenStreamError};
    }

    #[cfg(feature = "relay")]
    pub mod relay {
        #[cfg(not(target_arch = "wasm32"))]
        pub mod server {
            // TODO: Determine if CircuitId is needed
            pub use libp2p::relay::{Config, Event, RateLimiter, Status, StatusCode};
        }

        pub mod client {
            pub use libp2p::relay::client::Event;
        }
    }

    #[cfg(feature = "dcutr")]
    pub mod dcutr {
        pub use libp2p::dcutr::{Error, Event};
    }

    #[cfg(feature = "ping")]
    pub mod ping {
        pub use libp2p::ping::{Config, Event, Failure};
    }

    #[cfg(feature = "identify")]
    pub mod identify {
        pub use libp2p::identify::{Config, Event, Info, UpgradeError};
    }

    #[cfg(feature = "gossipsub")]
    pub mod gossipsub {
        pub use crate::handle::gossipsub::IntoTopic as IntoGossipsubTopic;
        pub use libp2p::gossipsub::{
            AllowAllSubscriptionFilter, Config, ConfigBuilder, Event, IdentTopic, Message,
            MessageAcceptance, MessageAuthenticity, MessageId, Sha256Topic, Topic, TopicHash,
            ValidationMode, Version,
        };
    }

    #[cfg(feature = "floodsub")]
    pub mod floodsub {
        pub use crate::handle::floodsub::IntoTopic as IntoFloodsubTopic;
        pub use libp2p::floodsub::{Config, Event, Topic};
    }

    #[cfg(feature = "rendezvous")]
    pub mod rendezvous {
        pub use crate::handle::rendezvous::IntoNamespace;
        pub use libp2p::rendezvous::{
            Cookie, ErrorCode, MAX_NAMESPACE, MAX_TTL, MIN_TTL, Namespace, Registration,
        };
    }

    #[cfg(feature = "mdns")]
    #[cfg(not(target_arch = "wasm32"))]
    pub mod mdns {
        pub use libp2p::mdns::{Config, Event};
    }

    #[cfg(feature = "autonat")]
    pub mod autonat {
        pub mod v1 {
            pub use libp2p::autonat::v1::{
                Config, Event, InboundFailure, InboundProbeError, InboundProbeEvent,
            };
        }

        pub mod v2 {
            pub mod server {
                pub use libp2p::autonat::v2::server::Event;
            }

            pub mod client {
                pub use libp2p::autonat::v2::client::{Config, Event};
            }
        }
    }

    #[cfg(feature = "upnp")]
    #[cfg(not(target_arch = "wasm32"))]
    pub mod upnp {
        pub use libp2p::upnp::Event;
    }

    pub mod connection_limits {
        pub use libp2p_connection_limits::{ConnectionLimits, Exceeded};
    }

    pub mod transport {
        #[cfg(feature = "dns")]
        pub mod dns {
            pub use crate::builder::transport::DnsResolver;
        }
        pub use libp2p::core::muxing;
        pub use libp2p::core::transport;
        pub use libp2p::core::upgrade;
        pub use libp2p::core::{ConnectedPoint, Endpoint};
        #[cfg(feature = "noise")]
        pub use libp2p::noise;
        #[cfg(feature = "pnet")]
        pub use libp2p::pnet;
        #[cfg(feature = "quic")]
        #[cfg(not(target_arch = "wasm32"))]
        pub use libp2p::quic;
        #[cfg(feature = "tcp")]
        #[cfg(not(target_arch = "wasm32"))]
        pub use libp2p::tcp;
        #[cfg(feature = "websocket")]
        #[cfg(not(target_arch = "wasm32"))]
        pub use libp2p::websocket;
        #[cfg(feature = "yamux")]
        pub use libp2p::yamux;
    }

    pub type DefaultConnexaBuilder = ConnexaBuilder<
        super::dummy::Behaviour,
        (),
        (),
        MemoryStore,
        crate::keystore::store::memory::MemoryKeystore,
    >;
}
