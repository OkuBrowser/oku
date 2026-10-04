// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use super::{flags::NeighbourFlags, NeighbourState};
use crate::{route::RouteType, AddressFamily};

pub(crate) const NEIGHBOUR_HEADER_LEN: usize = 12;

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
pub struct NeighbourMessageBuffer {
    family: u8,
    _pad: [u8; 3],
    ifindex: u32,
    state: u16,
    flags: u8,
    kind: u8,
}

/// Neighbour headers have the following structure:
///
/// ```no_rust
/// 0                8                16              24               32
/// +----------------+----------------+----------------+----------------+
/// |     family     |                     padding                      |
/// +----------------+----------------+----------------+----------------+
/// |                             link index                            |
/// +----------------+----------------+----------------+----------------+
/// |              state              |     flags      |     ntype      |
/// +----------------+----------------+----------------+----------------+
/// ```
///
/// `NeighbourHeader` exposes all these fields.
// Linux kernel struct `struct ndmsg`
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct NeighbourHeader {
    pub family: AddressFamily,
    pub ifindex: u32,
    /// Neighbour cache entry state.
    pub state: NeighbourState,
    /// Neighbour cache entry flags. It should be set to a combination
    /// of the `NTF_*` constants
    pub flags: NeighbourFlags,
    /// Neighbour cache entry type. It should be set to one of the
    /// `NDA_*` constants.
    pub kind: RouteType,
}

impl NeighbourHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = NeighbourMessageBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    NEIGHBOUR_HEADER_LEN,
                )
            })?;
        Ok(Self {
            family: raw.family.into(),
            ifindex: raw.ifindex,
            state: raw.state.into(),
            flags: NeighbourFlags::from_bits_retain(raw.flags),
            kind: raw.kind.into(),
        })
    }
}

impl From<&NeighbourHeader> for NeighbourMessageBuffer {
    fn from(header: &NeighbourHeader) -> Self {
        Self {
            family: header.family.into(),
            _pad: [0; 3],
            ifindex: header.ifindex,
            state: header.state.into(),
            flags: header.flags.bits(),
            kind: header.kind.into(),
        }
    }
}

impl Emitable for NeighbourHeader {
    fn buffer_len(&self) -> usize {
        NEIGHBOUR_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = NeighbourMessageBuffer::from(self);
        buffer[..NEIGHBOUR_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}
