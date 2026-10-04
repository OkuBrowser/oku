#[allow(unused_imports)]
use either::Either;
#[allow(unused_imports)]
use futures::future::Either as FutureEither;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::Boxed;
use libp2p::core::transport::dummy::{DummyStream, DummyTransport};
#[allow(unused_imports)]
use libp2p::core::transport::timeout::TransportTimeout;
use libp2p::core::transport::upgrade::Version;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "dns")]
use libp2p::dns::{ResolverConfig, ResolverOpts};
use std::fmt::{Debug, Formatter};

use libp2p::PeerId;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "pnet")]
use libp2p::pnet::{PnetConfig, PreSharedKey};
#[cfg(feature = "relay")]
use libp2p::relay::client::Transport as ClientTransport;
#[cfg(not(feature = "relay"))]
type ClientTransport = ();
use crate::error::ConnexaResult;

#[cfg(all(feature = "dns", target_arch = "wasm32"))]
use crate::builder::dns_websys;
use libp2p::identity::Keypair;
use std::io;
use std::time::Duration;
#[allow(unused_imports)]
use {
    libp2p::Transport,
    libp2p::core::transport::{MemoryTransport, OrTransport},
};

/// Transport type.
pub(crate) type TTransport = Boxed<(PeerId, StreamMuxerBox)>;

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "yamux",
    not(any(feature = "noise", feature = "tls"))
))]
compile_error!(
    "the `yamux` feature requires at least one of `noise` or `tls` to be enabled on native targets"
);

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "websocket",
    not(feature = "tcp")
))]
compile_error!("the `websocket` feature requires the `tcp` feature on native targets");

pub struct TransportConfig {
    #[cfg(feature = "tcp")]
    #[cfg(not(target_arch = "wasm32"))]
    pub enable_tcp: bool,
    #[cfg(feature = "tcp")]
    #[cfg(not(target_arch = "wasm32"))]
    pub tcp_config_callback: Box<dyn FnOnce(libp2p::tcp::Config) -> libp2p::tcp::Config>,
    pub timeout: Duration,
    #[cfg(feature = "dns")]
    pub dns_resolver: Option<DnsResolver>,
    pub version: UpgradeVersion,
    #[cfg(feature = "quic")]
    #[cfg(not(target_arch = "wasm32"))]
    pub enable_quic: bool,
    #[cfg(feature = "quic")]
    #[cfg(not(target_arch = "wasm32"))]
    pub quic_config_callback: Box<dyn FnOnce(libp2p::quic::Config) -> libp2p::quic::Config>,
    #[cfg(feature = "websocket")]
    pub enable_websocket: bool,
    #[cfg(feature = "dns")]
    pub enable_dns: bool,
    pub enable_memory_transport: bool,
    #[cfg(feature = "webtransport")]
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub enable_webtransport: bool,
    #[cfg(feature = "websocket")]
    #[cfg(not(target_arch = "wasm32"))]
    pub websocket_pem: Option<(Vec<String>, String)>,
    #[cfg(feature = "websocket")]
    pub enable_secure_websocket: bool,
    #[cfg(feature = "webrtc")]
    pub enable_webrtc: bool,
    #[cfg(feature = "webrtc")]
    #[cfg(not(target_arch = "wasm32"))]
    pub webrtc_pem: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "pnet")]
    pub enable_pnet: bool,
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "pnet")]
    pub pnet_psk: Option<PreSharedKey>,
    #[cfg(all(feature = "dns", target_arch = "wasm32"))]
    pub dns_config_fn: Box<dyn FnOnce(dns_websys::Config) -> dns_websys::Config>,
}

impl Debug for TransportConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransportConfig").finish()
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "tcp")]
            #[cfg(not(target_arch = "wasm32"))]
            enable_tcp: false,
            #[cfg(feature = "tcp")]
            #[cfg(not(target_arch = "wasm32"))]
            tcp_config_callback: Box::new(|config| config),
            #[cfg(feature = "quic")]
            #[cfg(not(target_arch = "wasm32"))]
            enable_quic: false,
            #[cfg(feature = "websocket")]
            enable_websocket: false,
            #[cfg(feature = "websocket")]
            #[cfg(not(target_arch = "wasm32"))]
            websocket_pem: None,
            #[cfg(feature = "websocket")]
            enable_secure_websocket: false,
            enable_memory_transport: false,
            #[cfg(feature = "quic")]
            #[cfg(not(target_arch = "wasm32"))]
            quic_config_callback: Box::new(|config| config),
            #[cfg(feature = "dns")]
            enable_dns: false,
            #[cfg(feature = "webtransport")]
            enable_webtransport: false,
            #[cfg(feature = "webrtc")]
            enable_webrtc: false,
            #[cfg(feature = "webrtc")]
            #[cfg(not(target_arch = "wasm32"))]
            webrtc_pem: None,
            timeout: Duration::from_secs(10),
            #[cfg(feature = "dns")]
            dns_resolver: None,
            version: UpgradeVersion::default(),
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(feature = "pnet")]
            enable_pnet: false,
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(feature = "pnet")]
            pnet_psk: None,
            #[cfg(target_arch = "wasm32")]
            #[cfg(feature = "dns")]
            dns_config_fn: Box::new(|config| config),
        }
    }
}

#[cfg(feature = "dns")]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DnsResolver {
    /// Google DNS Resolver
    Google,
    /// Cloudflare DNS Resolver
    #[default]
    Cloudflare,
    /// Quad9 DNS Resolver
    Quad9,
    /// Local DNS Resolver
    Local,
    /// No DNS Resolver
    None,
}

#[cfg(feature = "dns")]
#[cfg(not(target_arch = "wasm32"))]
impl From<DnsResolver> for (ResolverConfig, ResolverOpts) {
    fn from(value: DnsResolver) -> Self {
        match value {
            DnsResolver::Google => (
                ResolverConfig::udp_and_tcp(&hickory_resolver::config::GOOGLE),
                Default::default(),
            ),
            DnsResolver::Cloudflare => (
                ResolverConfig::udp_and_tcp(&hickory_resolver::config::CLOUDFLARE),
                Default::default(),
            ),
            DnsResolver::Quad9 => (
                ResolverConfig::udp_and_tcp(&hickory_resolver::config::QUAD9),
                Default::default(),
            ),
            DnsResolver::Local => {
                hickory_resolver::system_conf::read_system_conf().unwrap_or_default()
            }
            DnsResolver::None => (ResolverConfig::default(), Default::default()),
        }
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UpgradeVersion {
    /// See [`Version::V1`]
    Standard,

    /// See [`Version::V1Lazy`]
    #[default]
    Lazy,
}

impl From<UpgradeVersion> for Version {
    fn from(value: UpgradeVersion) -> Self {
        match value {
            UpgradeVersion::Standard => Version::V1,
            UpgradeVersion::Lazy => Version::V1Lazy,
        }
    }
}

pub(crate) fn build_other_transport<F, M, T, R>(
    keypair: &Keypair,
    other: F,
) -> io::Result<TTransport>
where
    M: libp2p::core::muxing::StreamMuxer + Send + 'static,
    M::Substream: Send,
    M::Error: Send + Sync,
    T: Transport<Output = (PeerId, M)> + Send + Unpin + 'static,
    T::Error: Send + Sync + 'static,
    T::Dial: Send,
    T::ListenerUpgrade: Send,
    R: TryIntoTransport<T>,
    F: FnOnce(&Keypair) -> R,
{
    let transport = other(keypair)
        .try_into_transport()?
        .map(|(peer_id, conn), _| (peer_id, StreamMuxerBox::new(conn)))
        .boxed();

    Ok(transport)
}

/// Builds the transport that serves as a common ground for all connections.
#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_variables)]
pub(crate) fn build_transport(
    keypair: &Keypair,
    relay: Option<ClientTransport>,
    TransportConfig {
        #[cfg(feature = "tcp")]
        enable_tcp,
        #[cfg(feature = "tcp")]
        tcp_config_callback,
        timeout,
        #[cfg(feature = "dns")]
        dns_resolver,
        version,
        #[cfg(feature = "quic")]
        enable_quic,
        enable_memory_transport,
        #[cfg(feature = "dns")]
        enable_dns,
        #[cfg(feature = "websocket")]
        enable_websocket,
        #[cfg(feature = "websocket")]
        enable_secure_websocket,
        #[cfg(feature = "webrtc")]
        enable_webrtc,
        #[cfg(feature = "webrtc")]
        webrtc_pem,
        #[cfg(feature = "websocket")]
        websocket_pem,
        #[cfg(feature = "webtransport")]
            enable_webtransport: _,
        #[cfg(feature = "pnet")]
        enable_pnet,
        #[cfg(feature = "pnet")]
        pnet_psk,
        #[cfg(feature = "quic")]
        quic_config_callback,
    }: TransportConfig,
) -> ConnexaResult<TTransport> {
    #[cfg(all(feature = "noise", feature = "tls"))]
    use dual_transport::SelectSecurityUpgrade;
    #[cfg(feature = "dns")]
    use libp2p::dns::tokio::Transport as TokioDnsConfig;
    #[cfg(feature = "noise")]
    use libp2p::noise;
    #[cfg(feature = "quic")]
    use libp2p::quic::{Config as QuicConfig, tokio::Transport as TokioQuicTransport};
    #[cfg(feature = "tcp")]
    use libp2p::tcp::{Config as GenTcpConfig, tokio::Transport as TokioTcpTransport};
    #[cfg(feature = "tls")]
    use libp2p::tls;

    // Note that this is a temporary fix in the event of webrtc being enabled with tls or quic
    // which may cause issues with the default crypto provider and require us to natively install
    // the ring provider.
    #[cfg(all(feature = "webrtc", any(feature = "tls", feature = "quic")))]
    if enable_webrtc && rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    let transport = match enable_memory_transport {
        true => Either::Left(MemoryTransport::new()),
        false => Either::Right(DummyTransport::<DummyStream>::new()),
    };

    #[cfg(feature = "relay")]
    let transport = match relay {
        Some(relay) => Either::Left(OrTransport::new(relay, transport)),
        None => Either::Right(transport),
    };

    #[cfg(all(feature = "yamux", any(feature = "noise", feature = "tls")))]
    let transport = {
        let config = {
            #[cfg(all(feature = "noise", feature = "tls"))]
            {
                let noise_config = noise::Config::new(keypair).map_err(io::Error::other)?;
                let tls_config = tls::Config::new(keypair).map_err(io::Error::other)?;

                //TODO: Make configurable
                let config: SelectSecurityUpgrade<noise::Config, tls::Config> =
                    SelectSecurityUpgrade::new(noise_config, tls_config);
                config
            }
            #[cfg(all(feature = "noise", not(feature = "tls")))]
            {
                noise::Config::new(keypair).map_err(io::Error::other)?
            }
            #[cfg(all(not(feature = "noise"), feature = "tls"))]
            {
                tls::Config::new(keypair).map_err(io::Error::other)?
            }
        };

        let yamux_config = libp2p::yamux::Config::default();

        #[cfg(feature = "tcp")]
        let (tcp_config, transport) = {
            let config = tcp_config_callback(GenTcpConfig::default());
            if enable_tcp {
                let config_0 = config.clone();
                let tcp_transport = TokioTcpTransport::new(config);
                (
                    config_0,
                    Either::Left(tcp_transport.or_transport(transport)),
                )
            } else {
                (config, Either::Right(transport))
            }
        };

        #[cfg(all(feature = "websocket", feature = "tcp"))]
        let transport = match enable_websocket {
            true => {
                let mut ws_transport =
                    libp2p::websocket::Config::new(TokioTcpTransport::new(tcp_config));
                if enable_secure_websocket {
                    let (certs, priv_key) = match websocket_pem {
                        Some((cert, kp)) => {
                            let mut certs = Vec::with_capacity(cert.len());
                            let kp = rcgen::KeyPair::from_pem(&kp).map_err(io::Error::other)?;
                            let priv_key =
                                libp2p::websocket::tls::PrivateKey::new(kp.serialize_der());
                            for cert in cert.iter().map(|c| c.as_bytes()) {
                                let pem = pem::parse(cert)
                                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                                let cert =
                                    libp2p::websocket::tls::Certificate::new(pem.into_contents());
                                certs.push(cert);
                            }

                            (certs, priv_key)
                        }
                        None => {
                            use rcgen::CertifiedKey;
                            let CertifiedKey {
                                cert,
                                signing_key: key_pair,
                            } = rcgen::generate_simple_self_signed(["localhost".into()])
                                .map_err(io::Error::other)?;

                            let priv_key =
                                libp2p::websocket::tls::PrivateKey::new(key_pair.serialize_der());
                            let self_cert =
                                libp2p::websocket::tls::Certificate::new(cert.der().to_vec());

                            (vec![self_cert], priv_key)
                        }
                    };

                    let tls_config = libp2p::websocket::tls::Config::new(priv_key, certs)
                        .map_err(io::Error::other)?;
                    ws_transport.set_tls_config(tls_config);
                }
                let transport = ws_transport.or_transport(transport);
                Either::Left(transport)
            }
            false => Either::Right(transport),
        };

        #[cfg(feature = "pnet")]
        let transport = match (enable_pnet, pnet_psk) {
            (true, Some(psk)) => Either::Left(
                transport.and_then(move |socket, _| PnetConfig::new(psk).handshake(socket)),
            ),
            _ => Either::Right(transport),
        };

        transport
            .upgrade(version.into())
            .authenticate(config)
            .multiplex(yamux_config)
            .timeout(timeout)
            .boxed()
    };

    #[cfg(not(feature = "yamux"))]
    let transport = DummyTransport::<(PeerId, StreamMuxerBox)>::new().boxed();

    #[cfg(feature = "webrtc")]
    fn generate_webrtc_transport(
        keypair: &Keypair,
        pem: &Option<String>,
    ) -> io::Result<libp2p_webrtc::tokio::Transport> {
        let cert = match pem {
            Some(pem) => {
                libp2p_webrtc::tokio::Certificate::from_pem(pem).map_err(io::Error::other)?
            }
            None => {
                let mut rng = rand::rng();
                libp2p_webrtc::tokio::Certificate::generate(&mut rng).map_err(io::Error::other)?
            }
        };

        let kp = keypair.clone();
        let wrtc_tp = libp2p_webrtc::tokio::Transport::new(kp, cert);
        Ok(wrtc_tp)
    }

    #[cfg(feature = "webrtc")]
    let transport = match enable_webrtc {
        true => {
            let wrtc_tp = generate_webrtc_transport(keypair, &webrtc_pem)?;

            wrtc_tp
                .or_transport(transport)
                .map(|either_output, _| match either_output {
                    FutureEither::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                    FutureEither::Right((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                })
                .boxed()
        }
        false => transport.boxed(),
    };

    #[cfg(feature = "quic")]
    let transport = match enable_quic {
        true => {
            let quic_config = quic_config_callback(QuicConfig::new(keypair));
            let quic_transport = TokioQuicTransport::new(quic_config);

            OrTransport::new(quic_transport, transport)
                .map(|either_output, _| match either_output {
                    FutureEither::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                    FutureEither::Right((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                })
                .boxed()
        }
        false => transport,
    };

    #[cfg(feature = "dns")]
    let transport = match enable_dns {
        true => {
            let (cfg, opts) = dns_resolver.unwrap_or_default().into();
            TokioDnsConfig::custom(transport, cfg, opts).boxed()
        }
        false => transport,
    };

    Ok(transport)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn build_transport(
    keypair: &Keypair,
    relay: Option<ClientTransport>,
    TransportConfig {
        timeout,
        version,
        #[cfg(feature = "websocket")]
        enable_websocket,
        #[cfg(feature = "websocket")]
        enable_secure_websocket,
        #[cfg(feature = "webrtc")]
        enable_webrtc,
        #[cfg(feature = "webtransport")]
        enable_webtransport,
        enable_memory_transport,
        #[cfg(feature = "dns")]
        enable_dns,
        #[cfg(feature = "dns")]
        dns_resolver,
        #[cfg(feature = "dns")]
        dns_config_fn,
        ..
    }: TransportConfig,
) -> ConnexaResult<TTransport> {
    #[cfg(feature = "websocket")]
    use libp2p::websocket_websys;
    #[cfg(feature = "webtransport")]
    use libp2p::webtransport_websys;

    #[cfg(feature = "webrtc")]
    use libp2p_webrtc_websys as webrtc_websys;

    let transport = match enable_memory_transport {
        true => Either::Left(MemoryTransport::new()),
        false => Either::Right(DummyTransport::<DummyStream>::new()),
    };

    #[cfg(feature = "relay")]
    let transport = match relay {
        Some(relay) => Either::Left(OrTransport::new(relay, transport)),
        None => Either::Right(transport),
    };

    #[cfg(not(feature = "relay"))]
    let _ = relay;

    let noise_config = libp2p::noise::Config::new(keypair).map_err(io::Error::other)?;
    let yamux_config = libp2p::yamux::Config::default();

    #[cfg(feature = "websocket")]
    let transport = match enable_websocket | enable_secure_websocket {
        true => {
            let ws_transport = websocket_websys::Transport::default();
            let transport = ws_transport.or_transport(transport);
            Either::Left(transport)
        }
        false => Either::Right(transport),
    };

    let transport = transport
        .upgrade(version.into())
        .authenticate(noise_config)
        .multiplex(yamux_config)
        .timeout(timeout)
        .boxed();

    #[cfg(feature = "webtransport")]
    let transport = match enable_webtransport {
        true => {
            let config = webtransport_websys::Config::new(&keypair);
            let wtransport = webtransport_websys::Transport::new(config);
            wtransport
                .or_transport(transport)
                .map(|either_output, _| match either_output {
                    FutureEither::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                    FutureEither::Right((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                })
                .boxed()
        }
        false => transport.boxed(),
    };

    #[cfg(feature = "webrtc")]
    let transport = match enable_webrtc {
        true => {
            let wrtc_transport =
                webrtc_websys::Transport::new(webrtc_websys::Config::new(&keypair));
            wrtc_transport
                .or_transport(transport)
                .map(|either_output, _| match either_output {
                    FutureEither::Left((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                    FutureEither::Right((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                })
                .boxed()
        }
        false => transport,
    };

    #[cfg(feature = "dns")]
    let transport = match enable_dns {
        true => {
            let resolver = dns_resolver.unwrap_or_default();

            let config = match resolver {
                DnsResolver::Cloudflare => dns_websys::Config::cloudflare(),
                DnsResolver::Google | _ => dns_websys::Config::google(),
            };

            let config = dns_config_fn(config);

            let transport = dns_websys::Transport::with_config(transport, config);
            transport.boxed()
        }
        false => transport,
    };

    Ok(transport.boxed())
}

pub trait TryIntoTransport<T> {
    fn try_into_transport(self) -> io::Result<T>;
}

impl<T: Transport> TryIntoTransport<T> for T {
    fn try_into_transport(self) -> io::Result<T> {
        Ok(self)
    }
}

impl<T: Transport> TryIntoTransport<T> for Result<T, Box<dyn std::error::Error + Send + Sync>> {
    fn try_into_transport(self) -> io::Result<T> {
        self.map_err(io::Error::other)
    }
}

// borrow from libp2p SwarmBuilder
#[cfg(not(target_arch = "wasm32"))]
#[cfg(all(feature = "noise", feature = "tls"))]
mod dual_transport {
    use either::Either;
    use futures::{
        TryFutureExt,
        future::{self, MapOk},
    };
    use libp2p::{
        PeerId,
        core::{
            UpgradeInfo,
            either::EitherFuture,
            upgrade::{InboundConnectionUpgrade, OutboundConnectionUpgrade},
        },
    };
    use std::iter::{Chain, Map};

    #[derive(Debug, Clone)]
    pub struct SelectSecurityUpgrade<A, B>(A, B);

    impl<A, B> SelectSecurityUpgrade<A, B> {
        /// Combines two upgrades into an `SelectUpgrade`.
        ///
        /// The protocols supported by the first element have a higher priority.
        pub fn new(a: A, b: B) -> Self {
            SelectSecurityUpgrade(a, b)
        }
    }

    impl<A, B> UpgradeInfo for SelectSecurityUpgrade<A, B>
    where
        A: UpgradeInfo,
        B: UpgradeInfo,
    {
        type Info = Either<A::Info, B::Info>;
        type InfoIter = Chain<
            Map<<A::InfoIter as IntoIterator>::IntoIter, fn(A::Info) -> Self::Info>,
            Map<<B::InfoIter as IntoIterator>::IntoIter, fn(B::Info) -> Self::Info>,
        >;

        fn protocol_info(&self) -> Self::InfoIter {
            let a = self
                .0
                .protocol_info()
                .into_iter()
                .map(Either::Left as fn(A::Info) -> _);
            let b = self
                .1
                .protocol_info()
                .into_iter()
                .map(Either::Right as fn(B::Info) -> _);

            a.chain(b)
        }
    }

    impl<C, A, B, TA, TB, EA, EB> InboundConnectionUpgrade<C> for SelectSecurityUpgrade<A, B>
    where
        A: InboundConnectionUpgrade<C, Output = (PeerId, TA), Error = EA>,
        B: InboundConnectionUpgrade<C, Output = (PeerId, TB), Error = EB>,
    {
        type Output = (PeerId, future::Either<TA, TB>);
        type Error = Either<EA, EB>;
        type Future = MapOk<
            EitherFuture<A::Future, B::Future>,
            fn(future::Either<(PeerId, TA), (PeerId, TB)>) -> (PeerId, future::Either<TA, TB>),
        >;

        fn upgrade_inbound(self, sock: C, info: Self::Info) -> Self::Future {
            match info {
                Either::Left(info) => EitherFuture::First(self.0.upgrade_inbound(sock, info)),
                Either::Right(info) => EitherFuture::Second(self.1.upgrade_inbound(sock, info)),
            }
            .map_ok(future::Either::factor_first)
        }
    }

    impl<C, A, B, TA, TB, EA, EB> OutboundConnectionUpgrade<C> for SelectSecurityUpgrade<A, B>
    where
        A: OutboundConnectionUpgrade<C, Output = (PeerId, TA), Error = EA>,
        B: OutboundConnectionUpgrade<C, Output = (PeerId, TB), Error = EB>,
    {
        type Output = (PeerId, future::Either<TA, TB>);
        type Error = Either<EA, EB>;
        type Future = MapOk<
            EitherFuture<A::Future, B::Future>,
            fn(future::Either<(PeerId, TA), (PeerId, TB)>) -> (PeerId, future::Either<TA, TB>),
        >;

        fn upgrade_outbound(self, sock: C, info: Self::Info) -> Self::Future {
            match info {
                Either::Left(info) => EitherFuture::First(self.0.upgrade_outbound(sock, info)),
                Either::Right(info) => EitherFuture::Second(self.1.upgrade_outbound(sock, info)),
            }
            .map_ok(future::Either::factor_first)
        }
    }
}
