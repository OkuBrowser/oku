// SPDX-License-Identifier: MIT

use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

#[cfg(not(target_os = "android"))]
use netlink_packet_route::route::{
    MplsLabel, RouteLwEnCapType, RouteLwTunnelEncap, RouteMplsIpTunnel,
};
use netlink_packet_route::{
    route::{
        RouteAddress, RouteAttribute, RouteFlags, RouteHeader, RouteMessage,
        RouteNextHop, RouteNextHopFlags, RouteProtocol, RouteScope, RouteType,
        RouteVia,
    },
    AddressFamily,
};

#[derive(Debug, Clone)]
pub struct RouteMessageBuilder<T = IpAddr> {
    message: RouteMessage,
    _phantom: PhantomData<T>,
}

#[derive(Debug, Clone)]
pub struct RouteNextHopBuilder {
    address_family: AddressFamily,
    nexthop: RouteNextHop,
}

impl<T> RouteMessageBuilder<T> {
    /// Create default RouteMessage with header set to:
    ///  * route: [RouteHeader::RT_TABLE_MAIN]
    ///  * protocol: [RouteProtocol::Static]
    ///  * scope: [RouteScope::Universe]
    ///  * kind: [RouteType::Unicast]
    ///
    /// For using this message in querying routes, these settings
    /// are ignored unless `NETLINK_GET_STRICT_CHK` been enabled.
    fn new_no_address_family() -> Self {
        let mut message = RouteMessage::default();
        message.header.table = RouteHeader::RT_TABLE_MAIN;
        message.header.protocol = RouteProtocol::Static;
        message.header.scope = RouteScope::Universe;
        message.header.kind = RouteType::Unicast;
        Self {
            message,
            _phantom: Default::default(),
        }
    }

    /// Sets the input interface index.
    pub fn input_interface(mut self, index: u32) -> Self {
        self.message.attributes.push(RouteAttribute::Iif(index));
        self
    }

    /// Sets the output interface index.
    pub fn output_interface(mut self, index: u32) -> Self {
        self.message.attributes.push(RouteAttribute::Oif(index));
        self
    }

    /// Sets the output MPLS encapsulation labels.
    #[cfg(not(target_os = "android"))]
    pub fn output_mpls(mut self, labels: Vec<MplsLabel>) -> Self {
        if labels.is_empty() {
            return self;
        }
        if self.message.header.address_family == AddressFamily::Mpls {
            self.message
                .attributes
                .push(RouteAttribute::NewDestination(labels));
        } else {
            self.message
                .attributes
                .push(RouteAttribute::EncapType(RouteLwEnCapType::Mpls));
            let encap = RouteLwTunnelEncap::Mpls(
                RouteMplsIpTunnel::Destination(labels),
            );
            self.message
                .attributes
                .push(RouteAttribute::Encap(vec![encap]));
        }
        self
    }

    /// Sets multiple nexthop entries for the route.
    pub fn multipath(mut self, nexthops: Vec<RouteNextHop>) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::MultiPath(nexthops));
        self
    }

    /// Sets the route priority (metric)
    pub fn priority(mut self, priority: u32) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Priority(priority));
        self
    }

    /// Sets the route table ID.
    ///
    /// Default is main route table.
    pub fn table_id(mut self, table: u32) -> Self {
        if table > 255 {
            self.message.attributes.push(RouteAttribute::Table(table));
        } else {
            self.message.header.table = table as u8;
        }
        self
    }

    #[cfg(not(target_os = "android"))]
    /// Sets mark value on route.
    pub fn mark(mut self, mark: u32) -> Self {
        self.message.attributes.push(RouteAttribute::Mark(mark));
        self
    }

    /// Sets the route protocol.
    ///
    /// Default is static route protocol.
    pub fn protocol(mut self, protocol: RouteProtocol) -> Self {
        self.message.header.protocol = protocol;
        self
    }

    /// Sets the route scope.
    ///
    /// Default is universe route scope.
    pub fn scope(mut self, scope: RouteScope) -> Self {
        self.message.header.scope = scope;
        self
    }

    /// Sets the route kind.
    ///
    /// Default is unicast route kind.
    pub fn kind(mut self, kind: RouteType) -> Self {
        self.message.header.kind = kind;
        self
    }

    /// Marks the next hop as directly reachable (on-link).
    ///
    /// Indicates that the next hop is reachable without passing through a
    /// connected subnet.
    pub fn onlink(mut self) -> Self {
        self.message.header.flags.insert(RouteFlags::Onlink);
        self
    }

    /// Return a mutable reference to the request message.
    pub fn get_mut(&mut self) -> &mut RouteMessage {
        &mut self.message
    }

    pub fn build(self) -> RouteMessage {
        self.message
    }
}

impl RouteMessageBuilder<Ipv4Addr> {
    /// Create default RouteMessage with header set to:
    ///  * route: [RouteHeader::RT_TABLE_MAIN]
    ///  * protocol: [RouteProtocol::Static]
    ///  * scope: [RouteScope::Universe]
    ///  * kind: [RouteType::Unicast]
    ///  * address_family: [AddressFamily::Inet4]
    ///
    /// For using this message in querying routes, these settings
    /// are ignored unless `NETLINK_GET_STRICT_CHK` been enabled.
    pub fn new() -> Self {
        let mut builder = Self::new_no_address_family();
        builder.get_mut().header.address_family = AddressFamily::Inet;
        builder
    }

    /// Sets the source address prefix.
    pub fn source_prefix(mut self, addr: Ipv4Addr, prefix_length: u8) -> Self {
        self.message.header.source_prefix_length = prefix_length;
        self.message
            .attributes
            .push(RouteAttribute::Source(RouteAddress::Inet(addr)));
        self
    }

    /// Sets the preferred source address.
    pub fn pref_source(mut self, addr: Ipv4Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::PrefSource(RouteAddress::Inet(addr)));
        self
    }

    /// Sets the destination address prefix.
    pub fn destination_prefix(
        mut self,
        addr: Ipv4Addr,
        prefix_length: u8,
    ) -> Self {
        self.message.header.destination_prefix_length = prefix_length;
        self.message
            .attributes
            .push(RouteAttribute::Destination(RouteAddress::Inet(addr)));
        self
    }

    /// Sets the gateway (via) address.
    pub fn gateway(mut self, addr: Ipv4Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Gateway(RouteAddress::Inet(addr)));
        self
    }

    /// Sets the IPv6 gateway (via) address.
    pub fn via(mut self, addr: Ipv6Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Via(RouteVia::Inet6(addr)));
        self
    }
}

impl Default for RouteMessageBuilder<Ipv4Addr> {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteMessageBuilder<Ipv6Addr> {
    /// Create default RouteMessage with header set to:
    ///  * route: [RouteHeader::RT_TABLE_MAIN]
    ///  * protocol: [RouteProtocol::Static]
    ///  * scope: [RouteScope::Universe]
    ///  * kind: [RouteType::Unicast]
    ///  * address_family: [AddressFamily::Inet6]
    ///
    /// For using this message in querying routes, these settings
    /// are ignored unless `NETLINK_GET_STRICT_CHK` been enabled.
    pub fn new() -> Self {
        let mut builder = Self::new_no_address_family();
        builder.get_mut().header.address_family = AddressFamily::Inet6;
        builder
    }

    /// Sets the source address prefix.
    pub fn source_prefix(mut self, addr: Ipv6Addr, prefix_length: u8) -> Self {
        self.message.header.source_prefix_length = prefix_length;
        self.message
            .attributes
            .push(RouteAttribute::Source(RouteAddress::Inet6(addr)));
        self
    }

    /// Sets the preferred source address.
    pub fn pref_source(mut self, addr: Ipv6Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::PrefSource(RouteAddress::Inet6(addr)));
        self
    }

    /// Sets the destination address prefix.
    pub fn destination_prefix(
        mut self,
        addr: Ipv6Addr,
        prefix_length: u8,
    ) -> Self {
        self.message.header.destination_prefix_length = prefix_length;
        self.message
            .attributes
            .push(RouteAttribute::Destination(RouteAddress::Inet6(addr)));
        self
    }

    /// Sets the gateway (via) address.
    pub fn gateway(mut self, addr: Ipv6Addr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Gateway(RouteAddress::Inet6(addr)));
        self
    }
}

impl Default for RouteMessageBuilder<Ipv6Addr> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidRouteMessage {
    #[error("invalid address family {:?}", _0)]
    AddressFamily(AddressFamily),

    #[error("invalid gateway {}", _0)]
    Gateway(IpAddr),

    #[error("invalid preferred source {}", _0)]
    PrefSource(IpAddr),

    #[error("invalid source prefix {}/{}", _0, _1)]
    SourcePrefix(IpAddr, u8),

    #[error("invalid destination prefix {}/{}", _0, _1)]
    DestinationPrefix(IpAddr, u8),
}

impl RouteMessageBuilder<IpAddr> {
    pub fn new() -> Self {
        Self::new_no_address_family()
    }

    /// Sets the source address prefix.
    pub fn source_prefix(
        mut self,
        addr: IpAddr,
        prefix_length: u8,
    ) -> Result<Self, InvalidRouteMessage> {
        self.set_address_family_from_ip_addr(addr);
        match self.message.header.address_family {
            AddressFamily::Inet => {
                if addr.is_ipv6() || prefix_length > 32 {
                    return Err(InvalidRouteMessage::SourcePrefix(
                        addr,
                        prefix_length,
                    ));
                }
            }
            AddressFamily::Inet6 => {
                if addr.is_ipv4() || prefix_length > 128 {
                    return Err(InvalidRouteMessage::SourcePrefix(
                        addr,
                        prefix_length,
                    ));
                }
            }
            af => return Err(InvalidRouteMessage::AddressFamily(af)),
        };
        self.message
            .attributes
            .push(RouteAttribute::Source(addr.into()));
        self.message.header.source_prefix_length = prefix_length;
        Ok(self)
    }

    /// Sets the preferred source address.
    pub fn pref_source(
        mut self,
        addr: IpAddr,
    ) -> Result<Self, InvalidRouteMessage> {
        self.set_address_family_from_ip_addr(addr);
        match self.message.header.address_family {
            AddressFamily::Inet => {
                if addr.is_ipv6() {
                    return Err(InvalidRouteMessage::PrefSource(addr));
                };
            }
            AddressFamily::Inet6 => {
                if addr.is_ipv4() {
                    return Err(InvalidRouteMessage::PrefSource(addr));
                };
            }
            af => {
                return Err(InvalidRouteMessage::AddressFamily(af));
            }
        }
        self.message
            .attributes
            .push(RouteAttribute::PrefSource(addr.into()));
        Ok(self)
    }

    /// Sets the destination address prefix.
    pub fn destination_prefix(
        mut self,
        addr: IpAddr,
        prefix_length: u8,
    ) -> Result<Self, InvalidRouteMessage> {
        self.set_address_family_from_ip_addr(addr);
        match self.message.header.address_family {
            AddressFamily::Inet => {
                if addr.is_ipv6() || prefix_length > 32 {
                    return Err(InvalidRouteMessage::DestinationPrefix(
                        addr,
                        prefix_length,
                    ));
                }
            }
            AddressFamily::Inet6 => {
                if addr.is_ipv4() || prefix_length > 128 {
                    return Err(InvalidRouteMessage::DestinationPrefix(
                        addr,
                        prefix_length,
                    ));
                }
            }
            af => {
                return Err(InvalidRouteMessage::AddressFamily(af));
            }
        };
        self.message.header.destination_prefix_length = prefix_length;
        self.message
            .attributes
            .push(RouteAttribute::Destination(addr.into()));
        Ok(self)
    }

    /// Sets the gateway (via) address.
    pub fn gateway(
        mut self,
        addr: IpAddr,
    ) -> Result<Self, InvalidRouteMessage> {
        let attr = match (self.message.header.address_family, addr) {
            (AddressFamily::Inet, addr @ IpAddr::V4(_))
            | (AddressFamily::Inet6, addr @ IpAddr::V6(_)) => {
                RouteAttribute::Gateway(addr.into())
            }
            (AddressFamily::Inet, IpAddr::V6(v6)) => {
                RouteAttribute::Via(RouteVia::Inet6(v6))
            }
            (af, _) => return Err(InvalidRouteMessage::AddressFamily(af)),
        };
        self.message.attributes.push(attr);
        Ok(self)
    }

    /// If it is not set already, set the address family based on the
    /// given IP address. This is a noop is the address family is
    /// already set.
    fn set_address_family_from_ip_addr(&mut self, addr: IpAddr) {
        if self.message.header.address_family != AddressFamily::Unspec {
            return;
        }
        if addr.is_ipv4() {
            self.message.header.address_family = AddressFamily::Inet;
        } else {
            self.message.header.address_family = AddressFamily::Inet6;
        }
    }
}

impl Default for RouteMessageBuilder<IpAddr> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "android"))]
impl RouteMessageBuilder<MplsLabel> {
    /// Create default RouteMessage with header set to:
    ///  * route: [RouteHeader::RT_TABLE_MAIN]
    ///  * protocol: [RouteProtocol::Static]
    ///  * scope: [RouteScope::Universe]
    ///  * kind: [RouteType::Unicast]
    ///  * address_family: [AddressFamily::Mpls]
    ///
    /// For using this message in querying routes, these settings
    /// are ignored unless `NETLINK_GET_STRICT_CHK` been enabled.
    pub fn new() -> Self {
        let mut builder = Self::new_no_address_family();
        builder.get_mut().header.address_family = AddressFamily::Mpls;
        builder
    }

    #[cfg(not(target_os = "android"))]
    /// Sets the destination MPLS label.
    pub fn label(mut self, label: MplsLabel) -> Self {
        self.message.header.address_family = AddressFamily::Mpls;
        self.message.header.destination_prefix_length = 20;
        self.message
            .attributes
            .push(RouteAttribute::Destination(RouteAddress::Mpls(label)));
        self
    }

    /// Sets the gateway (via) address.
    pub fn via(mut self, addr: IpAddr) -> Self {
        self.message
            .attributes
            .push(RouteAttribute::Via(addr.into()));
        self
    }
}
#[cfg(not(target_os = "android"))]
impl Default for RouteMessageBuilder<MplsLabel> {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteNextHopBuilder {
    /// Create default RouteNexthop for a route with the given address family.
    pub fn new(address_family: AddressFamily) -> Self {
        Self {
            address_family,
            nexthop: Default::default(),
        }
    }

    /// Create IPv4 RouteNexthop
    pub fn new_ipv4() -> Self {
        Self {
            address_family: AddressFamily::Inet,
            nexthop: Default::default(),
        }
    }

    /// Create IPv6 RouteNexthop
    pub fn new_ipv6() -> Self {
        Self {
            address_family: AddressFamily::Inet6,
            nexthop: Default::default(),
        }
    }

    /// Sets the nexthop interface index.
    pub fn interface(mut self, index: u32) -> Self {
        self.nexthop.interface_index = index;
        self
    }

    /// Sets the nexthop (via) address.
    pub fn via(mut self, addr: IpAddr) -> Result<Self, InvalidRouteMessage> {
        let attr = match (self.address_family, addr) {
            (AddressFamily::Inet, addr @ IpAddr::V4(_))
            | (AddressFamily::Inet6, addr @ IpAddr::V6(_)) => {
                RouteAttribute::Gateway(addr.into())
            }
            (AddressFamily::Inet, IpAddr::V6(v6)) => {
                RouteAttribute::Via(RouteVia::Inet6(v6))
            }
            #[cfg(not(target_os = "android"))]
            (AddressFamily::Mpls, _) => RouteAttribute::Via(addr.into()),
            (af, _) => return Err(InvalidRouteMessage::AddressFamily(af)),
        };
        self.nexthop.attributes.push(attr);
        Ok(self)
    }

    /// Marks the nexthop as directly reachable (on-link).
    ///
    /// Indicates that the nexthop is reachable without passing through a
    /// connected subnet.
    pub fn onlink(mut self) -> Self {
        self.nexthop.flags.insert(RouteNextHopFlags::Onlink);
        self
    }

    /// Sets the nexthop MPLS encapsulation labels.
    #[cfg(not(target_os = "android"))]
    pub fn mpls(mut self, labels: Vec<MplsLabel>) -> Self {
        if labels.is_empty() {
            return self;
        }
        if self.address_family == AddressFamily::Mpls {
            self.nexthop
                .attributes
                .push(RouteAttribute::NewDestination(labels));
        } else {
            self.nexthop
                .attributes
                .push(RouteAttribute::EncapType(RouteLwEnCapType::Mpls));
            let encap = RouteLwTunnelEncap::Mpls(
                RouteMplsIpTunnel::Destination(labels),
            );
            self.nexthop
                .attributes
                .push(RouteAttribute::Encap(vec![encap]));
        }
        self
    }

    /// Set the nexthop weight
    ///
    /// Equal to `weight` property in `ip route`, but please be advised the
    /// number shown in `ip route` command has plus 1. Meaning kernel has
    /// `weight 0`, but `ip route` shows as `weight 1`. This function is using
    /// kernel number from range of 0 to 255.
    pub fn weight(mut self, weight: u8) -> Self {
        self.nexthop.hops = weight;
        self
    }

    /// Set flags for next hop
    pub fn flags(mut self, flags: RouteNextHopFlags) -> Self {
        self.nexthop.flags = flags;
        self
    }

    pub fn build(self) -> RouteNextHop {
        self.nexthop
    }
}
