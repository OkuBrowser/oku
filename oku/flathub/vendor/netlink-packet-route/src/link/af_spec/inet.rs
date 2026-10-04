// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer,
    NlasIterator, Parseable, NLA_F_NESTED,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use super::super::buffer_tool::expand_buffer_if_small;

pub(crate) const IFLA_INET_CONF: u16 = 1;

// This number might change when kernel add more IPV4_DEV_CONF
const __IPV4_DEVCONF_MAX: usize = 34;
const IPV4_DEVCONF_MAX: usize = __IPV4_DEVCONF_MAX - 1;

// IPV4_DEVCONF_* enum values from linux/ip.h (1-based)
const IPV4_DEVCONF_FORWARDING: u16 = 1;
const IPV4_DEVCONF_MC_FORWARDING: u16 = 2;
const IPV4_DEVCONF_PROXY_ARP: u16 = 3;
const IPV4_DEVCONF_ACCEPT_REDIRECTS: u16 = 4;
const IPV4_DEVCONF_SECURE_REDIRECTS: u16 = 5;
const IPV4_DEVCONF_SEND_REDIRECTS: u16 = 6;
const IPV4_DEVCONF_SHARED_MEDIA: u16 = 7;
const IPV4_DEVCONF_RP_FILTER: u16 = 8;
const IPV4_DEVCONF_ACCEPT_SOURCE_ROUTE: u16 = 9;
const IPV4_DEVCONF_BOOTP_RELAY: u16 = 10;
const IPV4_DEVCONF_LOG_MARTIANS: u16 = 11;
const IPV4_DEVCONF_TAG: u16 = 12;
const IPV4_DEVCONF_ARPFILTER: u16 = 13;
const IPV4_DEVCONF_MEDIUM_ID: u16 = 14;
const IPV4_DEVCONF_NOXFRM: u16 = 15;
const IPV4_DEVCONF_NOPOLICY: u16 = 16;
const IPV4_DEVCONF_FORCE_IGMP_VERSION: u16 = 17;
const IPV4_DEVCONF_ARP_ANNOUNCE: u16 = 18;
const IPV4_DEVCONF_ARP_IGNORE: u16 = 19;
const IPV4_DEVCONF_PROMOTE_SECONDARIES: u16 = 20;
const IPV4_DEVCONF_ARP_ACCEPT: u16 = 21;
const IPV4_DEVCONF_ARP_NOTIFY: u16 = 22;
const IPV4_DEVCONF_ACCEPT_LOCAL: u16 = 23;
const IPV4_DEVCONF_SRC_VMARK: u16 = 24;
const IPV4_DEVCONF_PROXY_ARP_PVLAN: u16 = 25;
const IPV4_DEVCONF_ROUTE_LOCALNET: u16 = 26;
const IPV4_DEVCONF_IGMPV2_UNSOLICITED_REPORT_INTERVAL: u16 = 27;
const IPV4_DEVCONF_IGMPV3_UNSOLICITED_REPORT_INTERVAL: u16 = 28;
const IPV4_DEVCONF_IGNORE_ROUTES_WITH_LINKDOWN: u16 = 29;
const IPV4_DEVCONF_DROP_UNICAST_IN_L2_MULTICAST: u16 = 30;
const IPV4_DEVCONF_DROP_GRATUITOUS_ARP: u16 = 31;
const IPV4_DEVCONF_BC_FORWARDING: u16 = 32;
const IPV4_DEVCONF_ARP_EVICT_NOCARRIER: u16 = 33;

// ---- AfSpecInet (parse + flat-buffer emit, matches kernel dump format) ----

#[derive(Clone, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum AfSpecInet {
    /// This is used for parsing kernel dump
    DevConf(InetDevConf),
    /// This is used for sending request to kernel for making changes.
    DevConfRequest(InetDevConf),
    Other(DefaultNla),
}

pub(crate) struct VecAfSpecInet(pub(crate) Vec<AfSpecInet>);

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for VecAfSpecInet
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let mut nlas = vec![];
        let err = "Invalid AF_INET NLA for IFLA_AF_SPEC(AF_UNSPEC)";
        for nla in NlasIterator::new(buf.into_inner()) {
            let nla = nla.context(err)?;
            nlas.push(AfSpecInet::parse(&nla)?);
        }
        Ok(Self(nlas))
    }
}

impl Nla for AfSpecInet {
    fn value_len(&self) -> usize {
        match *self {
            Self::DevConf(ref c) => c.buffer_len(),
            Self::DevConfRequest(ref c) => devconf_nested_value_len(c),
            Self::Other(ref nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match *self {
            Self::DevConf(ref c) => c.emit(buffer),
            Self::DevConfRequest(ref c) => emit_devconf_nested(buffer, c),
            Self::Other(ref nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match *self {
            Self::DevConf(_) => IFLA_INET_CONF,
            Self::DevConfRequest(_) => IFLA_INET_CONF | NLA_F_NESTED,
            Self::Other(ref nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for AfSpecInet {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_INET_CONF => Self::DevConf(InetDevConf::parse(
                expand_buffer_if_small(
                    payload,
                    size_of::<InetDevConfBuffer>(),
                    "IFLA_INET_CONF",
                )
                .as_slice(),
            )?),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "Unknown NLA type {kind} for IFLA_AF_SPEC(inet)"
            ))?),
        })
    }
}

fn devconf_nested_value_len(conf: &InetDevConf) -> usize {
    let mut len = 0;
    let entries = devconf_entries(conf);
    for &(kind, val) in &entries {
        if val == 0 {
            continue;
        }
        let nla = DefaultNla::new(kind, (val as u32).to_ne_bytes().to_vec());
        len += nla.buffer_len();
    }
    len
}

fn emit_devconf_nested(buffer: &mut [u8], conf: &InetDevConf) {
    let entries = devconf_entries(conf);
    let mut offset = 0;
    for &(kind, val) in &entries {
        if val == 0 {
            continue;
        }
        let nla = DefaultNla::new(kind, (val as u32).to_ne_bytes().to_vec());
        nla.emit(&mut buffer[offset..offset + nla.buffer_len()]);
        offset += nla.buffer_len();
    }
}

fn devconf_entries(conf: &InetDevConf) -> [(u16, i32); IPV4_DEVCONF_MAX] {
    [
        (IPV4_DEVCONF_FORWARDING, conf.forwarding),
        (IPV4_DEVCONF_MC_FORWARDING, conf.mc_forwarding),
        (IPV4_DEVCONF_PROXY_ARP, conf.proxy_arp),
        (IPV4_DEVCONF_ACCEPT_REDIRECTS, conf.accept_redirects),
        (IPV4_DEVCONF_SECURE_REDIRECTS, conf.secure_redirects),
        (IPV4_DEVCONF_SEND_REDIRECTS, conf.send_redirects),
        (IPV4_DEVCONF_SHARED_MEDIA, conf.shared_media),
        (IPV4_DEVCONF_RP_FILTER, conf.rp_filter),
        (IPV4_DEVCONF_ACCEPT_SOURCE_ROUTE, conf.accept_source_route),
        (IPV4_DEVCONF_BOOTP_RELAY, conf.bootp_relay),
        (IPV4_DEVCONF_LOG_MARTIANS, conf.log_martians),
        (IPV4_DEVCONF_TAG, conf.tag),
        (IPV4_DEVCONF_ARPFILTER, conf.arpfilter),
        (IPV4_DEVCONF_MEDIUM_ID, conf.medium_id),
        (IPV4_DEVCONF_NOXFRM, conf.noxfrm),
        (IPV4_DEVCONF_NOPOLICY, conf.nopolicy),
        (IPV4_DEVCONF_FORCE_IGMP_VERSION, conf.force_igmp_version),
        (IPV4_DEVCONF_ARP_ANNOUNCE, conf.arp_announce),
        (IPV4_DEVCONF_ARP_IGNORE, conf.arp_ignore),
        (IPV4_DEVCONF_PROMOTE_SECONDARIES, conf.promote_secondaries),
        (IPV4_DEVCONF_ARP_ACCEPT, conf.arp_accept),
        (IPV4_DEVCONF_ARP_NOTIFY, conf.arp_notify),
        (IPV4_DEVCONF_ACCEPT_LOCAL, conf.accept_local),
        (IPV4_DEVCONF_SRC_VMARK, conf.src_vmark),
        (IPV4_DEVCONF_PROXY_ARP_PVLAN, conf.proxy_arp_pvlan),
        (IPV4_DEVCONF_ROUTE_LOCALNET, conf.route_localnet),
        (
            IPV4_DEVCONF_IGMPV2_UNSOLICITED_REPORT_INTERVAL,
            conf.igmpv2_unsolicited_report_interval,
        ),
        (
            IPV4_DEVCONF_IGMPV3_UNSOLICITED_REPORT_INTERVAL,
            conf.igmpv3_unsolicited_report_interval,
        ),
        (
            IPV4_DEVCONF_IGNORE_ROUTES_WITH_LINKDOWN,
            conf.ignore_routes_with_linkdown,
        ),
        (
            IPV4_DEVCONF_DROP_UNICAST_IN_L2_MULTICAST,
            conf.drop_unicast_in_l2_multicast,
        ),
        (IPV4_DEVCONF_DROP_GRATUITOUS_ARP, conf.drop_gratuitous_arp),
        (IPV4_DEVCONF_BC_FORWARDING, conf.bc_forwarding),
        (IPV4_DEVCONF_ARP_EVICT_NOCARRIER, conf.arp_evict_nocarrier),
    ]
}

// ---- InetDevConf (flat buffer, matches kernel dump format) ----

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
    Unaligned,
)]
#[repr(C, packed)]
pub struct InetDevConfBuffer {
    forwarding: i32,
    mc_forwarding: i32,
    proxy_arp: i32,
    accept_redirects: i32,
    secure_redirects: i32,
    send_redirects: i32,
    shared_media: i32,
    rp_filter: i32,
    accept_source_route: i32,
    bootp_relay: i32,
    log_martians: i32,
    tag: i32,
    arpfilter: i32,
    medium_id: i32,
    noxfrm: i32,
    nopolicy: i32,
    force_igmp_version: i32,
    arp_announce: i32,
    arp_ignore: i32,
    promote_secondaries: i32,
    arp_accept: i32,
    arp_notify: i32,
    accept_local: i32,
    src_vmark: i32,
    proxy_arp_pvlan: i32,
    route_localnet: i32,
    igmpv2_unsolicited_report_interval: i32,
    igmpv3_unsolicited_report_interval: i32,
    ignore_routes_with_linkdown: i32,
    drop_unicast_in_l2_multicast: i32,
    drop_gratuitous_arp: i32,
    bc_forwarding: i32,
    arp_evict_nocarrier: i32,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
#[non_exhaustive]
pub struct InetDevConf {
    pub forwarding: i32,
    pub mc_forwarding: i32,
    pub proxy_arp: i32,
    pub accept_redirects: i32,
    pub secure_redirects: i32,
    pub send_redirects: i32,
    pub shared_media: i32,
    pub rp_filter: i32,
    pub accept_source_route: i32,
    pub bootp_relay: i32,
    pub log_martians: i32,
    pub tag: i32,
    pub arpfilter: i32,
    pub medium_id: i32,
    pub noxfrm: i32,
    pub nopolicy: i32,
    pub force_igmp_version: i32,
    pub arp_announce: i32,
    pub arp_ignore: i32,
    pub promote_secondaries: i32,
    pub arp_accept: i32,
    pub arp_notify: i32,
    pub accept_local: i32,
    pub src_vmark: i32,
    pub proxy_arp_pvlan: i32,
    pub route_localnet: i32,
    pub igmpv2_unsolicited_report_interval: i32,
    pub igmpv3_unsolicited_report_interval: i32,
    pub ignore_routes_with_linkdown: i32,
    pub drop_unicast_in_l2_multicast: i32,
    pub drop_gratuitous_arp: i32,
    pub bc_forwarding: i32,
    pub arp_evict_nocarrier: i32,
}

impl InetDevConf {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            InetDevConfBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<InetDevConfBuffer>(),
                )
            })?;
        Ok(Self {
            forwarding: raw.forwarding,
            mc_forwarding: raw.mc_forwarding,
            proxy_arp: raw.proxy_arp,
            accept_redirects: raw.accept_redirects,
            secure_redirects: raw.secure_redirects,
            send_redirects: raw.send_redirects,
            shared_media: raw.shared_media,
            rp_filter: raw.rp_filter,
            accept_source_route: raw.accept_source_route,
            bootp_relay: raw.bootp_relay,
            log_martians: raw.log_martians,
            tag: raw.tag,
            arpfilter: raw.arpfilter,
            medium_id: raw.medium_id,
            noxfrm: raw.noxfrm,
            nopolicy: raw.nopolicy,
            force_igmp_version: raw.force_igmp_version,
            arp_announce: raw.arp_announce,
            arp_ignore: raw.arp_ignore,
            promote_secondaries: raw.promote_secondaries,
            arp_accept: raw.arp_accept,
            arp_notify: raw.arp_notify,
            accept_local: raw.accept_local,
            src_vmark: raw.src_vmark,
            proxy_arp_pvlan: raw.proxy_arp_pvlan,
            route_localnet: raw.route_localnet,
            igmpv2_unsolicited_report_interval: raw
                .igmpv2_unsolicited_report_interval,
            igmpv3_unsolicited_report_interval: raw
                .igmpv3_unsolicited_report_interval,
            ignore_routes_with_linkdown: raw.ignore_routes_with_linkdown,
            drop_unicast_in_l2_multicast: raw.drop_unicast_in_l2_multicast,
            drop_gratuitous_arp: raw.drop_gratuitous_arp,
            bc_forwarding: raw.bc_forwarding,
            arp_evict_nocarrier: raw.arp_evict_nocarrier,
        })
    }
}

impl From<&InetDevConf> for InetDevConfBuffer {
    fn from(value: &InetDevConf) -> Self {
        Self {
            forwarding: value.forwarding,
            mc_forwarding: value.mc_forwarding,
            proxy_arp: value.proxy_arp,
            accept_redirects: value.accept_redirects,
            secure_redirects: value.secure_redirects,
            send_redirects: value.send_redirects,
            shared_media: value.shared_media,
            rp_filter: value.rp_filter,
            accept_source_route: value.accept_source_route,
            bootp_relay: value.bootp_relay,
            log_martians: value.log_martians,
            tag: value.tag,
            arpfilter: value.arpfilter,
            medium_id: value.medium_id,
            noxfrm: value.noxfrm,
            nopolicy: value.nopolicy,
            force_igmp_version: value.force_igmp_version,
            arp_announce: value.arp_announce,
            arp_ignore: value.arp_ignore,
            promote_secondaries: value.promote_secondaries,
            arp_accept: value.arp_accept,
            arp_notify: value.arp_notify,
            accept_local: value.accept_local,
            src_vmark: value.src_vmark,
            proxy_arp_pvlan: value.proxy_arp_pvlan,
            route_localnet: value.route_localnet,
            igmpv2_unsolicited_report_interval: value
                .igmpv2_unsolicited_report_interval,
            igmpv3_unsolicited_report_interval: value
                .igmpv3_unsolicited_report_interval,
            ignore_routes_with_linkdown: value.ignore_routes_with_linkdown,
            drop_unicast_in_l2_multicast: value.drop_unicast_in_l2_multicast,
            drop_gratuitous_arp: value.drop_gratuitous_arp,
            bc_forwarding: value.bc_forwarding,
            arp_evict_nocarrier: value.arp_evict_nocarrier,
        }
    }
}

impl Emitable for InetDevConf {
    fn buffer_len(&self) -> usize {
        size_of::<InetDevConfBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = InetDevConfBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
