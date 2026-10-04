// SPDX-License-Identifier: MIT

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use netlink_packet_core::{
    emit_u16, emit_u16_be, emit_u32, parse_u16, parse_u16_be, parse_u32,
    parse_u8, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer, Parseable,
    ParseableParametrized,
};

use crate::ip::{parse_ip_addr, IpProtocol};

const IFLA_IPTUN_LINK: u16 = 1;
const IFLA_IPTUN_LOCAL: u16 = 2;
const IFLA_IPTUN_REMOTE: u16 = 3;
const IFLA_IPTUN_TTL: u16 = 4;
const IFLA_IPTUN_TOS: u16 = 5;
const IFLA_IPTUN_ENCAP_LIMIT: u16 = 6;
const IFLA_IPTUN_FLOWINFO: u16 = 7;
const IFLA_IPTUN_FLAGS: u16 = 8;
const IFLA_IPTUN_PROTO: u16 = 9;
const IFLA_IPTUN_PMTUDISC: u16 = 10;
const IFLA_IPTUN_6RD_PREFIX: u16 = 11;
const IFLA_IPTUN_6RD_RELAY_PREFIX: u16 = 12;
const IFLA_IPTUN_6RD_PREFIXLEN: u16 = 13;
const IFLA_IPTUN_6RD_RELAY_PREFIXLEN: u16 = 14;
const IFLA_IPTUN_ENCAP_TYPE: u16 = 15;
const IFLA_IPTUN_ENCAP_FLAGS: u16 = 16;
const IFLA_IPTUN_ENCAP_SPORT: u16 = 17;
const IFLA_IPTUN_ENCAP_DPORT: u16 = 18;
const IFLA_IPTUN_COLLECT_METADATA: u16 = 19;
const IFLA_IPTUN_FWMARK: u16 = 20;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoIpTunnel {
    Link(u32),
    Local(IpAddr),
    Remote(IpAddr),
    Ttl(u8),
    Tos(u8),
    EncapLimit(u8),
    FlowInfo(u32),
    Ipv6SitFlags(u16),
    Ipv4Flags(u16),
    Ipv6Flags(Ip6TunnelFlags),
    Protocol(IpProtocol),
    PMtuDisc(bool),
    Ipv6RdPrefix(Ipv6Addr),
    Ipv6RdRelayPrefix(Ipv4Addr),
    Ipv6RdPrefixLen(u16),
    Ipv6RdRelayPrefixLen(u16),
    EncapType(TunnelEncapType),
    EncapFlags(TunnelEncapFlags),
    EncapSPort(u16),
    EncapDPort(u16),
    CollectMetadata(bool),
    FwMark(u32),
    Other(DefaultNla),
}

impl Nla for InfoIpTunnel {
    fn value_len(&self) -> usize {
        use self::InfoIpTunnel::*;
        match self {
            Ipv6RdPrefix(_) => 16,
            Ipv6RdRelayPrefix(_) => 4,
            Local(value) | Remote(value) => match value {
                IpAddr::V4(_) => 4,
                IpAddr::V6(_) => 16,
            },
            Link(_) | FwMark(_) | FlowInfo(_) | Ipv6Flags(_) => 4,
            Ipv6SitFlags(_)
            | Ipv4Flags(_)
            | EncapType(_)
            | EncapFlags(_)
            | EncapSPort(_)
            | EncapDPort(_)
            | Ipv6RdPrefixLen(_)
            | Ipv6RdRelayPrefixLen(_) => 2,
            Ttl(_) | Tos(_) | Protocol(_) | PMtuDisc(_)
            | CollectMetadata(_) | EncapLimit(_) => 1,
            Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        use self::InfoIpTunnel::*;
        match self {
            Ipv6RdPrefix(value) => buffer.copy_from_slice(&value.octets()),
            Ipv6RdRelayPrefix(value) => buffer.copy_from_slice(&value.octets()),
            Link(value) | FwMark(value) | FlowInfo(value) => {
                emit_u32(buffer, *value).unwrap()
            }
            Ipv6Flags(f) => emit_u32(buffer, f.bits()).unwrap(),
            Ipv6SitFlags(val) | Ipv4Flags(val) => {
                emit_u16_be(buffer, *val).unwrap()
            }
            Local(value) | Remote(value) => match value {
                IpAddr::V4(ipv4) => buffer.copy_from_slice(&ipv4.octets()),
                IpAddr::V6(ipv6) => buffer.copy_from_slice(&ipv6.octets()),
            },
            EncapType(value) => emit_u16(buffer, (*value).into()).unwrap(),
            EncapFlags(f) => emit_u16(buffer, f.bits()).unwrap(),
            EncapSPort(value) | EncapDPort(value) => {
                emit_u16_be(buffer, *value).unwrap()
            }
            Ipv6RdPrefixLen(value) | Ipv6RdRelayPrefixLen(value) => {
                emit_u16(buffer, *value).unwrap()
            }
            Protocol(value) => buffer[0] = u8::from(*value),
            Ttl(value) | Tos(value) | EncapLimit(value) => buffer[0] = *value,
            PMtuDisc(value) | CollectMetadata(value) => {
                buffer[0] = if *value { 1 } else { 0 }
            }
            Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        use self::InfoIpTunnel::*;
        match self {
            Link(_) => IFLA_IPTUN_LINK,
            Local(_) => IFLA_IPTUN_LOCAL,
            Remote(_) => IFLA_IPTUN_REMOTE,
            Ttl(_) => IFLA_IPTUN_TTL,
            Tos(_) => IFLA_IPTUN_TOS,
            EncapLimit(_) => IFLA_IPTUN_ENCAP_LIMIT,
            FlowInfo(_) => IFLA_IPTUN_FLOWINFO,
            Ipv6SitFlags(_) | Ipv4Flags(_) | Ipv6Flags(_) => IFLA_IPTUN_FLAGS,
            Protocol(_) => IFLA_IPTUN_PROTO,
            PMtuDisc(_) => IFLA_IPTUN_PMTUDISC,
            Ipv6RdPrefix(_) => IFLA_IPTUN_6RD_PREFIX,
            Ipv6RdRelayPrefix(_) => IFLA_IPTUN_6RD_RELAY_PREFIX,
            Ipv6RdPrefixLen(_) => IFLA_IPTUN_6RD_PREFIXLEN,
            Ipv6RdRelayPrefixLen(_) => IFLA_IPTUN_6RD_RELAY_PREFIXLEN,
            EncapType(_) => IFLA_IPTUN_ENCAP_TYPE,
            EncapFlags(_) => IFLA_IPTUN_ENCAP_FLAGS,
            EncapSPort(_) => IFLA_IPTUN_ENCAP_SPORT,
            EncapDPort(_) => IFLA_IPTUN_ENCAP_DPORT,
            CollectMetadata(_) => IFLA_IPTUN_COLLECT_METADATA,
            FwMark(_) => IFLA_IPTUN_FWMARK,
            Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized>
    ParseableParametrized<NlaBuffer<&'a T>, super::InfoKind> for InfoIpTunnel
{
    fn parse_with_param(
        buf: &NlaBuffer<&'a T>,
        kind: super::InfoKind,
    ) -> Result<Self, DecodeError> {
        use self::InfoIpTunnel::*;
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_IPTUN_LINK => Link(
                parse_u32(payload).context("invalid IFLA_IPTUN_LINK value")?,
            ),
            IFLA_IPTUN_LOCAL => {
                let ip = parse_ip_addr(payload)
                    .context("invalid IFLA_IPTUN_LOCAL")?;
                Self::Local(ip)
            }
            IFLA_IPTUN_REMOTE => {
                let ip = parse_ip_addr(payload)
                    .context("invalid IFLA_IPTUN_REMOTE")?;
                Self::Remote(ip)
            }
            IFLA_IPTUN_TTL => {
                Ttl(parse_u8(payload)
                    .context("invalid IFLA_IPTUN_TTL value")?)
            }
            IFLA_IPTUN_TOS => {
                Tos(parse_u8(payload)
                    .context("invalid IFLA_IPTUN_TOS value")?)
            }
            IFLA_IPTUN_ENCAP_LIMIT => EncapLimit(
                parse_u8(payload)
                    .context("invalid IFLA_IPTUN_ENCAP_LIMIT value")?,
            ),
            IFLA_IPTUN_FLOWINFO => FlowInfo(
                parse_u32(payload)
                    .context("invalid IFLA_IPTUN_FLOWINFO value")?,
            ),
            IFLA_IPTUN_FLAGS => match kind {
                super::InfoKind::IpIp => InfoIpTunnel::Ipv4Flags(
                    parse_u16_be(payload)
                        .context("invalid IFLA_IPTUN_FLAGS for IPIP")?,
                ),
                super::InfoKind::SitTun => InfoIpTunnel::Ipv6SitFlags(
                    parse_u16_be(payload)
                        .context("invalid IFLA_IPTUN_FLAGS for SIT")?,
                ),
                super::InfoKind::Ip6Tnl => {
                    InfoIpTunnel::Ipv6Flags(Ip6TunnelFlags::from_bits_retain(
                        parse_u32(payload)
                            .context("invalid IFLA_IPTUN_FLAGS for IP6")?,
                    ))
                }
                _ => {
                    return Err(DecodeError::from(format!(
                        "unsupported InfoKind for IFLA_IPTUN_FLAGS: {kind:?}"
                    )));
                }
            },
            IFLA_IPTUN_PROTO => Protocol(IpProtocol::from(
                parse_u8(payload).context("invalid IFLA_IPTUN_PROTO value")?,
            )),
            IFLA_IPTUN_PMTUDISC => PMtuDisc(
                parse_u8(payload)
                    .context("invalid IFLA_IPTUN_PMTUDISC value")?
                    > 0,
            ),
            IFLA_IPTUN_6RD_PREFIX => {
                if payload.len() == 16 {
                    let mut data = [0u8; 16];
                    data.copy_from_slice(&payload[0..16]);
                    Self::Ipv6RdPrefix(Ipv6Addr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_IPTUN_6RD_PREFIX, got unexpected length \
                         of IPv6 address payload {payload:?}"
                    )));
                }
            }
            IFLA_IPTUN_6RD_RELAY_PREFIX => {
                if payload.len() == 4 {
                    let mut data = [0u8; 4];
                    data.copy_from_slice(&payload[0..4]);
                    Self::Ipv6RdRelayPrefix(Ipv4Addr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_IPTUN_6RD_RELAY_PREFIX, got unexpected \
                         length of IPv4 address payload {payload:?}"
                    )));
                }
            }
            IFLA_IPTUN_6RD_PREFIXLEN => Ipv6RdPrefixLen(
                parse_u16(payload)
                    .context("invalid IFLA_IPTUN_6RD_PREFIXLEN value")?,
            ),
            IFLA_IPTUN_6RD_RELAY_PREFIXLEN => Ipv6RdRelayPrefixLen(
                parse_u16(payload)
                    .context("invalid IFLA_IPTUN_6RD_RELAY_PREFIXLEN value")?,
            ),
            IFLA_IPTUN_ENCAP_TYPE => EncapType(
                parse_u16(payload)
                    .context("invalid IFLA_IPTUN_ENCAP_TYPE value")?
                    .into(),
            ),
            IFLA_IPTUN_ENCAP_FLAGS => {
                EncapFlags(TunnelEncapFlags::from_bits_retain(
                    parse_u16(payload)
                        .context("failed to parse IFLA_IPTUN_ENCAP_FLAGS")?,
                ))
            }
            IFLA_IPTUN_ENCAP_SPORT => EncapSPort(
                parse_u16_be(payload)
                    .context("invalid IFLA_IPTUN_ENCAP_SPORT value")?,
            ),
            IFLA_IPTUN_ENCAP_DPORT => EncapDPort(
                parse_u16_be(payload)
                    .context("invalid IFLA_IPTUN_ENCAP_DPORT value")?,
            ),
            IFLA_IPTUN_COLLECT_METADATA => CollectMetadata(
                parse_u8(payload)
                    .context("invalid IFLA_IPTUN_COLLECT_METADATA value")?
                    > 0,
            ),
            IFLA_IPTUN_FWMARK => FwMark(
                parse_u32(payload)
                    .context("invalid IFLA_IPTUN_FWMARK value")?,
            ),
            kind => Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind}"))?,
            ),
        })
    }
}

const TUNNEL_ENCAP_NONE: u16 = 0;
const TUNNEL_ENCAP_FOU: u16 = 1;
const TUNNEL_ENCAP_GUE: u16 = 2;
const TUNNEL_ENCAP_MPLS: u16 = 3;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
#[repr(u16)]
pub enum TunnelEncapType {
    None,
    Fou,
    Gue,
    Mpls,
    Other(u16),
}

impl From<u16> for TunnelEncapType {
    fn from(d: u16) -> Self {
        match d {
            TUNNEL_ENCAP_NONE => Self::None,
            TUNNEL_ENCAP_FOU => Self::Fou,
            TUNNEL_ENCAP_GUE => Self::Gue,
            TUNNEL_ENCAP_MPLS => Self::Mpls,
            _ => Self::Other(d),
        }
    }
}

impl From<TunnelEncapType> for u16 {
    fn from(d: TunnelEncapType) -> Self {
        match d {
            TunnelEncapType::None => TUNNEL_ENCAP_NONE,
            TunnelEncapType::Fou => TUNNEL_ENCAP_FOU,
            TunnelEncapType::Gue => TUNNEL_ENCAP_GUE,
            TunnelEncapType::Mpls => TUNNEL_ENCAP_MPLS,
            TunnelEncapType::Other(value) => value,
        }
    }
}

impl std::fmt::Display for TunnelEncapType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Fou => write!(f, "fou"),
            Self::Gue => write!(f, "gue"),
            Self::Mpls => write!(f, "mpls"),
            Self::Other(d) => write!(f, "{d}"),
        }
    }
}

const TUNNEL_ENCAP_FLAG_CSUM: u16 = 1 << 0;
const TUNNEL_ENCAP_FLAG_CSUM6: u16 = 1 << 1;
const TUNNEL_ENCAP_FLAG_REMCSUM: u16 = 1 << 2;

bitflags! {
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TunnelEncapFlags: u16 {
        const CSum = TUNNEL_ENCAP_FLAG_CSUM;
        const CSum6 = TUNNEL_ENCAP_FLAG_CSUM6;
        const RemCSum = TUNNEL_ENCAP_FLAG_REMCSUM;
        const _ = !0;
    }
}

const IP6_TNL_F_IGN_ENCAP_LIMIT: u32 = 0x1;
const IP6_TNL_F_USE_ORIG_TCLASS: u32 = 0x2;
const IP6_TNL_F_USE_ORIG_FLOWLABEL: u32 = 0x4;
const IP6_TNL_F_MIP6_DEV: u32 = 0x8;
const IP6_TNL_F_RCV_DSCP_COPY: u32 = 0x10;
const IP6_TNL_F_USE_ORIG_FWMARK: u32 = 0x20;
const IP6_TNL_F_ALLOW_LOCAL_REMOTE: u32 = 0x40;

const IP6_TNL_F_CAP_XMIT: u32 = 0x10000;
const IP6_TNL_F_CAP_RCV: u32 = 0x20000;
const IP6_TNL_F_CAP_PER_PACKET: u32 = 0x40000;

bitflags! {
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Ip6TunnelFlags: u32 {
        const IgnEncapLimit = IP6_TNL_F_IGN_ENCAP_LIMIT;
        const UseOrigTclass = IP6_TNL_F_USE_ORIG_TCLASS;
        const UseOrigFlowlabel = IP6_TNL_F_USE_ORIG_FLOWLABEL;
        const Mip6Dev = IP6_TNL_F_MIP6_DEV;
        const RcvDscpCopy = IP6_TNL_F_RCV_DSCP_COPY;
        const UseOrigFwMark = IP6_TNL_F_USE_ORIG_FWMARK;
        const AllowLocalRemote = IP6_TNL_F_ALLOW_LOCAL_REMOTE;
        const CapXmit = IP6_TNL_F_CAP_XMIT;
        const CapRcv = IP6_TNL_F_CAP_RCV;
        const CapPerPacket = IP6_TNL_F_CAP_PER_PACKET;
        const _ = !0;
    }
}
