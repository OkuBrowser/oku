// SPDX-License-Identifier: MIT

mod bridge;
mod in6_addr_gen_mode;
mod inet;
mod inet6;
mod inet6_cache;
mod inet6_devconf;
mod inet6_icmp;
mod inet6_iface_flag;
mod inet6_stats;
mod mctp;
mod unspec;

#[cfg(any(
    target_os = "linux",
    target_os = "fuchsia",
    target_os = "android"
))]
pub(crate) use self::bridge::VecAfSpecBridge;
pub use self::{
    bridge::{
        AfSpecBridge, BridgeFlag, BridgeMode, BridgeVlanInfo,
        BridgeVlanInfoFlags, BridgeVlanTunnelInfo,
    },
    in6_addr_gen_mode::In6AddrGenMode,
    inet::{AfSpecInet, InetDevConf},
    inet6::AfSpecInet6,
    inet6_cache::{Inet6CacheInfo, Inet6CacheInfoBuffer},
    inet6_devconf::{Inet6DevConf, Inet6DevConfBuffer},
    inet6_icmp::{Icmp6Stats, Icmp6StatsBuffer},
    inet6_iface_flag::Inet6IfaceFlags,
    inet6_stats::{Inet6Stats, Inet6StatsBuffer},
    mctp::{AfSpecMctp, MctpPhysBinding},
    unspec::AfSpecUnspec,
};
pub(crate) use self::{
    inet::VecAfSpecInet, inet6::VecAfSpecInet6, mctp::VecAfSpecMctp,
    unspec::VecAfSpecUnspec,
};
