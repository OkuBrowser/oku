//! A port mapping created with one of the supported protocols.

use std::{net::Ipv4Addr, num::NonZeroU16, time::Duration};

use n0_error::stack_error;

use super::{nat_pmp, pcp, upnp};
use crate::Protocol;

pub(super) trait PortMapped: std::fmt::Debug + Unpin {
    fn external(&self) -> (Ipv4Addr, NonZeroU16);
    /// Half the lifetime of a mapping. This is used to calculate when a mapping should be renewed.
    fn half_lifetime(&self) -> Duration;
}

/// A port mapping created with one of the supported protocols.
#[derive(derive_more::Debug)]
pub enum Mapping {
    /// A UPnP mapping.
    Upnp(upnp::Mapping),
    /// A PCP mapping.
    Pcp(pcp::Mapping),
    /// A NAT-PMP mapping.
    NatPmp(nat_pmp::Mapping),
}

/// Mapping error.
#[allow(missing_docs)]
#[stack_error(derive, add_meta, from_sources)]
#[non_exhaustive]
pub enum Error {
    #[error("PCP mapping failed")]
    Pcp { source: pcp::Error },
    #[error("NAT-PMP mapping failed")]
    NatPmp { source: nat_pmp::Error },
    #[error("UPnP mapping failed")]
    Upnp { source: upnp::Error },
}

impl Mapping {
    /// Create a new PCP mapping.
    pub(crate) async fn new_pcp(
        protocol: Protocol,
        local_ip: Ipv4Addr,
        local_port: NonZeroU16,
        gateway: Ipv4Addr,
        external_addr: Option<(Ipv4Addr, NonZeroU16)>,
    ) -> Result<Self, Error> {
        pcp::Mapping::new(protocol, local_ip, local_port, gateway, external_addr)
            .await
            .map(Self::Pcp)
            .map_err(Error::from)
    }

    /// Create a new NAT-PMP mapping.
    pub(crate) async fn new_nat_pmp(
        protocol: Protocol,
        local_ip: Ipv4Addr,
        local_port: NonZeroU16,
        gateway: Ipv4Addr,
        external_addr: Option<(Ipv4Addr, NonZeroU16)>,
    ) -> Result<Self, Error> {
        nat_pmp::Mapping::new(
            protocol,
            local_ip,
            local_port,
            gateway,
            external_addr.map(|(_addr, port)| port),
        )
        .await
        .map(Self::NatPmp)
        .map_err(Error::from)
    }

    /// Create a new UPnP mapping.
    pub(crate) async fn new_upnp(
        protocol: Protocol,
        local_ip: Ipv4Addr,
        local_port: NonZeroU16,
        gateway: Option<upnp::Gateway>,
        external_port: Option<NonZeroU16>,
    ) -> Result<Self, Error> {
        upnp::Mapping::new(protocol, local_ip, local_port, gateway, external_port)
            .await
            .map(Self::Upnp)
            .map_err(Error::from)
    }

    /// Release the mapping.
    pub(crate) async fn release(self) -> Result<(), Error> {
        match self {
            Mapping::Upnp(m) => m.release().await?,
            Mapping::Pcp(m) => m.release().await?,
            Mapping::NatPmp(m) => m.release().await?,
        }
        Ok(())
    }
}

impl PortMapped for Mapping {
    fn external(&self) -> (Ipv4Addr, NonZeroU16) {
        match self {
            Mapping::Upnp(m) => m.external(),
            Mapping::Pcp(m) => m.external(),
            Mapping::NatPmp(m) => m.external(),
        }
    }

    fn half_lifetime(&self) -> Duration {
        match self {
            Mapping::Upnp(m) => m.half_lifetime(),
            Mapping::Pcp(m) => m.half_lifetime(),
            Mapping::NatPmp(m) => m.half_lifetime(),
        }
    }
}
