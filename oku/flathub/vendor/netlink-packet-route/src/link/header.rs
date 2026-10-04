// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use super::link_flag::LinkFlags;
use crate::{link::LinkLayerType, AddressFamily};

pub(crate) const LINK_HEADER_LEN: usize = 16;

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
pub struct LinkMessageBuffer {
    interface_family: u8,
    reserved_1: u8,
    link_layer_type: u16,
    link_index: u32,
    flags: u32,
    change_mask: u32,
}

/// High level representation of `RTM_GETLINK`, `RTM_SETLINK`, `RTM_NEWLINK` and
/// `RTM_DELLINK` messages headers.
///
/// These headers have the following structure:
///
/// ```no_rust
/// 0                8                16              24               32
/// +----------------+----------------+----------------+----------------+
/// |interface family|    reserved    |         link layer type         |
/// +----------------+----------------+----------------+----------------+
/// |                             link index                            |
/// +----------------+----------------+----------------+----------------+
/// |                               flags                               |
/// +----------------+----------------+----------------+----------------+
/// |                            change mask                            |
/// +----------------+----------------+----------------+----------------+
/// ```
///
/// `LinkHeader` exposes all these fields except for the "reserved" one.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct LinkHeader {
    /// Address family: one of the `AF_*` constants.
    /// The [AddressFamily] has `From<u8>` and `From<AddressFamily> for u8`
    /// implemented.
    pub interface_family: AddressFamily,
    /// Link index.
    pub index: u32,
    /// Link type. It should be set to one of the `ARPHRD_*`
    /// constants. The most common value is [LinkLayerType::Ether] for
    /// Ethernet.
    /// The LinkLayerType has `From<u16>` and `From<LinkLayerType> for u16`
    /// implemented.
    pub link_layer_type: LinkLayerType,
    /// State of the link, described by a combinations of `IFF_*`
    /// constants.
    pub flags: LinkFlags,
    /// Change mask for the `flags` field.
    pub change_mask: LinkFlags,
}

impl Emitable for LinkHeader {
    fn buffer_len(&self) -> usize {
        LINK_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = LinkMessageBuffer::from(self);
        buffer[..LINK_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}

impl LinkHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            LinkMessageBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), LINK_HEADER_LEN)
            })?;
        Ok(Self {
            interface_family: raw.interface_family.into(),
            link_layer_type: raw.link_layer_type.into(),
            index: raw.link_index,
            change_mask: LinkFlags::from_bits_retain(raw.change_mask),
            flags: LinkFlags::from_bits_retain(raw.flags),
        })
    }
}

impl From<&LinkHeader> for LinkMessageBuffer {
    fn from(header: &LinkHeader) -> Self {
        Self {
            interface_family: u8::from(header.interface_family),
            // The kernel expects the reserved byte to always be zero.
            reserved_1: 0,
            link_layer_type: u16::from(header.link_layer_type),
            link_index: header.index,
            flags: header.flags.bits(),
            change_mask: header.change_mask.bits(),
        }
    }
}
