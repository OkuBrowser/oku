// SPDX-License-Identifier: MIT

mod address;
mod attribute;
mod cache_info;
mod flags;
mod header;
mod lwtunnel;
mod message;
pub(crate) mod metrics;
mod mfc_stats;
mod mpls;
mod next_hops;
mod preference;
mod realm;
mod seg6;
mod via;

#[cfg(test)]
mod tests;

pub use flags::RouteFlags;

pub use self::{
    address::RouteAddress,
    attribute::RouteAttribute,
    cache_info::{RouteCacheInfo, RouteCacheInfoBuffer},
    header::{
        RouteHeader, RouteMessageBuffer, RouteProtocol, RouteScope, RouteType,
    },
    lwtunnel::{RouteIp6Tunnel, RouteLwEnCapType, RouteLwTunnelEncap},
    message::RouteMessage,
    metrics::RouteMetric,
    mfc_stats::{RouteMfcStats, RouteMfcStatsBuffer},
    mpls::{MplsLabel, RouteMplsIpTunnel, RouteMplsTtlPropagation},
    next_hops::{RouteNextHop, RouteNextHopBuffer, RouteNextHopFlags},
    preference::RoutePreference,
    realm::RouteRealm,
    seg6::{RouteSeg6IpTunnel, Seg6Header, Seg6Mode},
    via::{RouteVia, RouteViaBuffer},
};
