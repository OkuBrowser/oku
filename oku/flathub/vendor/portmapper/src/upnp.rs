use std::{
    net::{Ipv4Addr, SocketAddrV4},
    num::NonZeroU16,
    sync::Arc,
    time::Duration,
};

use igd_next::{AddAnyPortError, GetExternalIpError, RemovePortError, SearchError, aio as aigd};
use n0_error::{e, stack_error};
use tracing::debug;

use super::Metrics;

pub type Gateway = aigd::Gateway<aigd::tokio::Tokio>;

use crate::{Protocol, defaults::UPNP_SEARCH_TIMEOUT as SEARCH_TIMEOUT};

/// Seconds we ask the router to maintain the port mapping. Use 2 hours for now.
const PORT_MAPPING_LEASE_DURATION_SECONDS: u32 = 2 * 60 * 60;

/// Tailscale uses the recommended port mapping lifetime for PMP, which is 2 hours. So we assume a
/// half lifetime of 1h. See <https://datatracker.ietf.org/doc/html/rfc6886#section-3.3>
const HALF_LIFETIME: Duration = Duration::from_secs(60 * 60);

/// Name with which we register the mapping in the router.
const PORT_MAPPING_DESCRIPTION: &str = "iroh-portmap";

#[derive(derive_more::Debug, Clone)]
pub struct Mapping {
    /// Protocol for this mapping.
    protocol: igd_next::PortMappingProtocol,
    /// The internet Gateway device (router) used to create this mapping.
    #[debug("{}", gateway)]
    gateway: Gateway,
    /// The external address obtained by this mapping.
    external_ip: Ipv4Addr,
    /// External port obtained by this mapping.
    external_port: NonZeroU16,
}

#[allow(missing_docs)]
#[stack_error(derive, add_meta, std_sources, from_sources)]
#[non_exhaustive]
pub enum Error {
    #[error("Zero external port")]
    ZeroExternalPort {},
    #[error("igd device's external ip is ipv6")]
    NotIpv4 {},
    #[error("Remove Port")]
    RemovePort { source: RemovePortError },
    #[error("Search")]
    Search { source: SearchError },
    #[error("Get external IP")]
    GetExternalIp { source: GetExternalIpError },
    #[error("Add any port")]
    AddAnyPort { source: AddAnyPortError },
    #[error("IO")]
    Io { source: std::io::Error },
}

impl Mapping {
    pub(crate) async fn new(
        protocol: Protocol,
        local_addr: Ipv4Addr,
        port: NonZeroU16,
        gateway: Option<Gateway>,
        preferred_port: Option<NonZeroU16>,
    ) -> Result<Self, Error> {
        let local_addr = SocketAddrV4::new(local_addr, port.into());

        // search for a gateway if there is not one already
        let gateway = if let Some(known_gateway) = gateway {
            known_gateway
        } else {
            // Wrap in manual timeout, because igd_next doesn't respect the set timeout
            tokio::time::timeout(
                SEARCH_TIMEOUT,
                aigd::tokio::search_gateway(igd_next::SearchOptions {
                    timeout: Some(SEARCH_TIMEOUT),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "read timeout".to_string())
            })??
        };

        let std::net::IpAddr::V4(external_ip) = gateway.get_external_ip().await? else {
            return Err(e!(Error::NotIpv4));
        };

        let protocol = match protocol {
            Protocol::Udp => igd_next::PortMappingProtocol::UDP,
            Protocol::Tcp => igd_next::PortMappingProtocol::TCP,
        };

        // if we are trying to get a specific external port, try this first. If this fails, default
        // to try to get any port
        if let Some(external_port) = preferred_port
            && gateway
                .add_port(
                    protocol,
                    external_port.into(),
                    local_addr.into(),
                    PORT_MAPPING_LEASE_DURATION_SECONDS,
                    PORT_MAPPING_DESCRIPTION,
                )
                .await
                .is_ok()
        {
            return Ok(Mapping {
                protocol,
                gateway,
                external_ip,
                external_port,
            });
        }

        let external_port = gateway
            .add_any_port(
                protocol,
                local_addr.into(),
                PORT_MAPPING_LEASE_DURATION_SECONDS,
                PORT_MAPPING_DESCRIPTION,
            )
            .await?
            .try_into()
            .map_err(|_| e!(Error::ZeroExternalPort))?;

        Ok(Mapping {
            protocol,
            gateway,
            external_ip,
            external_port,
        })
    }

    pub fn half_lifetime(&self) -> Duration {
        HALF_LIFETIME
    }

    /// Releases the mapping.
    pub(crate) async fn release(self) -> Result<(), Error> {
        let Mapping {
            gateway,
            external_port,
            protocol,
            ..
        } = self;
        gateway.remove_port(protocol, external_port.into()).await?;
        Ok(())
    }

    /// Returns the external gateway ip and port that can be used to contact this node.
    pub fn external(&self) -> (Ipv4Addr, NonZeroU16) {
        (self.external_ip, self.external_port)
    }
}

/// Searches for UPnP gateways.
pub async fn probe_available(metrics: &Arc<Metrics>) -> Option<Gateway> {
    metrics.upnp_probes.inc();

    // Wrap in manual timeout, because igd_next doesn't respect the set timeout
    let res = tokio::time::timeout(
        SEARCH_TIMEOUT,
        aigd::tokio::search_gateway(igd_next::SearchOptions {
            timeout: Some(SEARCH_TIMEOUT),
            ..Default::default()
        }),
    )
    .await;

    match res {
        Ok(Ok(gateway)) => Some(gateway),
        Err(e) => {
            metrics.upnp_probes_failed.inc();
            debug!("upnp probe timed out: {e}");
            None
        }
        Ok(Err(e)) => {
            metrics.upnp_probes_failed.inc();
            debug!("upnp probe failed: {e}");
            None
        }
    }
}
