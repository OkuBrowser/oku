// SPDX-License-Identifier: MIT

mod broadcast;
mod guid;
mod link_state;
mod mac;
mod rate;
mod rss_query;
mod spoofchk;
mod stats;
mod trust;
mod tx_rate;
mod vf_list;
mod vf_port;
mod vf_vlan;
mod vlan;

pub use self::{
    broadcast::{VfInfoBroadcast, VfInfoBroadcastBuffer},
    guid::{VfInfoGuid, VfInfoGuidBuffer},
    link_state::{VfInfoLinkState, VfInfoLinkStateBuffer, VfLinkState},
    mac::{VfInfoMac, VfInfoMacBuffer},
    rate::{VfInfoRate, VfInfoRateBuffer},
    rss_query::{VfInfoRssQueryEn, VfInfoRssQueryEnBuffer},
    spoofchk::{VfInfoSpoofCheck, VfInfoSpoofCheckBuffer},
    stats::VfStats,
    trust::{VfInfoTrust, VfInfoTrustBuffer},
    tx_rate::{VfInfoTxRate, VfInfoTxRateBuffer},
    vf_list::{LinkVfInfo, VfInfo},
    vf_port::{LinkVfPort, VfPort},
    vf_vlan::{VfVlan, VfVlanInfo},
    vlan::{VfInfoVlan, VfInfoVlanBuffer},
};
pub(crate) use self::{vf_list::VecLinkVfInfo, vf_port::VecLinkVfPort};
