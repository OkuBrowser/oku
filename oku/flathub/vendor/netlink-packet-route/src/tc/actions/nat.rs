// SPDX-License-Identifier: MIT

/// Nat action
///
/// The nat action maps one IP prefix to another
use std::mem::size_of;
use std::net::Ipv4Addr;

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, Nla, NlaBuffer, Parseable,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use super::{
    nat_flag::TcNatFlags, TcActionGeneric, TcActionGenericBuffer, Tcf,
    TcfBuffer,
};

const TCA_NAT_PARMS: u16 = 1;
const TCA_NAT_TM: u16 = 2;

/// Network address translation action.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub struct TcActionNat {}

impl TcActionNat {
    pub(crate) const KIND: &'static str = "nat";
}

/// Options for the [`TcActionNat`] action.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum TcActionNatOption {
    /// Rule installation and usage time
    Tm(Tcf),
    /// Parameters for the nat action.
    Parms(TcNat),
    /// Other attributes unknown at the time of writing.
    Other(DefaultNla),
}

impl Nla for TcActionNatOption {
    fn value_len(&self) -> usize {
        match self {
            Self::Tm(_) => size_of::<TcfBuffer>(),
            Self::Parms(v) => v.buffer_len(),
            Self::Other(attr) => attr.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Tm(p) => p.emit(buffer),
            Self::Parms(p) => p.emit(buffer),
            Self::Other(attr) => attr.emit_value(buffer),
        }
    }
    fn kind(&self) -> u16 {
        match self {
            Self::Tm(_) => TCA_NAT_TM,
            Self::Parms(_) => TCA_NAT_PARMS,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for TcActionNatOption
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            TCA_NAT_TM => Self::Tm(Tcf::parse(payload)?),
            TCA_NAT_PARMS => Self::Parms(TcNat::parse(payload)?),
            _ => Self::Other(DefaultNla::parse(buf)?),
        })
    }
}

/// Network address translation action.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub struct TcNat {
    /// Common attributes for all actions.
    pub generic: TcActionGeneric,
    /// Original address.
    pub old_addr: Ipv4Addr,
    /// New address.
    pub new_addr: Ipv4Addr,
    /// Mask of the old address
    pub mask: Ipv4Addr,
    /// Flags for the NAT action.
    pub flags: TcNatFlags,
}

impl Default for TcNat {
    fn default() -> Self {
        Self {
            generic: TcActionGeneric::default(),
            old_addr: Ipv4Addr::UNSPECIFIED,
            new_addr: Ipv4Addr::UNSPECIFIED,
            mask: Ipv4Addr::UNSPECIFIED,
            flags: TcNatFlags::empty(),
        }
    }
}

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
pub struct TcNatBuffer {
    generic: TcActionGenericBuffer,
    old_addr: [u8; 4],
    new_addr: [u8; 4],
    mask: [u8; 4],
    flags: u32,
}

impl TcNat {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = TcNatBuffer::ref_from_prefix(payload).map_err(|_| {
            DecodeError::buffer_too_small(
                payload.len(),
                size_of::<TcNatBuffer>(),
            )
        })?;
        Ok(Self {
            generic: TcActionGeneric::parse(
                &payload[..size_of::<TcActionGenericBuffer>()],
            )?,
            old_addr: Ipv4Addr::from(raw.old_addr),
            new_addr: Ipv4Addr::from(raw.new_addr),
            mask: Ipv4Addr::from(raw.mask),
            flags: TcNatFlags::from_bits_retain(raw.flags),
        })
    }
}

impl From<&TcNat> for TcNatBuffer {
    fn from(nat: &TcNat) -> Self {
        Self {
            generic: TcActionGenericBuffer::from(&nat.generic),
            old_addr: nat.old_addr.octets(),
            new_addr: nat.new_addr.octets(),
            mask: nat.mask.octets(),
            flags: nat.flags.bits(),
        }
    }
}

impl Emitable for TcNat {
    fn buffer_len(&self) -> usize {
        size_of::<TcNatBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = TcNatBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
