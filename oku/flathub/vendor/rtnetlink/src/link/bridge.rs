// SPDX-License-Identifier: MIT

use crate::{
    link::LinkMessageBuilder,
    packet_route::link::{
        BridgeBooleanOptionFlags, BridgeBooleanOptions,
        BridgeMulticastRouterType, BridgeStpState, InfoBridge, InfoData,
        InfoKind, VlanProtocol,
    },
};

/// Represent Linux Bridge interface.
/// Example code on creating a linux bridge interface
/// ```no_run
/// use rtnetlink::{new_connection, LinkBridge};
/// #[tokio::main]
/// async fn main() -> Result<(), String> {
///     let (connection, handle, _) = new_connection().unwrap();
///     tokio::spawn(connection);
///
///     handle
///         .link()
///         .add(LinkBridge::new("br0").build())
///         .execute()
///         .await
///         .map_err(|e| format!("{e}"))
/// }
/// ```
///
/// Please check LinkMessageBuilder::<LinkBridge> for more detail.
#[derive(Debug)]
pub struct LinkBridge;

impl LinkBridge {
    /// Equal to `LinkMessageBuilder::<LinkBridge>::new().up()`
    pub fn new(name: &str) -> LinkMessageBuilder<Self> {
        LinkMessageBuilder::<LinkBridge>::new(name).up()
    }
}

impl LinkMessageBuilder<LinkBridge> {
    /// Create [LinkMessageBuilder] for linux bridge
    pub fn new(name: &str) -> Self {
        LinkMessageBuilder::<LinkBridge>::new_with_info_kind(InfoKind::Bridge)
            .name(name.to_string())
    }

    pub fn append_info_data(self, info: InfoBridge) -> Self {
        let mut ret = self;
        if let InfoData::Bridge(infos) = ret
            .info_data
            .get_or_insert_with(|| InfoData::Bridge(Vec::new()))
        {
            infos.push(info);
        }
        ret
    }

    pub fn set_boolean_opt(
        self,
        opt: BridgeBooleanOptionFlags,
        value: bool,
    ) -> Self {
        let mut ret = self;
        if let InfoData::Bridge(infos) = ret
            .info_data
            .get_or_insert_with(|| InfoData::Bridge(Vec::new()))
        {
            let mut found = false;
            for info in infos.iter_mut() {
                if let InfoBridge::MultiBoolOpt(opts) = info {
                    found = true;
                    opts.value.set(opt, value);
                    opts.mask.set(opt, true);
                    break;
                }
            }
            if !found {
                infos.push(InfoBridge::MultiBoolOpt(BridgeBooleanOptions {
                    value: if value {
                        opt
                    } else {
                        BridgeBooleanOptionFlags::empty()
                    },
                    mask: opt,
                }));
            }
        }
        ret
    }

    pub fn ageing_time(self, value: u32) -> Self {
        self.append_info_data(InfoBridge::AgeingTime(value))
    }

    pub fn group_fwd_mask(self, value: u16) -> Self {
        self.append_info_data(InfoBridge::GroupFwdMask(value))
    }

    pub fn group_address(self, value: [u8; 6]) -> Self {
        self.append_info_data(InfoBridge::GroupAddr(value))
    }

    pub fn forward_delay(self, value: u32) -> Self {
        self.append_info_data(InfoBridge::ForwardDelay(value))
    }

    pub fn hello_time(self, value: u32) -> Self {
        self.append_info_data(InfoBridge::HelloTime(value))
    }

    pub fn max_age(self, value: u32) -> Self {
        self.append_info_data(InfoBridge::MaxAge(value))
    }

    pub fn stp_state(self, value: BridgeStpState) -> Self {
        self.append_info_data(InfoBridge::StpState(value))
    }

    pub fn mst_enabled(self, value: bool) -> Self {
        self.set_boolean_opt(BridgeBooleanOptionFlags::MstEnable, value)
    }

    pub fn priority(self, value: u16) -> Self {
        self.append_info_data(InfoBridge::Priority(value))
    }

    pub fn no_linklocal_learn(self, value: bool) -> Self {
        self.set_boolean_opt(BridgeBooleanOptionFlags::NoLinkLocalLearn, value)
    }

    pub fn fdb_local_vlan_0(self, value: bool) -> Self {
        self.set_boolean_opt(BridgeBooleanOptionFlags::FdbLocalVlan0, value)
    }

    pub fn fdb_max_learned(self, value: u32) -> Self {
        self.append_info_data(InfoBridge::FdbMaxLearned(value))
    }

    pub fn vlan_filtering(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::VlanFiltering(value))
    }

    pub fn vlan_protocol(self, value: VlanProtocol) -> Self {
        self.append_info_data(InfoBridge::VlanProtocol(value))
    }

    pub fn vlan_default_pvid(self, value: u16) -> Self {
        self.append_info_data(InfoBridge::VlanDefaultPvid(value))
    }

    pub fn vlan_stats_enabled(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::VlanStatsEnabled(value))
    }

    pub fn vlan_stats_per_port(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::VlanStatsPerPort(value))
    }

    pub fn mcast_snooping(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::MulticastSnooping(value))
    }

    pub fn mcast_vlan_snooping(self, value: bool) -> Self {
        self.set_boolean_opt(
            BridgeBooleanOptionFlags::VlanMulticastSnooping,
            value,
        )
    }

    pub fn mcast_router(self, value: BridgeMulticastRouterType) -> Self {
        self.append_info_data(InfoBridge::MulticastRouter(value))
    }

    pub fn mcast_query_use_ifaddr(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::MulticastQueryUseIfaddr(value))
    }

    pub fn mcast_querier(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::MulticastQuerier(value))
    }

    pub fn mcast_hash_max(self, value: u32) -> Self {
        self.append_info_data(InfoBridge::MulticastHashMax(value))
    }

    pub fn mcast_last_member_count(self, value: u32) -> Self {
        self.append_info_data(InfoBridge::MulticastLastMemberCount(value))
    }

    pub fn mcast_startup_query_count(self, value: u32) -> Self {
        self.append_info_data(InfoBridge::MulticastStartupQueryCount(value))
    }

    pub fn mcast_last_member_interval(self, value: u64) -> Self {
        self.append_info_data(InfoBridge::MulticastLastMemberInterval(value))
    }

    pub fn mcast_membership_interval(self, value: u64) -> Self {
        self.append_info_data(InfoBridge::MulticastMembershipInterval(value))
    }

    pub fn mcast_querier_interval(self, value: u64) -> Self {
        self.append_info_data(InfoBridge::MulticastQuerierInterval(value))
    }

    pub fn mcast_query_interval(self, value: u64) -> Self {
        self.append_info_data(InfoBridge::MulticastQueryInterval(value))
    }

    pub fn mcast_query_response_interval(self, value: u64) -> Self {
        self.append_info_data(InfoBridge::MulticastQueryResponseInterval(value))
    }

    pub fn mcast_startup_query_interval(self, value: u64) -> Self {
        self.append_info_data(InfoBridge::MulticastStartupQueryInterval(value))
    }

    pub fn mcast_stats_enabled(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::MulticastStatsEnabled(value))
    }

    pub fn mcast_igmp_version(self, value: u8) -> Self {
        self.append_info_data(InfoBridge::MulticastIgmpVersion(value))
    }

    pub fn mcast_mld_version(self, value: u8) -> Self {
        self.append_info_data(InfoBridge::MulticastMldVersion(value))
    }

    pub fn nf_call_iptables(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::NfCallIpTables(value))
    }

    pub fn nf_call_ip6tables(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::NfCallIp6Tables(value))
    }

    pub fn nf_call_arptables(self, value: bool) -> Self {
        self.append_info_data(InfoBridge::NfCallArpTables(value))
    }

    pub fn mdb_offload_fail_notification(self, value: bool) -> Self {
        self.set_boolean_opt(
            BridgeBooleanOptionFlags::MdbOffloadFailNotif,
            value,
        )
    }
}
