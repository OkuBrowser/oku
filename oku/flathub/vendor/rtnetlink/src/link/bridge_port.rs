// SPDX-License-Identifier: MIT

use crate::{
    packet_route::link::{
        BridgeMulticastRouterType, BridgePortState, InfoBridgePort,
        InfoPortData, InfoPortKind,
    },
    LinkMessageBuilder,
};

#[derive(Debug)]
pub struct LinkBridgePort;

impl LinkBridgePort {
    pub fn new(port_index: u32) -> LinkMessageBuilder<Self> {
        LinkMessageBuilder::<LinkBridgePort>::default()
            .index(port_index)
            .set_port_kind(InfoPortKind::Bridge)
    }
}

impl LinkMessageBuilder<LinkBridgePort> {
    /// Append arbitrary [InfoBridgePort]
    pub fn append_info_data(self, info: InfoBridgePort) -> Self {
        let mut ret = self;

        if let InfoPortData::BridgePort(infos) = ret
            .port_data
            .get_or_insert_with(|| InfoPortData::BridgePort(Vec::new()))
        {
            infos.push(info);
        }

        ret
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave fdb_flush`.
    pub fn fdb_flush(self) -> Self {
        self.append_info_data(InfoBridgePort::Flush)
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave state STATE`.
    pub fn state(self, v: BridgePortState) -> Self {
        self.append_info_data(InfoBridgePort::State(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave priority PRIO`.
    pub fn priority(self, v: u16) -> Self {
        self.append_info_data(InfoBridgePort::Priority(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave cost COST`.
    pub fn cost(self, v: u32) -> Self {
        self.append_info_data(InfoBridgePort::Cost(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave guard { on | off }`.
    pub fn guard(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::Guard(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave hairpin { on | off }`.
    pub fn hairpin(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::HairpinMode(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave root_block { on | off }`.
    pub fn root_block(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::Protect(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave learning { on | off }`.
    pub fn learning(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::Learning(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave flood { on | off }`.
    pub fn flood(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::UnicastFlood(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave proxy_arp { on | off }`.
    pub fn proxy_arp(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::ProxyARP(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave proxy_arp_wifi { on | off }`.
    pub fn proxy_arp_wifi(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::ProxyARPWifi(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave mcast_router MULTICAST_ROUTER`.
    pub fn mcast_router(self, v: BridgeMulticastRouterType) -> Self {
        self.append_info_data(InfoBridgePort::MulticastRouter(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave mcast_fast_leave { on | off }`.
    /// and
    /// `ip link set name NAME type bridge_slave fastleave { on | off }`.
    pub fn mcast_fast_leave(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::FastLeave(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave bcast_flood { on | off }`.
    pub fn bcast_flood(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::BroadcastFlood(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave mcast_flood { on | off }`.
    pub fn mcast_flood(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::MulticastFlood(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave mcast_to_unicast { on | off }`.
    pub fn mcast_to_unicast(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::MulticastToUnicast(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave group_fwd_mask MASK`.
    pub fn group_fwd_mask(self, v: u16) -> Self {
        self.append_info_data(InfoBridgePort::GroupFwdMask(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave neigh_suppress { on | off }`.
    pub fn neigh_suppress(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::NeighSupress(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave \
    ///     neigh_vlan_suppress { on | off }`.
    pub fn neigh_vlan_suppress(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::NeighVlanSupress(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave vlan_tunnel { on | off }`.
    pub fn vlan_tunnel(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::VlanTunnel(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave isolated { on | off }`.
    pub fn isolated(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::Isolated(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave locked { on | off }`.
    pub fn locked(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::Locked(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave mab { on | off }`.
    pub fn mab(self, v: bool) -> Self {
        self.append_info_data(InfoBridgePort::Mab(v))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave backup_port DEVICE`
    /// but only accept interface index. Setting to 0 equal to
    /// `Self::nobackup_port()`.
    pub fn backup_port(self, iface_index: u32) -> Self {
        self.append_info_data(InfoBridgePort::BackupPort(iface_index))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave nobackup_port`
    pub fn nobackup_port(self) -> Self {
        self.append_info_data(InfoBridgePort::BackupPort(0))
    }

    /// This is equivalent to
    /// `ip link set name NAME type bridge_slave backup_nhid NHID`
    pub fn backup_nhid(self, v: u32) -> Self {
        self.append_info_data(InfoBridgePort::BackupNextHopId(v))
    }
}
