use crate::context::IpfsContext;
#[cfg(not(target_arch = "wasm32"))]
use crate::p2p::RelayConfig;
use crate::p2p::{
    AddressBookConfig, IdentifyConfiguration, PubsubConfig, TSwarm, create_create_behaviour,
};
use crate::repo::{DefaultKeystore, DefaultStorage, GCConfig, GCTrigger, Repo};
use crate::{
    ConnectionLimits, FDLimit, Ipfs, IpfsEvent, IpfsOptions, Keypair, Multiaddr, NetworkBehaviour,
    RecordKey, RepoProvider, TSwarmEvent, TSwarmEventFn, context, ipns_to_dht_key, p2p, to_dht_key,
};
use anyhow::Error;
use async_rt::AbortableJoinHandle;
use connexa::behaviour::peer_store::store::memory::MemoryStore;
use connexa::behaviour::request_response::RequestResponseConfig;
use connexa::builder::{ConnexaBuilder, FileDescLimit, IntoKeypair};
use connexa::keystore::Keychain;
use connexa::prelude::identify::Event;
use connexa::prelude::swarm::SwarmEvent;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "pnet")]
use connexa::prelude::transport::pnet::PreSharedKey;
use connexa::prelude::{gossipsub, ping, swarm};
use connexa::{behaviour, dummy};
use futures::{StreamExt, TryStreamExt, stream::FuturesUnordered};
use ipld_core::cid::Cid;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::convert::Infallible;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tracing::Span;
use tracing_futures::Instrument;

/// Configured Ipfs which can only be started.
#[allow(clippy::type_complexity)]
pub struct IpfsBuilder<C: NetworkBehaviour<ToSwarm = Infallible> + Send + Sync + 'static> {
    init: ConnexaBuilder<p2p::Behaviour<C>, IpfsContext, IpfsEvent, MemoryStore, DefaultKeystore>,
    options: IpfsOptions,

    repo_handle: Repo<DefaultStorage>,
    swarm_event: Option<TSwarmEventFn<C>>,
    record_key_validator:
        HashMap<String, Box<dyn Fn(&str) -> anyhow::Result<RecordKey> + Sync + Send>>,
    gc_config: Option<GCConfig>,
    custom_behaviour: Option<Box<dyn FnOnce(&Keypair) -> std::io::Result<C>>>,
    gc_repo_duration: Option<Duration>,
}

pub type DefaultIpfsBuilder = IpfsBuilder<dummy::Behaviour>;

impl<C: NetworkBehaviour<ToSwarm = Infallible> + Send + Sync + 'static> Default for IpfsBuilder<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: NetworkBehaviour<ToSwarm = Infallible> + Send + Sync + 'static> IpfsBuilder<C> {
    /// New uninitualized instance
    pub fn new() -> Self {
        let keypair = Keypair::generate_ed25519();
        Self::with_keypair(&keypair).expect("keypair is valid")
    }

    /// New instance with an existing keypair
    pub fn with_keypair(keypair: impl IntoKeypair) -> std::io::Result<Self> {
        let builder = ConnexaBuilder::with_existing_identity(keypair)?;
        Ok(Self::from_identity(builder))
    }

    /// Create an instance that resolves its identity from `keychain` under `label`
    /// loading it, or generating and storing a new identity if none exists yet.
    pub fn with_keychain_identity(
        keychain: Keychain<DefaultKeystore>,
        label: impl Into<String>,
    ) -> Self {
        let builder = ConnexaBuilder::with_keychain_identity(keychain, label);
        Self::from_identity(builder)
    }

    /// Create an instance that loads its identity from `keychain` under `label`.
    pub fn with_existing_keychain_identity(
        keychain: Keychain<DefaultKeystore>,
        label: impl Into<String>,
    ) -> Self {
        let builder = ConnexaBuilder::with_existing_keychain_identity(keychain, label);
        Self::from_identity(builder)
    }

    fn from_identity(
        builder: ConnexaBuilder<
            p2p::Behaviour<C>,
            IpfsContext,
            IpfsEvent,
            MemoryStore,
            DefaultKeystore,
        >,
    ) -> Self {
        Self {
            init: builder,
            options: Default::default(),
            repo_handle: Repo::new_memory(),
            record_key_validator: Default::default(),
            swarm_event: None,
            gc_config: None,
            gc_repo_duration: None,
            custom_behaviour: None,
        }
    }

    /// Set default listening unspecified ipv4 and ipv6 addresses for tcp and quic
    /// Note that this still requires for the transports to be enabled to be usable
    pub fn set_default_listener(self) -> Self {
        self.add_listening_addrs(vec![
            "/ip4/0.0.0.0/tcp/0".parse().unwrap(),
            "/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap(),
        ])
    }

    /// Adds a listening address
    pub fn add_listening_addr(mut self, addr: Multiaddr) -> Self {
        if !self.options.listening_addrs.contains(&addr) {
            self.options.listening_addrs.push(addr)
        }
        self
    }

    /// Set a connection limit
    pub fn set_connection_limits<F>(mut self, f: F) -> Self
    where
        F: Fn(ConnectionLimits) -> ConnectionLimits + Send + Sync + 'static,
    {
        self.init = self.init.with_connection_limits_with_config(f);
        self
    }

    /// Adds a listening addresses
    pub fn add_listening_addrs(mut self, addrs: Vec<Multiaddr>) -> Self {
        self.options.listening_addrs.extend(addrs);
        self
    }

    /// Set a list of listening addresses
    pub fn set_listening_addrs(mut self, addrs: Vec<Multiaddr>) -> Self {
        self.options.listening_addrs = addrs;
        self
    }

    /// Adds a bootstrap node
    pub fn add_bootstrap(mut self, addr: Multiaddr) -> Self {
        if !self.options.bootstrap.contains(&addr) {
            self.options.bootstrap.push(addr)
        }
        self
    }

    /// Load default behaviour for basic functionality
    pub fn with_default(self) -> Self {
        self.with_identify(Default::default())
            .with_autonat()
            .with_bitswap()
            .with_kademlia()
            .with_ping(Default::default())
            .with_pubsub(Default::default())
    }

    /// Enables kademlia
    pub fn with_kademlia(mut self) -> Self {
        self.init = self.init.with_kademlia();
        self
    }

    /// Enables bitswap
    pub fn with_bitswap(mut self) -> Self {
        self.options.protocols.bitswap = true;
        self
    }

    /// Enables bitswap with explicit configuration
    pub fn with_bitswap_config<F>(mut self, f: F) -> Self
    where
        F: Fn(p2p::bitswap::Config) -> p2p::bitswap::Config + 'static,
    {
        self.options.protocols.bitswap = true;
        self.options.bitswap_config = Box::new(f);
        self
    }

    /// Enables trustless HTTP gateway retrieval.
    #[cfg(feature = "gateway")]
    pub fn enable_gateway_retrieval(
        mut self,
        gateways: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.options.gateway = Some(gateways.into_iter().map(Into::into).collect());
        self
    }

    /// Enables delegated routing V1 HTTP provider discovery.
    #[cfg(feature = "routing")]
    pub fn enable_delegated_routing(
        mut self,
        routers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.options.router = Some(routers.into_iter().map(Into::into).collect());
        self
    }

    /// Enable mdns
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_mdns(mut self) -> Self {
        self.init = self.init.with_mdns();
        self
    }

    /// Enable relay client
    pub fn with_relay(mut self, with_dcutr: bool) -> Self {
        self.options.protocols.relay = true;
        self.init = self.init.with_relay();
        if with_dcutr {
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.init = self.init.with_dcutr();
            }
        }
        self
    }

    /// Enable autorelay
    pub fn with_autorelay(mut self) -> Self {
        self.init = self.init.with_autorelay();
        self
    }

    /// Enable autorelay with configuration option
    pub fn with_autorelay_with_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(behaviour::autorelay::Config) -> behaviour::autorelay::Config + 'static,
    {
        self.init = self.init.with_autorelay_with_config(f);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Enable relay server
    pub fn with_relay_server(mut self, config: RelayConfig) -> Self {
        self.init = self
            .init
            .with_relay_server_with_config(move |_| config.into());
        self
    }

    /// Enable port mapping (AKA UPnP)
    #[cfg(all(not(target_arch = "wasm32"), feature = "upnp"))]
    pub fn with_upnp(mut self) -> Self {
        self.init = self.init.with_upnp();
        self
    }

    /// Enables rendezvous server
    pub fn with_rendezvous_server(mut self) -> Self {
        self.init = self.init.with_rendezvous_server();
        self
    }

    /// Enables rendezvous client
    pub fn with_rendezvous_client(mut self) -> Self {
        self.init = self.init.with_rendezvous_client();
        self
    }

    /// Enables identify
    pub fn with_identify(mut self, config: IdentifyConfiguration) -> Self {
        self.init = self
            .init
            .with_identify_with_config(config.protocol_version, move |cfg| {
                cfg.with_agent_version(config.agent_version)
                    .with_interval(config.interval)
                    .with_push_listen_addr_updates(config.push_update)
                    .with_cache_size(config.cache)
            });
        self
    }

    #[cfg(feature = "stream")]
    pub fn with_streams(mut self) -> Self {
        self.init = self.init.with_streams();
        self
    }

    /// Enables pubsub
    pub fn with_pubsub(mut self, config: PubsubConfig) -> Self {
        self.init = self
            .init
            .with_gossipsub_with_config(move |keypair, mut builder| {
                if let Some(protocol) = config.custom_protocol_id {
                    builder.protocol_id(protocol, gossipsub::Version::V1_1);
                }

                builder.max_transmit_size(config.max_transmit_size);

                if config.floodsub_compat {
                    builder.support_floodsub();
                }

                builder.validation_mode(config.validate.into());
                let auth =
                    connexa::prelude::gossipsub::MessageAuthenticity::Signed(keypair.clone());
                (builder, auth)
            });
        self
    }

    /// Enables request response.
    /// Note: At this time, this option will only support up to 10 request-response behaviours.
    ///       with any additional being ignored. Additionally, any duplicated protocols that are
    ///       provided will be ignored.
    pub fn with_request_response(mut self, config: Vec<RequestResponseConfig>) -> Self {
        self.init = self.init.with_request_response(config);

        self
    }

    /// Enables autonat
    pub fn with_autonat(mut self) -> Self {
        self.init = self.init.with_autonat_v1();
        self
    }

    /// Enables ping
    pub fn with_ping(mut self, config: ping::Config) -> Self {
        self.init = self.init.with_ping_with_config(move |_| config);
        self
    }

    /// Set a custom behaviour
    pub fn with_custom_behaviour<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&Keypair) -> std::io::Result<C> + 'static,
    {
        self.custom_behaviour.replace(Box::new(f));
        self
    }

    /// Enables automatic garbage collection
    pub fn with_gc(mut self, config: GCConfig) -> Self {
        self.gc_config = Some(config);
        self
    }

    /// Set a duration for which blocks are not removed due to the garbage collector
    /// Defaults: 2 mins
    pub fn set_temp_pin_duration(mut self, duration: Duration) -> Self {
        self.gc_repo_duration = Some(duration);
        self
    }

    /// Sets a path
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_path<P: AsRef<std::path::Path>>(mut self, path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        self.options.ipfs_path = Some(path);
        self
    }

    /// Sets a namespace
    #[cfg(target_arch = "wasm32")]
    pub fn set_namespace(mut self, ns: Option<String>) -> Self {
        self.options.namespace = Some(ns);
        self
    }

    /// Set timeout for idle connections
    pub fn set_idle_connection_timeout(mut self, duration: u64) -> Self {
        self.init = self.init.set_swarm_config(move |swarm| {
            swarm.with_idle_connection_timeout(Duration::from_secs(duration))
        });
        self
    }

    /// Set swarm configuration
    pub fn set_swarm_configuration<F>(mut self, f: F) -> Self
    where
        F: FnOnce(swarm::Config) -> swarm::Config + Send + Sync + 'static,
    {
        self.init = self.init.set_swarm_config(f);
        self
    }

    /// Set default record validator for IPFS
    /// Note: This will override any keys set for `ipns` prefix
    pub fn default_record_key_validator(mut self) -> Self {
        self.record_key_validator.insert(
            "ipns".into(),
            Box::new(|key| to_dht_key(("ipns", |key| ipns_to_dht_key(key)), key)),
        );
        self
    }

    pub fn set_record_prefix_validator<F>(mut self, key: &str, callback: F) -> Self
    where
        F: Fn(&str) -> anyhow::Result<RecordKey> + Sync + Send + 'static,
    {
        self.record_key_validator
            .insert(key.to_string(), Box::new(callback));
        self
    }

    /// Set address book configuration
    pub fn set_addrbook_configuration(mut self, config: AddressBookConfig) -> Self {
        self.options.addr_config = config;
        self
    }

    /// Set RepoProvider option to provide blocks automatically
    pub fn set_provider(mut self, opt: RepoProvider) -> Self {
        self.options.provider = opt;
        self
    }

    /// Set block and data repo
    pub fn set_repo(mut self, repo: &Repo<DefaultStorage>) -> Self {
        self.repo_handle = Repo::clone(repo);
        self
    }

    /// Enables quic transport
    #[cfg(feature = "quic")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_quic(mut self) -> Self {
        self.init = self.init.enable_quic();
        self
    }

    /// Enables quic transport with custom configuration
    #[cfg(feature = "quic")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_quic_with_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(
                connexa::prelude::transport::quic::Config,
            ) -> connexa::prelude::transport::quic::Config
            + 'static,
    {
        self.init = self.init.enable_quic_with_config(f);
        self
    }

    /// Enables tcp transport
    #[cfg(feature = "tcp")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_tcp(mut self) -> Self {
        self.init = self.init.enable_tcp();
        self
    }

    /// Enables tcp transport with custom configuration
    #[cfg(feature = "tcp")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_tcp_with_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(
                connexa::prelude::transport::tcp::Config,
            ) -> connexa::prelude::transport::tcp::Config
            + 'static,
    {
        self.init = self.init.enable_tcp_with_config(f);
        self
    }

    // /// Enables pnet transport
    #[cfg(feature = "pnet")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_pnet(mut self, psk: PreSharedKey) -> Self {
        self.init = self.init.enable_pnet(psk);
        self
    }

    /// Enables websocket transport
    #[cfg(feature = "websocket")]
    pub fn enable_websocket(mut self) -> Self {
        self.init = self.init.enable_websocket();
        self
    }

    /// Enables secure websocket transport
    #[cfg(feature = "websocket")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_secure_websocket(mut self) -> Self {
        self.init = self.init.enable_secure_websocket();
        self
    }

    /// Enables secure websocket transport
    #[cfg(feature = "websocket")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_secure_websocket_with_pem(mut self, keypair: String, certs: Vec<String>) -> Self {
        self.init = self.init.enable_secure_websocket_with_pem(keypair, certs);
        self
    }

    /// Enables secure websocket transport
    #[cfg(feature = "websocket")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_secure_websocket_with_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&Keypair) -> std::io::Result<(Vec<String>, String)> + 'static,
    {
        self.init = self.init.enable_secure_websocket_with_config(f);
        self
    }

    /// Enables DNS
    #[cfg(feature = "dns")]
    pub fn enable_dns(self) -> Self {
        self.enable_dns_with_resolver(connexa::prelude::transport::dns::DnsResolver::default())
    }

    /// Enables DNS with a specific resolver
    #[cfg(feature = "dns")]
    pub fn enable_dns_with_resolver(
        mut self,
        resolver: connexa::prelude::transport::dns::DnsResolver,
    ) -> Self {
        self.init = self.init.enable_dns_with_resolver(resolver);
        self
    }

    /// Enables WebRTC transport
    #[cfg(feature = "webrtc")]
    pub fn enable_webrtc(mut self) -> Self {
        self.init = self.init.enable_webrtc();
        self
    }

    /// Enables WebRTC transport, allowing one to generate a certificate using the provided keypair in the closure.
    #[cfg(feature = "webrtc")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_webrtc_with_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&Keypair) -> std::io::Result<String> + 'static,
    {
        self.init = self.init.enable_webrtc_with_config(f);
        self
    }

    /// Enable WebRTC transport with a provided pre-generated pem.
    #[cfg(feature = "webrtc")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_webrtc_with_pem(self, pem: impl Into<String>) -> Self {
        let pem = pem.into();
        self.enable_webrtc_with_config(move |_| Ok(pem))
    }

    /// Enables memory transport
    pub fn enable_memory_transport(mut self) -> Self {
        self.init = self.init.enable_memory_transport();
        self
    }

    /// Set file desc limit
    pub fn fd_limit(mut self, limit: FDLimit) -> Self {
        let limit = match limit {
            FDLimit::Max => FileDescLimit::Max,
            FDLimit::Custom(n) => FileDescLimit::Custom(n),
        };
        self.init = self.init.set_file_descriptor_limit(limit);
        self
    }

    /// Set tracing span
    pub fn set_span(mut self, span: Span) -> Self {
        self.options.span = Some(span);
        self
    }

    /// Handle libp2p swarm events
    pub fn swarm_events<F>(mut self, func: F) -> Self
    where
        F: Fn(&mut TSwarm<C>, &TSwarmEvent<C>) + Sync + Send + 'static,
    {
        self.swarm_event = Some(Arc::new(func));
        self
    }

    /// Initialize the ipfs node. The returned `Ipfs` value is cloneable, send and sync.
    pub async fn start(self) -> Result<Ipfs, Error> {
        let IpfsBuilder {
            mut options,
            record_key_validator,
            repo_handle,
            gc_config,
            init,
            custom_behaviour,
            swarm_event,
            ..
        } = self;

        let root_span = Option::take(&mut options.span)
            // not sure what would be the best practice with tracing and spans
            .unwrap_or_else(|| tracing::trace_span!(parent: &Span::current(), "ipfs"));

        // the "current" span which is not entered but the awaited futures are instrumented with it
        let init_span = tracing::trace_span!(parent: &root_span, "init");

        // stored in the Ipfs, instrumenting every method call
        let facade_span = tracing::trace_span!("facade");

        // stored in the executor given to libp2p, used to spawn at least the connections,
        // instrumenting each of those.
        // let exec_span = tracing::trace_span!(parent: &root_span, "exec");
        //
        // // instruments the IpfsFuture, the background task.
        // let swarm_span = tracing::trace_span!(parent: &root_span, "swarm");

        let mut repo = repo_handle;

        if repo.is_online() {
            anyhow::bail!("Repo is already initialized");
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            repo = match &options.ipfs_path {
                Some(path) => {
                    if !path.is_dir() {
                        tokio::fs::create_dir_all(path).await?;
                    }
                    Repo::<DefaultStorage>::new_fs(path)
                }
                None => repo,
            };
        }

        #[cfg(target_arch = "wasm32")]
        {
            repo = match options.namespace.take() {
                Some(ns) => Repo::<DefaultStorage>::new_idb(ns),
                None => repo,
            };
        }

        repo.init().instrument(init_span.clone()).await?;

        let repo_events = repo.initialize_channel();

        //Note: If `All` or `Pinned` are used, we would have to auto adjust the amount of
        //      provider records by adding the amount of blocks to the config.
        //TODO: Add persistent layer for kad store
        let blocks = match options.provider {
            RepoProvider::None => vec![],
            RepoProvider::All => repo.list_blocks().await.collect::<Vec<_>>().await,
            RepoProvider::Pinned => {
                repo.list_pins(None)
                    .await
                    .filter_map(|result| futures::future::ready(result.map(|(cid, _)| cid).ok()))
                    .collect()
                    .await
            }
            RepoProvider::Roots => {
                //TODO: Scan blockstore for root unixfs blocks
                warn!("RepoProvider::Roots is not implemented... ignoring...");
                vec![]
            }
        };

        // TODO: use to calculate store records limits when it is implemented in connexa
        let _count = blocks.len();

        let listening_addrs = options.listening_addrs.clone();

        let gc_handle = gc_config.map(|config| {
            async_rt::task::spawn_abortable({
                let repo = Repo::clone(&repo);
                async move {
                    let GCConfig { duration, trigger } = config;
                    let use_config_timer = duration != Duration::ZERO;
                    if trigger == GCTrigger::None && !use_config_timer {
                        tracing::warn!("GC does not have a set timer or a trigger. Disabling GC");
                        return;
                    }

                    let time = match use_config_timer {
                        true => duration,
                        false => Duration::from_secs(60 * 60),
                    };

                    let mut interval = futures_timer::Delay::new(time);

                    loop {
                        tokio::select! {
                            _ = &mut interval => {
                                let _g = repo.inner.gclock.write().await;
                                tracing::debug!("preparing gc operation");
                                let pinned = repo
                                    .list_pins(None)
                                    .await
                                    .try_filter_map(|(cid, _)| futures::future::ready(Ok(Some(cid))))
                                    .try_collect::<BTreeSet<_>>()
                                    .await
                                    .unwrap_or_default();
                                let pinned = Vec::from_iter(pinned);
                                let total_size = repo.get_total_size().await.unwrap_or_default();
                                let pinned_size = repo
                                    .get_blocks_size(&pinned)
                                    .await
                                    .ok()
                                    .flatten()
                                    .unwrap_or_default();

                                let unpinned_blocks = total_size.saturating_sub(pinned_size);

                                tracing::debug!(total_size = %total_size, ?trigger, unpinned_blocks);

                                let cleanup = match trigger {
                                    GCTrigger::At { size } => {
                                        total_size > 0 && unpinned_blocks >= size
                                    }
                                    GCTrigger::AtStorage => {
                                        unpinned_blocks > 0
                                            && unpinned_blocks >= repo.max_storage_size()
                                    }
                                    GCTrigger::None => unpinned_blocks > 0,
                                };

                                tracing::debug!(will_run = %cleanup);

                                if cleanup {
                                    tracing::debug!("running cleanup of unpinned blocks");
                                    match repo.cleanup().await {
                                        Ok(blocks) => {
                                            tracing::debug!(
                                                removed_blocks = blocks.len(),
                                                "blocks removed"
                                            );
                                            tracing::debug!("cleanup finished");
                                        }
                                        Err(e) => {
                                            tracing::error!(error = %e, "gc cleanup failed");
                                        }
                                    }
                                }

                                interval.reset(time);
                            }
                        }
                    }
                }
            })
        }).unwrap_or(AbortableJoinHandle::empty());

        let (discovery_tx, discovery_rx) = futures::channel::mpsc::channel::<Cid>(256);

        let mut context = context::IpfsContext::new(&repo);
        context.repo_events.replace(repo_events);
        context.discovery_tx.replace(discovery_tx);

        #[cfg(feature = "gateway")]
        let (gateways, gateway_guard) = match options.gateway.take() {
            Some(config) => {
                let gateways = crate::gateway::GatewayList::new(config);
                let (gateway_tx, gateway_rx) = futures::channel::mpsc::channel::<Cid>(256);
                context.gateway_tx.replace(gateway_tx);
                let guard = async_rt::task::spawn_abortable({
                    let repo = repo.clone();
                    let gateways = gateways.clone();
                    async move { crate::gateway::run(gateway_rx, repo, gateways).await }
                });
                (Some(gateways), guard)
            }
            None => (None, AbortableJoinHandle::empty()),
        };

        #[cfg(feature = "routing")]
        let routing_setup = options.router.take().map(|config| {
            let routers = crate::routing::RouterList::new(config);
            let (router_tx, router_rx) = futures::channel::mpsc::channel::<Cid>(256);
            context.router_tx.replace(router_tx);
            (routers, router_rx)
        });

        let connexa = init
            .with_custom_behaviour_with_context((options, repo.clone()), |keys, (options, repo)| {
                let custom_behaviour = match custom_behaviour {
                    Some(custom_behaviour) => Some(custom_behaviour(keys)?),
                    None => None,
                };
                Ok(create_create_behaviour(
                    keys,
                    &options,
                    &repo,
                    custom_behaviour,
                ))
            })
            .set_context(context)
            .set_custom_task_callback(|swarm, _, context, event| context.handle_event(swarm, event))
            .set_custom_event_callback(|_swarm, _, context, event| {
                if let crate::p2p::BehaviourEvent::Bitswap(
                    crate::p2p::bitswap::Event::NeedBlock { cid },
                ) = event
                {
                    context.enqueue_retrieval(cid);
                }
            })
            .set_swarm_event_callback(move |swarm, _, event, context| {
                if let Some(callback) = swarm_event.as_ref() {
                    callback(swarm, event);
                }
                if let SwarmEvent::Behaviour(connexa::behaviour::BehaviourEvent::Identify(event)) =
                    event
                {
                    match event {
                        Event::Received { info, .. } => {
                            let peer_id = info.public_key.to_peer_id();
                            if let Some(chs) = context.find_peer_identify.remove(&peer_id) {
                                for ch in chs {
                                    let _ = ch.send(Ok(info.clone()));
                                }
                            }
                        }
                        Event::Sent { .. } => {}
                        Event::Pushed { .. } => {}
                        Event::Error { .. } => {}
                    }
                }
            })
            .set_pollable_callback(|cx, swarm, _, context| {
                let custom = swarm
                    .behaviour_mut()
                    .custom
                    .as_mut()
                    .expect("behaviour enabled");
                while let Poll::Ready(Some(event)) = context.repo_events.poll_next_unpin(cx) {
                    context.handle_repo_event(custom, event);
                }
                context.flush_retrieval_queues(cx);
                Poll::Pending
            })
            .set_preload(|_, swarm, _, _| {
                for addr in listening_addrs {
                    if let Err(e) = swarm.listen_on(addr.clone()) {
                        tracing::error!(%addr, %e, "failed to listen on address");
                    }
                }

                for block in blocks {
                    if let Some(kad) = swarm.behaviour_mut().kademlia.as_mut() {
                        let key = RecordKey::from(block.hash().to_bytes());
                        if let Err(e) = kad.start_providing(key) {
                            match e {
                                connexa::prelude::dht::store::Error::MaxProvidedKeys => break,
                                _ => unreachable!(),
                            }
                        }
                    }
                }
            })
            .build()
            .await?;

        let discovery_guard = async_rt::task::spawn_abortable({
            let connexa = connexa.clone();
            let repo = repo.clone();
            async move {
                let mut rx = discovery_rx;
                let mut inflight: HashSet<Cid> = HashSet::new();
                let mut lookups = FuturesUnordered::new();
                loop {
                    tokio::select! {
                        maybe_cid = rx.next(), if lookups.len() < 8 => {
                            let Some(cid) = maybe_cid else { break };
                            if !repo.is_block_wanted(&cid) {
                                continue;
                            }
                            if !inflight.insert(cid) {
                                continue;
                            }
                            let connexa = connexa.clone();
                            lookups.push(async move {
                                if let Ok(mut providers) = connexa.dht().get_providers(cid).await {
                                    let mut deadline =
                                        futures_timer::Delay::new(Duration::from_secs(10));
                                    let mut dialed = 0usize;
                                    loop {
                                        tokio::select! {
                                            item = providers.next() => {
                                                let Some(Ok(set)) = item else { break };
                                                for peer in set {
                                                    let opts = crate::DialOpts::peer_id(peer).build();
                                                    let _ = connexa.swarm().dial(opts).await;
                                                    dialed += 1;
                                                    if dialed >= 8 {
                                                        break;
                                                    }
                                                }
                                                if dialed >= 8 {
                                                    break;
                                                }
                                            }
                                            _ = &mut deadline => break,
                                        }
                                    }
                                }
                                cid
                            });
                        }
                        Some(cid) = lookups.next(), if !lookups.is_empty() => {
                            inflight.remove(&cid);
                        }
                    }
                }
            }
        });

        #[cfg(feature = "routing")]
        let (routers, routing_guard) = match routing_setup {
            Some((routers, router_rx)) => {
                let guard = async_rt::task::spawn_abortable({
                    let connexa = connexa.clone();
                    let routers = routers.clone();
                    let repo = repo.clone();
                    async move { crate::routing::run(router_rx, connexa, routers, repo).await }
                });
                (Some(routers), guard)
            }
            None => (None, AbortableJoinHandle::empty()),
        };

        let ipfs = Ipfs {
            span: facade_span,
            repo,
            connexa,
            record_key_validator: Arc::new(record_key_validator),
            _gc_guard: gc_handle,
            _discovery_guard: discovery_guard,
            #[cfg(feature = "gateway")]
            gateways,
            #[cfg(feature = "gateway")]
            _gateway_guard: gateway_guard,
            #[cfg(feature = "routing")]
            routers,
            #[cfg(feature = "routing")]
            _routing_guard: routing_guard,
        };

        Ok(ipfs)
    }
}
