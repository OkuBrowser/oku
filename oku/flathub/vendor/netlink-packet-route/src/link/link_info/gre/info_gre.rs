// SPDX-License-Identifier: MIT

use std::{mem::size_of, net::Ipv4Addr};

use netlink_packet_core::{
    emit_u16, emit_u16_be, emit_u32, emit_u32_be, parse_u16, parse_u16_be,
    parse_u32, parse_u32_be, parse_u8, DecodeError, DefaultNla, ErrorContext,
    Nla, NlaBuffer, Parseable,
};

use super::{
    gre_common::{
        IFLA_GRE_COLLECT_METADATA, IFLA_GRE_ENCAP_DPORT, IFLA_GRE_ENCAP_FLAGS,
        IFLA_GRE_ENCAP_SPORT, IFLA_GRE_ENCAP_TYPE, IFLA_GRE_ERSPAN_DIR,
        IFLA_GRE_ERSPAN_HWID, IFLA_GRE_ERSPAN_INDEX, IFLA_GRE_ERSPAN_VER,
        IFLA_GRE_FWMARK, IFLA_GRE_IFLAGS, IFLA_GRE_IKEY, IFLA_GRE_LOCAL,
        IFLA_GRE_OFLAGS, IFLA_GRE_OKEY, IFLA_GRE_PMTUDISC, IFLA_GRE_REMOTE,
        IFLA_GRE_TOS, IFLA_GRE_TTL,
    },
    ErSpanDir, GreEncapFlags, GreEncapType, GreIOFlags,
};
use crate::{
    ip::parse_ipv4_addr, link::link_info::gre::gre_common::IFLA_GRE_LINK,
};

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoGre {
    Link(u32),
    IFlags(GreIOFlags),
    OFlags(GreIOFlags),
    IKey(u32),
    OKey(u32),
    Local(Ipv4Addr),
    Remote(Ipv4Addr),
    Ttl(u8),
    Tos(u8),
    PathMTUDiscovery(bool),
    EncapType(GreEncapType),
    EncapFlags(GreEncapFlags),
    SourcePort(u16),
    DestinationPort(u16),
    CollectMetadata,
    FwMask(u32),
    ErSpanIndex(u32),
    ErSpanVer(u8),
    ErSpanDir(ErSpanDir),
    ErSpanHwId(u16),
    Other(DefaultNla),
}

impl Nla for InfoGre {
    fn value_len(&self) -> usize {
        match self {
            Self::Link(_) => size_of::<u32>(),
            Self::IFlags(_) | Self::OFlags(_) => size_of::<u16>(),
            Self::IKey(_) | Self::OKey(_) => size_of::<u32>(),
            Self::Local(_) | Self::Remote(_) => size_of::<Ipv4Addr>(),
            Self::Ttl(_) | Self::Tos(_) | Self::PathMTUDiscovery(_) => {
                size_of::<u8>()
            }
            Self::EncapType(_) => size_of::<u16>(),
            Self::EncapFlags(_) => size_of::<u16>(),
            Self::SourcePort(_) | Self::DestinationPort(_) => size_of::<u16>(),
            Self::CollectMetadata => 0,
            Self::FwMask(_) => size_of::<u32>(),
            Self::ErSpanIndex(_) => size_of::<u32>(),
            Self::ErSpanVer(_) | Self::ErSpanDir(_) => size_of::<u8>(),
            Self::ErSpanHwId(_) => size_of::<u16>(),
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Link(id) => emit_u32(buffer, *id).unwrap(),
            Self::IFlags(flags) | Self::OFlags(flags) => {
                emit_u16_be(buffer, flags.bits()).unwrap()
            }
            Self::IKey(key) | Self::OKey(key) => {
                emit_u32_be(buffer, *key).unwrap()
            }
            Self::Local(ip) | Self::Remote(ip) => {
                buffer.copy_from_slice(&ip.octets());
            }
            Self::Ttl(value) | Self::Tos(value) => buffer[0] = *value,
            Self::PathMTUDiscovery(discover) => {
                buffer[0] = if *discover { 1 } else { 0 }
            }
            Self::EncapType(t) => emit_u16_be(buffer, t.into()).unwrap(),
            Self::EncapFlags(flags) => {
                emit_u16_be(buffer, flags.bits()).unwrap()
            }
            Self::SourcePort(port) | Self::DestinationPort(port) => {
                emit_u16_be(buffer, *port).unwrap()
            }
            Self::CollectMetadata => {}
            Self::FwMask(fw_mask) => emit_u32(buffer, *fw_mask).unwrap(),
            Self::ErSpanIndex(idx) => emit_u32(buffer, *idx).unwrap(),
            Self::ErSpanVer(v) => buffer[0] = *v,
            Self::ErSpanDir(v) => buffer[0] = v.into(),
            Self::ErSpanHwId(id) => emit_u16(buffer, *id).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Link(_) => IFLA_GRE_LINK,
            Self::IFlags(_) => IFLA_GRE_IFLAGS,
            Self::OFlags(_) => IFLA_GRE_OFLAGS,
            Self::IKey(_) => IFLA_GRE_IKEY,
            Self::Local(_) => IFLA_GRE_LOCAL,
            Self::Remote(_) => IFLA_GRE_REMOTE,
            Self::OKey(_) => IFLA_GRE_OKEY,
            Self::Ttl(_) => IFLA_GRE_TTL,
            Self::Tos(_) => IFLA_GRE_TOS,
            Self::PathMTUDiscovery(_) => IFLA_GRE_PMTUDISC,
            Self::EncapType(_) => IFLA_GRE_ENCAP_TYPE,
            Self::EncapFlags(_) => IFLA_GRE_ENCAP_FLAGS,
            Self::SourcePort(_) => IFLA_GRE_ENCAP_SPORT,
            Self::DestinationPort(_) => IFLA_GRE_ENCAP_DPORT,
            Self::CollectMetadata => IFLA_GRE_COLLECT_METADATA,
            Self::FwMask(_) => IFLA_GRE_FWMARK,
            Self::ErSpanIndex(_) => IFLA_GRE_ERSPAN_INDEX,
            Self::ErSpanVer(_) => IFLA_GRE_ERSPAN_VER,
            Self::ErSpanDir(_) => IFLA_GRE_ERSPAN_DIR,
            Self::ErSpanHwId(_) => IFLA_GRE_ERSPAN_HWID,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoGre {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_GRE_LINK => Self::Link(
                parse_u32(payload).context("invalid IFLA_GRE_LINK value")?,
            ),
            IFLA_GRE_IFLAGS => Self::IFlags(GreIOFlags::from_bits_retain(
                parse_u16_be(payload)
                    .context("invalid IFLA_GRE_IFLAGS value")?,
            )),
            IFLA_GRE_OFLAGS => Self::OFlags(GreIOFlags::from_bits_retain(
                parse_u16_be(payload)
                    .context("invalid IFLA_GRE_OFLAGS value")?,
            )),
            IFLA_GRE_IKEY => Self::IKey(
                parse_u32_be(payload).context("invalid IFLA_GRE_IKEY value")?,
            ),
            IFLA_GRE_OKEY => Self::OKey(
                parse_u32_be(payload).context("invalid IFLA_GRE_OKEY value")?,
            ),
            IFLA_GRE_LOCAL => Self::Local(
                parse_ipv4_addr(payload)
                    .context("invalid IFLA_GRE_LOCAL value")?,
            ),
            IFLA_GRE_REMOTE => Self::Remote(
                parse_ipv4_addr(payload)
                    .context("invalid IFLA_GRE_LOCAL value")?,
            ),
            IFLA_GRE_TTL => Self::Ttl(
                parse_u8(payload).context("invalid IFLA_GRE_TTL value")?,
            ),
            IFLA_GRE_TOS => Self::Tos(
                parse_u8(payload).context("invalid IFLA_GRE_TOS value")?,
            ),
            IFLA_GRE_PMTUDISC => Self::PathMTUDiscovery(
                parse_u8(payload).context("invalid IFLA_GRE_TOS value")? == 1,
            ),
            IFLA_GRE_ENCAP_TYPE => Self::EncapType(GreEncapType::from(
                parse_u16_be(payload)
                    .context("invalid IFLA_GRE_ENCAP_TYPE value")?,
            )),
            IFLA_GRE_ENCAP_FLAGS => {
                Self::EncapFlags(GreEncapFlags::from_bits_retain(
                    parse_u16_be(payload)
                        .context("invalid IFLA_GRE_ENCAP_FLAGS value")?,
                ))
            }
            IFLA_GRE_ENCAP_SPORT => Self::SourcePort(
                parse_u16_be(payload)
                    .context("invalid IFLA_GRE_ENCAP_SPORT value")?,
            ),
            IFLA_GRE_ENCAP_DPORT => Self::DestinationPort(
                parse_u16_be(payload)
                    .context("invalid IFLA_GRE_ENCAP_DPORT value")?,
            ),
            IFLA_GRE_COLLECT_METADATA => Self::CollectMetadata,
            IFLA_GRE_FWMARK => Self::FwMask(
                parse_u32(payload).context("invalid IFLA_GRE_FWMARK value")?,
            ),
            IFLA_GRE_ERSPAN_INDEX => Self::ErSpanIndex(
                parse_u32(payload)
                    .context("invalid IFLA_GRE_ERSPAN_INDEX value")?,
            ),
            IFLA_GRE_ERSPAN_VER => Self::ErSpanVer(
                parse_u8(payload)
                    .context("invalid IFLA_GRE_ERSPAN_VER value")?,
            ),
            IFLA_GRE_ERSPAN_DIR => Self::ErSpanDir(ErSpanDir::from(
                parse_u8(payload)
                    .context("invalid IFLA_GRE_ERSPAN_DIR value")?,
            )),
            IFLA_GRE_ERSPAN_HWID => Self::ErSpanHwId(
                parse_u16(payload)
                    .context("invalid IFLA_GRE_ERSPAN_HWID value")?,
            ),
            kind => Self::Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind} for ip6gre"))?,
            ),
        })
    }
}
