// SPDX-License-Identifier: MIT

use std::{mem::size_of, net::Ipv6Addr};

use netlink_packet_core::{
    emit_u32, parse_u32, parse_u8, DecodeError, DefaultNla, Emitable,
    ErrorContext, Nla, NlaBuffer, NlasIterator, Parseable,
};

use super::super::{
    buffer_tool::expand_buffer_if_small, Icmp6Stats, Icmp6StatsBuffer,
    Inet6CacheInfo, Inet6DevConf, Inet6DevConfBuffer, Inet6IfaceFlags,
    Inet6Stats, Inet6StatsBuffer,
};
use crate::{ip::parse_ipv6_addr, link::af_spec::In6AddrGenMode};

const IFLA_INET6_FLAGS: u16 = 1;
const IFLA_INET6_CONF: u16 = 2;
const IFLA_INET6_STATS: u16 = 3;
// No kernel code used IFLA_INET6_MCAST
// const IFLA_INET6_MCAST: u16 = 4;
const IFLA_INET6_CACHEINFO: u16 = 5;
const IFLA_INET6_ICMP6STATS: u16 = 6;
const IFLA_INET6_TOKEN: u16 = 7;
const IFLA_INET6_ADDR_GEN_MODE: u16 = 8;
const IFLA_INET6_RA_MTU: u16 = 9;

#[derive(Clone, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum AfSpecInet6 {
    //TODO(Gris Ge): Use Vec<enum> for `IFF_UP` and etc
    Flags(Inet6IfaceFlags),
    CacheInfo(Inet6CacheInfo),
    DevConf(Inet6DevConf),
    Stats(Inet6Stats),
    Icmp6Stats(Icmp6Stats),
    Token(Ipv6Addr),
    AddrGenMode(In6AddrGenMode),
    RaMtu(u32),
    Other(DefaultNla),
}

pub(crate) struct VecAfSpecInet6(pub(crate) Vec<AfSpecInet6>);

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for VecAfSpecInet6
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let mut nlas = vec![];
        let err = "Invalid AF_INET6 NLA for IFLA_AF_SPEC(AF_UNSPEC)";
        for nla in NlasIterator::new(buf.into_inner()) {
            let nla = nla.context(err)?;
            nlas.push(AfSpecInet6::parse(&nla).context(err)?);
        }
        Ok(Self(nlas))
    }
}

impl Nla for AfSpecInet6 {
    fn value_len(&self) -> usize {
        match *self {
            Self::CacheInfo(ref cache_info) => cache_info.buffer_len(),
            Self::DevConf(ref dev_conf) => dev_conf.buffer_len(),
            Self::Stats(ref stats) => stats.buffer_len(),
            Self::Icmp6Stats(ref icmp_stats) => icmp_stats.buffer_len(),
            Self::Flags(_) | Self::RaMtu(_) => 4,
            Self::Token(_) => 16,
            Self::AddrGenMode(_) => 1,
            Self::Other(ref nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match *self {
            Self::Flags(ref value) => emit_u32(buffer, value.bits()).unwrap(),
            Self::RaMtu(ref value) => emit_u32(buffer, *value).unwrap(),
            Self::CacheInfo(ref v) => v.emit(buffer),
            Self::DevConf(ref v) => v.emit(buffer),
            Self::Stats(ref v) => v.emit(buffer),
            Self::Icmp6Stats(ref v) => v.emit(buffer),
            Self::Token(v) => buffer.copy_from_slice(&v.octets()),
            Self::AddrGenMode(ref v) => buffer[0] = v.into(),
            Self::Other(ref nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match *self {
            Self::Flags(_) => IFLA_INET6_FLAGS,
            Self::CacheInfo(_) => IFLA_INET6_CACHEINFO,
            Self::DevConf(_) => IFLA_INET6_CONF,
            Self::Stats(_) => IFLA_INET6_STATS,
            Self::Icmp6Stats(_) => IFLA_INET6_ICMP6STATS,
            Self::Token(_) => IFLA_INET6_TOKEN,
            Self::AddrGenMode(_) => IFLA_INET6_ADDR_GEN_MODE,
            Self::RaMtu(_) => IFLA_INET6_RA_MTU,
            Self::Other(ref nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for AfSpecInet6 {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_INET6_FLAGS => Self::Flags(Inet6IfaceFlags::from_bits_retain(
                parse_u32(payload).context("invalid IFLA_INET6_FLAGS value")?,
            )),
            IFLA_INET6_CACHEINFO => {
                Self::CacheInfo(Inet6CacheInfo::parse(payload).context(
                    format!("invalid IFLA_INET6_CACHEINFO value {payload:?}"),
                )?)
            }
            IFLA_INET6_CONF => Self::DevConf(
                Inet6DevConf::parse(
                    expand_buffer_if_small(
                        payload,
                        size_of::<Inet6DevConfBuffer>(),
                        "IFLA_INET6_CONF",
                    )
                    .as_slice(),
                )
                .context(format!(
                    "invalid IFLA_INET6_CONF value {payload:?}"
                ))?,
            ),
            IFLA_INET6_STATS => Self::Stats(
                Inet6Stats::parse(
                    expand_buffer_if_small(
                        payload,
                        size_of::<Inet6StatsBuffer>(),
                        "IFLA_INET6_STATS",
                    )
                    .as_slice(),
                )
                .context(format!(
                    "invalid IFLA_INET6_STATS value {payload:?}"
                ))?,
            ),
            IFLA_INET6_ICMP6STATS => Self::Icmp6Stats(
                Icmp6Stats::parse(
                    expand_buffer_if_small(
                        payload,
                        size_of::<Icmp6StatsBuffer>(),
                        "IFLA_INET6_ICMP6STATS",
                    )
                    .as_slice(),
                )
                .context(format!(
                    "invalid IFLA_INET6_ICMP6STATS value {payload:?}"
                ))?,
            ),
            IFLA_INET6_TOKEN => Self::Token(
                parse_ipv6_addr(payload)
                    .context("invalid IFLA_INET6_TOKEN value")?,
            ),
            IFLA_INET6_ADDR_GEN_MODE => {
                let mode = parse_u8(payload)
                    .context("invalid IFLA_INET6_ADDR_GEN_MODE")?;
                Self::AddrGenMode(In6AddrGenMode::from(mode))
            }
            IFLA_INET6_RA_MTU => Self::RaMtu(
                parse_u32(payload)
                    .context("invalid IFLA_INET6_RA_MTU value")?,
            ),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown AF_INET6 NLA type {kind} for IFLA_AF_SPEC(AF_UNSPEC)"
            ))?),
        })
    }
}
