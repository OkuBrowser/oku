// SPDX-License-Identifier: MIT

//! This crate provides methods to manipulate networking resources (links,
//! addresses, arp tables, route tables) via the netlink protocol.

#![allow(clippy::module_inception)]

pub use netlink_packet_core as packet_core;
pub use netlink_packet_route as packet_route;
pub use netlink_proto as proto;
pub use netlink_sys as sys;

mod addr;
mod connection;
pub mod constants;
mod errors;
mod handle;
mod link;
mod macros;
mod multicast;
mod neighbour;
#[cfg(not(target_os = "freebsd"))]
mod ns;
mod route;
mod rule;
#[cfg(not(target_os = "freebsd"))]
mod traffic_control;

#[cfg(feature = "tokio_socket")]
pub use crate::connection::{new_connection, new_multicast_connection};
#[cfg(not(target_os = "freebsd"))]
pub use crate::ns::{NetworkNamespace, NETNS_PATH, NONE_FS, SELF_NS_PATH};
#[cfg(not(target_os = "freebsd"))]
pub use crate::traffic_control::{
    QDiscDelRequest, QDiscGetRequest, QDiscHandle, QDiscNewRequest,
    TrafficChainGetRequest, TrafficChainHandle, TrafficClassGetRequest,
    TrafficClassHandle, TrafficFilterGetRequest, TrafficFilterHandle,
    TrafficFilterNewRequest,
};
pub use crate::{
    addr::{
        AddressAddRequest, AddressDelRequest, AddressGetRequest, AddressHandle,
        AddressMessageBuilder,
    },
    connection::{
        from_socket, new_connection_with_socket,
        new_multicast_connection_with_socket,
    },
    errors::Error,
    handle::Handle,
    link::{
        LinkAddRequest, LinkBond, LinkBondPort, LinkBridge, LinkBridgePort,
        LinkBridgeVlan, LinkDelPropRequest, LinkDelRequest, LinkDummy,
        LinkGetRequest, LinkHandle, LinkMacSec, LinkMacVlan, LinkMacVtap,
        LinkMessageBuilder, LinkNetkit, LinkSetRequest, LinkUnspec, LinkVeth,
        LinkVlan, LinkVrf, LinkVxlan, LinkWireguard, LinkXfrm, QosMapping,
    },
    multicast::MulticastGroup,
    neighbour::{
        NeighbourAddRequest, NeighbourDelRequest, NeighbourGetRequest,
        NeighbourHandle,
    },
    route::{
        IpVersion, RouteAddRequest, RouteDelRequest, RouteGetRequest,
        RouteHandle, RouteMessageBuilder, RouteNextHopBuilder,
    },
    rule::{RuleAddRequest, RuleDelRequest, RuleGetRequest, RuleHandle},
};
