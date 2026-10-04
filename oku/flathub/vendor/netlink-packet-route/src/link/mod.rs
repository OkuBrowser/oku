// SPDX-License-Identifier: MIT

mod af_spec;
mod attribute;
mod buffer_tool;
mod devlink_port;
mod down_reason;
mod dpll_pin;
mod event;
pub(crate) mod ext_mask;
mod header;
mod link_flag;
mod link_info;
mod link_layer_type;
mod link_mode;
mod link_state;
mod map;
mod message;
mod phys_id;
mod prop_list;
mod proto_info;
pub(crate) mod sriov;
mod stats;
mod stats64;
mod vlan_protocol;
mod wireless;
mod xdp;

#[cfg(test)]
mod tests;

pub use self::{
    af_spec::{
        AfSpecBridge, AfSpecInet, AfSpecInet6, AfSpecMctp, AfSpecUnspec,
        BridgeFlag, BridgeMode, BridgeVlanInfo, BridgeVlanInfoFlags,
        BridgeVlanTunnelInfo, Icmp6Stats, Icmp6StatsBuffer, In6AddrGenMode,
        Inet6CacheInfo, Inet6CacheInfoBuffer, Inet6DevConf, Inet6DevConfBuffer,
        Inet6IfaceFlags, Inet6Stats, Inet6StatsBuffer, InetDevConf,
        InetDevConfBuffer, MctpPhysBinding,
    },
    attribute::LinkAttribute,
    devlink_port::DevlinkPort,
    down_reason::LinkProtocolDownReason,
    dpll_pin::DpllPin,
    event::LinkEvent,
    ext_mask::LinkExtentMask,
    header::{LinkHeader, LinkMessageBuffer},
    link_flag::LinkFlags,
    link_info::{
        AmtMode, BondAdInfo, BondAdSelect, BondAllPortActive,
        BondArpAllTargets, BondArpValidate, BondFailOverMac, BondLacpRate,
        BondMode, BondPortState, BondPrimaryReselect, BondXmitHashPolicy,
        BridgeBooleanOptionFlags, BridgeBooleanOptions, BridgeId,
        BridgeIdBuffer, BridgeMulticastRouterType, BridgePortState,
        BridgeQuerierState, BridgeStpMode, BridgeStpState, CanBerrCounter,
        CanBitTiming, CanBitTimingConst, CanClock, CanCtrlMode,
        CanCtrlModeFlags, CanTdc, ChurnState, ErSpanDir, GeneveDf,
        GreEncapFlags, GreEncapType, GreIOFlags, GtpRole, HsrProtocol, InfoAmt,
        InfoBareUdp, InfoBatAdv, InfoBond, InfoBondPort, InfoBridge,
        InfoBridgePort, InfoCan, InfoData, InfoDsa, InfoGeneve, InfoGre,
        InfoGre6, InfoGtp, InfoHsr, InfoIpTunnel, InfoIpVlan, InfoIpVtap,
        InfoIpoib, InfoKind, InfoMacSec, InfoMacVlan, InfoMacVtap, InfoNetkit,
        InfoPortData, InfoPortKind, InfoRmNet, InfoRmNetFlags, InfoTeamPort,
        InfoTun, InfoVeth, InfoVlan, InfoVrf, InfoVrfPort, InfoVti, InfoVxcan,
        InfoVxlan, InfoWireguard, InfoWwan, InfoXfrm, Ip6TunnelFlags,
        IpVlanFlags, IpVlanMode, IpVtapFlags, IpVtapMode, IpoibMode, LacpState,
        LinkInfo, LinkXstats, MacSecCipherId, MacSecOffload, MacSecValidate,
        MacVlanFlags, MacVlanMacAddressMode, MacVlanMode, MacVtapFlags,
        MacVtapMacAddressMode, MacVtapMode, MiiStatus, NetkitMode,
        NetkitPolicy, NetkitScrub, RmNetFlags, TunnelEncapFlags,
        TunnelEncapType, VlanFlags, VlanQosMapping, VxlanDf,
    },
    link_layer_type::LinkLayerType,
    link_mode::LinkMode,
    link_state::State,
    map::{Map, MapBuffer},
    message::LinkMessage,
    phys_id::LinkPhysId,
    prop_list::Prop,
    proto_info::{LinkProtoInfoBridge, LinkProtoInfoInet6},
    sriov::{
        LinkVfInfo, LinkVfPort, VfInfo, VfInfoBroadcast, VfInfoBroadcastBuffer,
        VfInfoGuid, VfInfoGuidBuffer, VfInfoLinkState, VfInfoLinkStateBuffer,
        VfInfoMac, VfInfoMacBuffer, VfInfoRate, VfInfoRateBuffer,
        VfInfoRssQueryEn, VfInfoRssQueryEnBuffer, VfInfoSpoofCheck,
        VfInfoSpoofCheckBuffer, VfInfoTrust, VfInfoTrustBuffer, VfInfoTxRate,
        VfInfoTxRateBuffer, VfInfoVlan, VfInfoVlanBuffer, VfLinkState, VfPort,
        VfStats, VfVlan, VfVlanInfo,
    },
    stats::{Stats, StatsBuffer},
    stats64::{Stats64, Stats64Buffer},
    vlan_protocol::VlanProtocol,
    wireless::WirelessEvent,
    xdp::{LinkXdp, XdpAttached},
};
