// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    DecodeError, Emitable, ErrorContext, Nla, NlaBuffer, NlasIterator,
    Parseable, ParseableParametrized,
};

use super::super::{
    InfoBond, InfoBridge, InfoGeneve, InfoGre, InfoGre6, InfoGtp, InfoHsr,
    InfoIpTunnel, InfoIpVlan, InfoIpVtap, InfoIpoib, InfoKind, InfoMacSec,
    InfoMacVlan, InfoMacVtap, InfoNetkit, InfoTun, InfoVeth, InfoVlan, InfoVrf,
    InfoVti, InfoVxcan, InfoVxlan, InfoXfrm,
};

const IFLA_INFO_DATA: u16 = 2;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoData {
    Bridge(Vec<InfoBridge>),
    Tun(Vec<InfoTun>),
    Vlan(Vec<InfoVlan>),
    Veth(InfoVeth),
    Vxlan(Vec<InfoVxlan>),
    Bond(Vec<InfoBond>),
    IpVlan(Vec<InfoIpVlan>),
    IpVtap(Vec<InfoIpVtap>),
    MacVlan(Vec<InfoMacVlan>),
    MacVtap(Vec<InfoMacVtap>),
    GreTap(Vec<InfoGre>),
    GreTap6(Vec<InfoGre6>),
    GreTun(Vec<InfoGre>),
    GreTun6(Vec<InfoGre6>),
    Vti(Vec<InfoVti>),
    Vrf(Vec<InfoVrf>),
    Gtp(Vec<InfoGtp>),
    Ipoib(Vec<InfoIpoib>),
    Xfrm(Vec<InfoXfrm>),
    MacSec(Vec<InfoMacSec>),
    Hsr(Vec<InfoHsr>),
    Geneve(Vec<InfoGeneve>),
    IpTunnel(Vec<InfoIpTunnel>),
    Netkit(Vec<InfoNetkit>),
    Vxcan(InfoVxcan),
    Other(Vec<u8>),
}

impl Nla for InfoData {
    fn value_len(&self) -> usize {
        match self {
            Self::Bond(nlas) => nlas.as_slice().buffer_len(),
            Self::Bridge(nlas) => nlas.as_slice().buffer_len(),
            Self::Vlan(nlas) => nlas.as_slice().buffer_len(),
            Self::Veth(msg) => msg.buffer_len(),
            Self::IpVlan(nlas) => nlas.as_slice().buffer_len(),
            Self::IpVtap(nlas) => nlas.as_slice().buffer_len(),
            Self::Ipoib(nlas) => nlas.as_slice().buffer_len(),
            Self::MacVlan(nlas) => nlas.as_slice().buffer_len(),
            Self::MacVtap(nlas) => nlas.as_slice().buffer_len(),
            Self::Vrf(nlas) => nlas.as_slice().buffer_len(),
            Self::Vxlan(nlas) => nlas.as_slice().buffer_len(),
            Self::Xfrm(nlas) => nlas.as_slice().buffer_len(),
            Self::MacSec(nlas) => nlas.as_slice().buffer_len(),
            Self::Hsr(nlas) => nlas.as_slice().buffer_len(),
            Self::Tun(nlas) => nlas.as_slice().buffer_len(),
            Self::GreTap(nlas) => nlas.as_slice().buffer_len(),
            Self::GreTap6(nlas) => nlas.as_slice().buffer_len(),
            Self::GreTun(nlas) => nlas.as_slice().buffer_len(),
            Self::GreTun6(nlas) => nlas.as_slice().buffer_len(),
            Self::Vti(nlas) => nlas.as_slice().buffer_len(),
            Self::Gtp(nlas) => nlas.as_slice().buffer_len(),
            Self::Geneve(nlas) => nlas.as_slice().buffer_len(),
            Self::IpTunnel(nlas) => nlas.as_slice().buffer_len(),
            Self::Netkit(nlas) => nlas.as_slice().buffer_len(),
            Self::Vxcan(nlas) => nlas.buffer_len(),
            Self::Other(v) => v.len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Bond(nlas) => nlas.as_slice().emit(buffer),
            Self::Bridge(nlas) => nlas.as_slice().emit(buffer),
            Self::Vlan(nlas) => nlas.as_slice().emit(buffer),
            Self::Veth(msg) => msg.emit(buffer),
            Self::IpVlan(nlas) => nlas.as_slice().emit(buffer),
            Self::IpVtap(nlas) => nlas.as_slice().emit(buffer),
            Self::Ipoib(nlas) => nlas.as_slice().emit(buffer),
            Self::MacVlan(nlas) => nlas.as_slice().emit(buffer),
            Self::MacVtap(nlas) => nlas.as_slice().emit(buffer),
            Self::Vrf(nlas) => nlas.as_slice().emit(buffer),
            Self::Vxlan(nlas) => nlas.as_slice().emit(buffer),
            Self::Xfrm(nlas) => nlas.as_slice().emit(buffer),
            Self::MacSec(nlas) => nlas.as_slice().emit(buffer),
            Self::Hsr(nlas) => nlas.as_slice().emit(buffer),
            Self::Tun(nlas) => nlas.as_slice().emit(buffer),
            Self::GreTap(nlas) => nlas.as_slice().emit(buffer),
            Self::GreTap6(nlas) => nlas.as_slice().emit(buffer),
            Self::GreTun(nlas) => nlas.as_slice().emit(buffer),
            Self::GreTun6(nlas) => nlas.as_slice().emit(buffer),
            Self::Vti(nlas) => nlas.as_slice().emit(buffer),
            Self::Gtp(nlas) => nlas.as_slice().emit(buffer),
            Self::Geneve(nlas) => nlas.as_slice().emit(buffer),
            Self::IpTunnel(nlas) => nlas.as_slice().emit(buffer),
            Self::Netkit(nlas) => nlas.as_slice().emit(buffer),
            Self::Vxcan(msg) => msg.emit(buffer),
            Self::Other(v) => buffer.copy_from_slice(v.as_slice()),
        }
    }

    fn kind(&self) -> u16 {
        IFLA_INFO_DATA
    }
}

impl InfoData {
    pub(crate) fn parse_with_param(
        payload: &[u8],
        kind: &InfoKind,
    ) -> Result<InfoData, DecodeError> {
        Ok(match kind {
            InfoKind::Bridge => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoBridge::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Bridge(v)
            }
            InfoKind::Vlan => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoVlan::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Vlan(v)
            }
            InfoKind::Tun => {
                let mut nlas = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoTun::parse(nla)?;
                    nlas.push(parsed);
                }
                InfoData::Tun(nlas)
            }
            InfoKind::Veth => {
                let nla_buf = NlaBuffer::new_checked(&payload).context(
                    format!("invalid IFLA_INFO_DATA for {kind} {payload:?}"),
                )?;
                let parsed = InfoVeth::parse(&nla_buf)?;
                InfoData::Veth(parsed)
            }
            InfoKind::Vxlan => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoVxlan::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Vxlan(v)
            }
            InfoKind::Bond => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoBond::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Bond(v)
            }
            InfoKind::IpVlan => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoIpVlan::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::IpVlan(v)
            }
            InfoKind::IpVtap => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoIpVtap::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::IpVtap(v)
            }
            InfoKind::MacVlan => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoMacVlan::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::MacVlan(v)
            }
            InfoKind::MacVtap => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoMacVtap::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::MacVtap(v)
            }
            InfoKind::GreTap => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoGre::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::GreTap(v)
            }
            InfoKind::GreTap6 => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoGre6::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::GreTap6(v)
            }
            InfoKind::GreTun => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoGre::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::GreTun(v)
            }
            InfoKind::GreTun6 => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoGre6::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::GreTun6(v)
            }
            InfoKind::Vti => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoVti::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Vti(v)
            }
            InfoKind::Vrf => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoVrf::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Vrf(v)
            }
            InfoKind::Gtp => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoGtp::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Gtp(v)
            }
            InfoKind::Ipoib => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoIpoib::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Ipoib(v)
            }
            InfoKind::Xfrm => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoXfrm::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Xfrm(v)
            }
            InfoKind::MacSec => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoMacSec::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::MacSec(v)
            }
            InfoKind::Hsr => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoHsr::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Hsr(v)
            }
            InfoKind::IpIp | InfoKind::Ip6Tnl | InfoKind::SitTun => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed =
                        InfoIpTunnel::parse_with_param(nla, kind.clone())?;
                    v.push(parsed);
                }
                InfoData::IpTunnel(v)
            }
            InfoKind::Geneve => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoGeneve::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Geneve(v)
            }
            InfoKind::Netkit => {
                let mut v = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(format!(
                        "invalid IFLA_INFO_DATA for {kind} {payload:?}"
                    ))?;
                    let parsed = InfoNetkit::parse(nla)?;
                    v.push(parsed);
                }
                InfoData::Netkit(v)
            }
            InfoKind::Vxcan => {
                let nla_buf = NlaBuffer::new_checked(&payload).context(
                    format!("invalid IFLA_INFO_DATA for {kind} {payload:?}"),
                )?;
                let parsed = InfoVxcan::parse(&nla_buf)?;
                InfoData::Vxcan(parsed)
            }
            _ => InfoData::Other(payload.to_vec()),
        })
    }
}
