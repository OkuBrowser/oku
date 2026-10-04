// SPDX-License-Identifier: MIT

use std::{fmt::Debug, mem::size_of, net::Ipv6Addr};

use netlink_packet_core::{
    emit_u16_be, emit_u64_be, parse_u16_be, parse_u64_be, parse_u8,
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer,
    NlasIterator, Parseable, ParseableParametrized,
};

use super::{RouteMplsIpTunnel, RouteSeg6IpTunnel};
use crate::ip::parse_ipv6_addr;

const LWTUNNEL_ENCAP_NONE: u16 = 0;
const LWTUNNEL_ENCAP_MPLS: u16 = 1;
const LWTUNNEL_ENCAP_IP: u16 = 2;
const LWTUNNEL_ENCAP_ILA: u16 = 3;
const LWTUNNEL_ENCAP_IP6: u16 = 4;
const LWTUNNEL_ENCAP_SEG6: u16 = 5;
const LWTUNNEL_ENCAP_BPF: u16 = 6;
const LWTUNNEL_ENCAP_SEG6_LOCAL: u16 = 7;
const LWTUNNEL_ENCAP_RPL: u16 = 8;
const LWTUNNEL_ENCAP_IOAM6: u16 = 9;
const LWTUNNEL_ENCAP_XFRM: u16 = 10;

const LWTUNNEL_IP6_UNSPEC: u16 = 0;
const LWTUNNEL_IP6_ID: u16 = 1;
const LWTUNNEL_IP6_DST: u16 = 2;
const LWTUNNEL_IP6_SRC: u16 = 3;
const LWTUNNEL_IP6_HOPLIMIT: u16 = 4;
const LWTUNNEL_IP6_TC: u16 = 5;
const LWTUNNEL_IP6_FLAGS: u16 = 6;
//const LWTUNNEL_IP6_PAD: u16 = 7;
//const LWTUNNEL_IP6_OPTS: u16 = 8;

const IP_TUNNEL_CSUM_BIT: u16 = 1;
const IP_TUNNEL_KEY_BIT: u16 = 4;
const IP_TUNNEL_SEQ_BIT: u16 = 8;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[non_exhaustive]
pub enum RouteLwEnCapType {
    #[default]
    None,
    Mpls,
    Ip,
    Ila,
    Ip6,
    Seg6,
    Bpf,
    Seg6Local,
    Rpl,
    Ioam6,
    Xfrm,
    Other(u16),
}

impl From<u16> for RouteLwEnCapType {
    fn from(d: u16) -> Self {
        match d {
            LWTUNNEL_ENCAP_NONE => Self::None,
            LWTUNNEL_ENCAP_MPLS => Self::Mpls,
            LWTUNNEL_ENCAP_IP => Self::Ip,
            LWTUNNEL_ENCAP_ILA => Self::Ila,
            LWTUNNEL_ENCAP_IP6 => Self::Ip6,
            LWTUNNEL_ENCAP_SEG6 => Self::Seg6,
            LWTUNNEL_ENCAP_BPF => Self::Bpf,
            LWTUNNEL_ENCAP_SEG6_LOCAL => Self::Seg6Local,
            LWTUNNEL_ENCAP_RPL => Self::Rpl,
            LWTUNNEL_ENCAP_IOAM6 => Self::Ioam6,
            LWTUNNEL_ENCAP_XFRM => Self::Xfrm,
            _ => Self::Other(d),
        }
    }
}

impl From<RouteLwEnCapType> for u16 {
    fn from(v: RouteLwEnCapType) -> u16 {
        match v {
            RouteLwEnCapType::None => LWTUNNEL_ENCAP_NONE,
            RouteLwEnCapType::Mpls => LWTUNNEL_ENCAP_MPLS,
            RouteLwEnCapType::Ip => LWTUNNEL_ENCAP_IP,
            RouteLwEnCapType::Ila => LWTUNNEL_ENCAP_ILA,
            RouteLwEnCapType::Ip6 => LWTUNNEL_ENCAP_IP6,
            RouteLwEnCapType::Seg6 => LWTUNNEL_ENCAP_SEG6,
            RouteLwEnCapType::Bpf => LWTUNNEL_ENCAP_BPF,
            RouteLwEnCapType::Seg6Local => LWTUNNEL_ENCAP_SEG6_LOCAL,
            RouteLwEnCapType::Rpl => LWTUNNEL_ENCAP_RPL,
            RouteLwEnCapType::Ioam6 => LWTUNNEL_ENCAP_IOAM6,
            RouteLwEnCapType::Xfrm => LWTUNNEL_ENCAP_XFRM,
            RouteLwEnCapType::Other(d) => d,
        }
    }
}

impl Emitable for RouteLwEnCapType {
    fn buffer_len(&self) -> usize {
        2
    }

    fn emit(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(&(u16::from(*self).to_ne_bytes()))
    }
}

impl std::fmt::Display for RouteLwEnCapType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Mpls => write!(f, "mpls"),
            Self::Ip => write!(f, "ip"),
            Self::Ila => write!(f, "ila"),
            Self::Ip6 => write!(f, "ip6"),
            Self::Seg6 => write!(f, "seg6"),
            Self::Bpf => write!(f, "bpf"),
            Self::Seg6Local => write!(f, "seg6_local"),
            Self::Rpl => write!(f, "rpl"),
            Self::Ioam6 => write!(f, "ioam6"),
            Self::Xfrm => write!(f, "xfrm"),
            Self::Other(d) => write!(f, "other({d})"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub enum RouteIp6Tunnel {
    #[default]
    Unspecified,
    Id(u64),
    Destination(Ipv6Addr),
    Source(Ipv6Addr),
    Hoplimit(u8),
    Tc(u8),
    Flags(RouteIp6TunnelFlags),
    Other(DefaultNla),
}

bitflags! {
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RouteIp6TunnelFlags : u16 {
        const Key = IP_TUNNEL_KEY_BIT;
        const Checksum = IP_TUNNEL_CSUM_BIT;
        const Sequence = IP_TUNNEL_SEQ_BIT;
        const _ = !0;
    }
}

impl std::fmt::Display for RouteIp6Tunnel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unspecified => write!(f, "unspecified"),
            Self::Id(id) => write!(f, "id {id}"),
            Self::Destination(dst) => write!(f, "dst {dst}"),
            Self::Source(src) => write!(f, "src, {src}"),
            Self::Hoplimit(hoplimit) => write!(f, "hoplimit {hoplimit}"),
            Self::Tc(tc) => write!(f, "tc {tc}"),
            Self::Flags(flags) => {
                if flags.contains(RouteIp6TunnelFlags::Key) {
                    write!(f, "key ")?;
                }
                if flags.contains(RouteIp6TunnelFlags::Checksum) {
                    write!(f, "csum ")?;
                }

                if flags.contains(RouteIp6TunnelFlags::Sequence) {
                    write!(f, "seq ")?;
                }

                Ok(())
            }
            Self::Other(other) => other.fmt(f),
        }
    }
}

impl Nla for RouteIp6Tunnel {
    fn value_len(&self) -> usize {
        match self {
            Self::Unspecified => 0,
            Self::Id(_) => size_of::<u64>(),
            Self::Destination(_) => size_of::<Ipv6Addr>(),
            Self::Source(_) => size_of::<Ipv6Addr>(),
            Self::Hoplimit(_) => size_of::<u8>(),
            Self::Tc(_) => size_of::<u8>(),
            Self::Flags(_) => size_of::<u16>(),
            Self::Other(_) => size_of::<DefaultNla>(),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Unspecified => LWTUNNEL_IP6_UNSPEC,
            Self::Id(_) => LWTUNNEL_IP6_ID,
            Self::Destination(_) => LWTUNNEL_IP6_DST,
            Self::Source(_) => LWTUNNEL_IP6_SRC,
            Self::Hoplimit(_) => LWTUNNEL_IP6_HOPLIMIT,
            Self::Tc(_) => LWTUNNEL_IP6_TC,
            Self::Flags(_) => LWTUNNEL_IP6_FLAGS,
            Self::Other(other) => other.kind(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Unspecified => {}
            Self::Id(id) => emit_u64_be(buffer, *id).unwrap(),
            Self::Destination(ip) | Self::Source(ip) => {
                buffer.copy_from_slice(&ip.octets());
            }
            Self::Hoplimit(value) | Self::Tc(value) => buffer[0] = *value,
            Self::Flags(flags) => emit_u16_be(buffer, flags.bits()).unwrap(),
            Self::Other(other) => other.emit_value(buffer),
        }
    }
}

impl<'a, T> Parseable<NlaBuffer<&'a T>> for RouteIp6Tunnel
where
    T: AsRef<[u8]> + ?Sized,
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            LWTUNNEL_IP6_UNSPEC => Self::Unspecified,
            LWTUNNEL_IP6_ID => Self::Id(
                parse_u64_be(payload)
                    .context("invalid LWTUNNEL_IP6_ID value")?,
            ),
            LWTUNNEL_IP6_DST => Self::Destination(
                parse_ipv6_addr(payload)
                    .context("invalid LWTUNNEL_IP6_DST value")?,
            ),
            LWTUNNEL_IP6_SRC => Self::Source(
                parse_ipv6_addr(payload)
                    .context("invalid LWTUNNEL_IP6_SRC value")?,
            ),
            LWTUNNEL_IP6_HOPLIMIT => Self::Hoplimit(
                parse_u8(payload)
                    .context("invalid LWTUNNEL_IP6_HOPLIMIT value")?,
            ),
            LWTUNNEL_IP6_TC => Self::Tc(
                parse_u8(payload).context("invalid LWTUNNEL_IP6_TC value")?,
            ),
            LWTUNNEL_IP6_FLAGS => {
                Self::Flags(RouteIp6TunnelFlags::from_bits_retain(
                    parse_u16_be(payload)
                        .context("invalid LWTUNNEL_IP6_FLAGS value")?,
                ))
            }
            _ => Self::Other(DefaultNla::parse(buf)?),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum RouteLwTunnelEncap {
    Mpls(RouteMplsIpTunnel),
    Seg6(RouteSeg6IpTunnel),
    Ip6(RouteIp6Tunnel),
    Other(DefaultNla),
}

impl Nla for RouteLwTunnelEncap {
    fn value_len(&self) -> usize {
        match self {
            Self::Mpls(v) => v.value_len(),
            Self::Seg6(v) => v.value_len(),
            Self::Ip6(v) => v.value_len(),
            Self::Other(v) => v.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Mpls(v) => v.emit_value(buffer),
            Self::Seg6(v) => v.emit_value(buffer),
            Self::Ip6(v) => v.emit_value(buffer),
            Self::Other(v) => v.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Mpls(v) => v.kind(),
            Self::Seg6(v) => v.kind(),
            Self::Ip6(v) => v.kind(),
            Self::Other(v) => v.kind(),
        }
    }
}

impl<'a, T> ParseableParametrized<NlaBuffer<&'a T>, RouteLwEnCapType>
    for RouteLwTunnelEncap
where
    T: AsRef<[u8]> + ?Sized,
{
    fn parse_with_param(
        buf: &NlaBuffer<&'a T>,
        kind: RouteLwEnCapType,
    ) -> Result<Self, DecodeError> {
        Ok(match kind {
            RouteLwEnCapType::Mpls => {
                Self::Mpls(RouteMplsIpTunnel::parse(buf)?)
            }
            RouteLwEnCapType::Seg6 => {
                Self::Seg6(RouteSeg6IpTunnel::parse(buf)?)
            }
            RouteLwEnCapType::Ip6 => Self::Ip6(RouteIp6Tunnel::parse(buf)?),
            _ => Self::Other(DefaultNla::parse(buf)?),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub(crate) struct VecRouteLwTunnelEncap(pub(crate) Vec<RouteLwTunnelEncap>);

impl<'a, T> ParseableParametrized<NlaBuffer<&'a T>, RouteLwEnCapType>
    for VecRouteLwTunnelEncap
where
    T: AsRef<[u8]> + ?Sized,
{
    fn parse_with_param(
        buf: &NlaBuffer<&'a T>,
        kind: RouteLwEnCapType,
    ) -> Result<Self, DecodeError> {
        let mut ret = Vec::new();
        for nla in NlasIterator::new(buf.value()) {
            let nla =
                nla.context(format!("Invalid RTA_ENCAP for kind: {kind}"))?;
            ret.push(RouteLwTunnelEncap::parse_with_param(&nla, kind).context(
                format!("Failed to parse RTA_ENCAP for kind: {kind}",),
            )?)
        }
        Ok(Self(ret))
    }
}
