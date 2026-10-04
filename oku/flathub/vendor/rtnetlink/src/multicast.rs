// SPDX-License-Identifier: MIT

// const RTNLGRP_NONE: u32 = 0;
const RTNLGRP_LINK: u32 = 1;
const RTNLGRP_NOTIFY: u32 = 2;
const RTNLGRP_NEIGH: u32 = 3;
const RTNLGRP_TC: u32 = 4;
const RTNLGRP_IPV4_IFADDR: u32 = 5;
const RTNLGRP_IPV4_MROUTE: u32 = 6;
const RTNLGRP_IPV4_ROUTE: u32 = 7;
const RTNLGRP_IPV4_RULE: u32 = 8;
const RTNLGRP_IPV6_IFADDR: u32 = 9;
const RTNLGRP_IPV6_MROUTE: u32 = 10;
const RTNLGRP_IPV6_ROUTE: u32 = 11;
const RTNLGRP_IPV6_IFINFO: u32 = 12;
const RTNLGRP_DECNET_IFADDR: u32 = 13;
// const RTNLGRP_NOP2: u32 = 14
const RTNLGRP_DECNET_ROUTE: u32 = 15;
const RTNLGRP_DECNET_RULE: u32 = 16;
// const RTNLGRP_NOP4: u32 = 17;
const RTNLGRP_IPV6_PREFIX: u32 = 18;
const RTNLGRP_IPV6_RULE: u32 = 19;
const RTNLGRP_ND_USEROPT: u32 = 20;
const RTNLGRP_PHONET_IFADDR: u32 = 21;
const RTNLGRP_PHONET_ROUTE: u32 = 22;
const RTNLGRP_DCB: u32 = 23;
const RTNLGRP_IPV4_NETCONF: u32 = 24;
const RTNLGRP_IPV6_NETCONF: u32 = 25;
const RTNLGRP_MDB: u32 = 26;
const RTNLGRP_MPLS_ROUTE: u32 = 27;
const RTNLGRP_NSID: u32 = 28;
const RTNLGRP_MPLS_NETCONF: u32 = 29;
const RTNLGRP_IPV4_MROUTE_R: u32 = 30;
const RTNLGRP_IPV6_MROUTE_R: u32 = 31;
const RTNLGRP_NEXTHOP: u32 = 32;
const RTNLGRP_BRVLAN: u32 = 33;
const RTNLGRP_MCTP_IFADDR: u32 = 34;
const RTNLGRP_TUNNEL: u32 = 35;
const RTNLGRP_STATS: u32 = 36;
const RTNLGRP_IPV4_MCADDR: u32 = 37;
const RTNLGRP_IPV6_MCADDR: u32 = 38;
const RTNLGRP_IPV6_ACADDR: u32 = 39;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
#[non_exhaustive]
pub enum MulticastGroup {
    Link = RTNLGRP_LINK,
    Notify = RTNLGRP_NOTIFY,
    Neigh = RTNLGRP_NEIGH,
    Tc = RTNLGRP_TC,
    Ipv4Ifaddr = RTNLGRP_IPV4_IFADDR,
    Ipv4Mroute = RTNLGRP_IPV4_MROUTE,
    Ipv4Route = RTNLGRP_IPV4_ROUTE,
    Ipv4Rule = RTNLGRP_IPV4_RULE,
    Ipv6Ifaddr = RTNLGRP_IPV6_IFADDR,
    Ipv6Mroute = RTNLGRP_IPV6_MROUTE,
    Ipv6Route = RTNLGRP_IPV6_ROUTE,
    Ipv6Ifinfo = RTNLGRP_IPV6_IFINFO,
    DecnetIfaddr = RTNLGRP_DECNET_IFADDR,
    DecnetRoute = RTNLGRP_DECNET_ROUTE,
    DecnetRule = RTNLGRP_DECNET_RULE,
    Ipv6Prefix = RTNLGRP_IPV6_PREFIX,
    Ipv6Rule = RTNLGRP_IPV6_RULE,
    NdUseropt = RTNLGRP_ND_USEROPT,
    PhonetIfaddr = RTNLGRP_PHONET_IFADDR,
    PhonetRoute = RTNLGRP_PHONET_ROUTE,
    Dcb = RTNLGRP_DCB,
    Ipv4Netconf = RTNLGRP_IPV4_NETCONF,
    Ipv6Netconf = RTNLGRP_IPV6_NETCONF,
    Mdb = RTNLGRP_MDB,
    MplsRoute = RTNLGRP_MPLS_ROUTE,
    Nsid = RTNLGRP_NSID,
    MplsNetconf = RTNLGRP_MPLS_NETCONF,
    Ipv4MrouteR = RTNLGRP_IPV4_MROUTE_R,
    Ipv6MrouteR = RTNLGRP_IPV6_MROUTE_R,
    Nexthop = RTNLGRP_NEXTHOP,
    Brvlan = RTNLGRP_BRVLAN,
    MctpIfaddr = RTNLGRP_MCTP_IFADDR,
    Tunnel = RTNLGRP_TUNNEL,
    Stats = RTNLGRP_STATS,
    Ipv4Mcaddr = RTNLGRP_IPV4_MCADDR,
    Ipv6Mcaddr = RTNLGRP_IPV6_MCADDR,
    Ipv6Acaddr = RTNLGRP_IPV6_ACADDR,
}

impl MulticastGroup {
    /// Whether need to use `netlink_sys::Socket::add_membership()`.
    pub fn need_via_add_membership(self) -> bool {
        self as u32 > 31
    }
}
