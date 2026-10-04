// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u16, emit_u16_be, emit_u32, parse_u16, parse_u16_be, parse_u32,
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer,
    NlasIterator, Parseable,
};

use crate::link::VlanProtocol;

const VLAN_FLAG_REORDER_HDR: u32 = 0x1;
const VLAN_FLAG_GVRP: u32 = 0x2;
const VLAN_FLAG_LOOSE_BINDING: u32 = 0x4;
const VLAN_FLAG_MVRP: u32 = 0x8;
const VLAN_FLAG_BRIDGE_BINDING: u32 = 0x10;

bitflags! {
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VlanFlags: u32 {
        /// Ethernet headers are reordered
        const ReorderHdr = VLAN_FLAG_REORDER_HDR;
        /// VLAN is registered using GARP VLAN Registration Protocol
        const Gvrp = VLAN_FLAG_GVRP;
        /// VLAN device state is bound to the physical device state
        const LooseBinding = VLAN_FLAG_LOOSE_BINDING;
        /// VLAN is be registered using Multiple VLAN Registration Protocol
        const Mvrp = VLAN_FLAG_MVRP;
        /// VLAN device link state tracks the state of bridge ports that are
        /// members of the VLAN
        const BridgeBinding = VLAN_FLAG_BRIDGE_BINDING;
        const _ = !0;
    }
}

const IFLA_VLAN_ID: u16 = 1;
const IFLA_VLAN_FLAGS: u16 = 2;
const IFLA_VLAN_EGRESS_QOS: u16 = 3;
const IFLA_VLAN_INGRESS_QOS: u16 = 4;
const IFLA_VLAN_PROTOCOL: u16 = 5;

const IFLA_VLAN_QOS_MAPPING: u16 = 1;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoVlan {
    Id(u16),
    /// Active flags and flags mask
    Flags((VlanFlags, VlanFlags)),
    EgressQos(Vec<VlanQosMapping>),
    IngressQos(Vec<VlanQosMapping>),
    Protocol(VlanProtocol),
    Other(DefaultNla),
}

impl Nla for InfoVlan {
    fn value_len(&self) -> usize {
        match self {
            Self::Id(_) | Self::Protocol(_) => 2,
            Self::Flags(_) => 8,
            Self::EgressQos(mappings) | Self::IngressQos(mappings) => {
                mappings.as_slice().buffer_len()
            }
            Self::Other(v) => v.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::EgressQos(ref mappings) | Self::IngressQos(ref mappings) => {
                mappings.as_slice().emit(buffer)
            }
            Self::Id(value) => emit_u16(buffer, *value).unwrap(),
            Self::Protocol(value) => {
                emit_u16_be(buffer, (*value).into()).unwrap()
            }
            Self::Flags((active, mask)) => {
                emit_u32(&mut buffer[0..4], active.bits()).unwrap();
                emit_u32(&mut buffer[4..8], mask.bits()).unwrap()
            }
            Self::Other(v) => v.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Id(_) => IFLA_VLAN_ID,
            Self::Flags(_) => IFLA_VLAN_FLAGS,
            Self::EgressQos(_) => IFLA_VLAN_EGRESS_QOS,
            Self::IngressQos(_) => IFLA_VLAN_INGRESS_QOS,
            Self::Protocol(_) => IFLA_VLAN_PROTOCOL,
            Self::Other(v) => v.kind(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum VlanQosMapping {
    /// Tuple (from, to)
    Mapping(u32, u32),
    Other(DefaultNla),
}

impl Nla for VlanQosMapping {
    fn value_len(&self) -> usize {
        match self {
            VlanQosMapping::Mapping { .. } => 8,
            VlanQosMapping::Other(nla) => nla.value_len(),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            VlanQosMapping::Mapping { .. } => IFLA_VLAN_QOS_MAPPING,
            VlanQosMapping::Other(nla) => nla.kind(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        use VlanQosMapping::*;
        match self {
            Mapping(from, to) => {
                emit_u32(buffer, *from).unwrap();
                emit_u32(&mut buffer[4..], *to).unwrap();
            }
            Other(nla) => nla.emit_value(buffer),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for VlanQosMapping
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        use VlanQosMapping::*;
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_VLAN_QOS_MAPPING => {
                if payload.len() != 8 {
                    return Err("invalid IFLA_VLAN_QOS_MAPPING value".into());
                }
                Mapping(
                    parse_u32(&payload[..4])
                        .context("expected u32 from value")?,
                    parse_u32(&payload[4..])
                        .context("expected u32 to value")?,
                )
            }
            kind => Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for VLAN QoS mapping"
            ))?),
        })
    }
}

fn parse_mappings(payload: &[u8]) -> Result<Vec<VlanQosMapping>, DecodeError> {
    let mut mappings = Vec::new();
    for nla in NlasIterator::new(payload) {
        let nla = nla?;
        let parsed = VlanQosMapping::parse(&nla)?;
        mappings.push(parsed);
    }
    Ok(mappings)
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoVlan {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        use self::InfoVlan::*;
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_VLAN_ID => {
                Id(parse_u16(payload).context("invalid IFLA_VLAN_ID value")?)
            }
            IFLA_VLAN_FLAGS => {
                let err = "invalid IFLA_VLAN_FLAGS value";
                if payload.len() != 8 {
                    return Err(err.into());
                }
                let flags = parse_u32(&payload[0..4]).context(err)?;
                let mask = parse_u32(&payload[4..]).context(err)?;
                Flags((
                    VlanFlags::from_bits_retain(flags),
                    VlanFlags::from_bits_retain(mask),
                ))
            }
            IFLA_VLAN_EGRESS_QOS => EgressQos(
                parse_mappings(payload)
                    .context("failed to parse IFLA_VLAN_EGRESS_QOS")?,
            ),
            IFLA_VLAN_INGRESS_QOS => IngressQos(
                parse_mappings(payload)
                    .context("failed to parse IFLA_VLAN_INGRESS_QOS")?,
            ),
            IFLA_VLAN_PROTOCOL => Protocol(
                parse_u16_be(payload)
                    .context("invalid IFLA_VLAN_PROTOCOL value")?
                    .into(),
            ),
            _ => Self::Other(DefaultNla::parse(buf).context(format!(
                "invalid NLA for {}: {payload:?}",
                buf.kind()
            ))?),
        })
    }
}
