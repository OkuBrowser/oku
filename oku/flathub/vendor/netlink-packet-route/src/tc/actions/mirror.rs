// SPDX-License-Identifier: MIT

/// Mirror action
///
/// The mirred action allows packet mirroring (copying) or
/// redirecting (stealing) the packet it receives. Mirroring is what
/// is sometimes referred to as Switch Port Analyzer (SPAN) and is
/// commonly used to analyze and/or debug flows.
use std::mem::size_of;

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, Nla, NlaBuffer, Parseable,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use super::{TcActionGeneric, TcActionGenericBuffer, Tcf, TcfBuffer};

/// Traffic control action used to mirror or redirect packets.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub struct TcActionMirror {}
impl TcActionMirror {
    /// The `TcActionAttribute::Kind` of this action.
    pub const KIND: &'static str = "mirred";
}

const TCA_MIRRED_TM: u16 = 1;
const TCA_MIRRED_PARMS: u16 = 2;

/// Options for the `TcActionMirror` action.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum TcActionMirrorOption {
    /// Rule installation and usage time
    Tm(Tcf),
    /// Parameters for the mirred action.
    Parms(TcMirror),
    /// Other attributes unknown at the time of writing.
    Other(DefaultNla),
}

impl Nla for TcActionMirrorOption {
    fn value_len(&self) -> usize {
        match self {
            Self::Tm(_) => size_of::<TcfBuffer>(),
            Self::Parms(_) => size_of::<TcMirrorBuffer>(),
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
            Self::Tm(_) => TCA_MIRRED_TM,
            Self::Parms(_) => TCA_MIRRED_PARMS,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for TcActionMirrorOption
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            TCA_MIRRED_TM => Self::Tm(Tcf::parse(payload)?),
            TCA_MIRRED_PARMS => Self::Parms(TcMirror::parse(payload)?),
            _ => Self::Other(DefaultNla::parse(buf)?),
        })
    }
}

/// Parameters for the mirred action.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct TcMirror {
    /// Generic action parameters.
    pub generic: TcActionGeneric,
    /// Describes how the packet be mirrored or redirected.
    pub eaction: TcMirrorActionType,
    /// Interface index to mirror or redirect to.
    pub ifindex: u32,
}

// kernel struct `tc_mirred`
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
pub struct TcMirrorBuffer {
    generic: TcActionGenericBuffer,
    eaction: i32,
    ifindex: u32,
}

impl TcMirror {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            TcMirrorBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<TcMirrorBuffer>(),
                )
            })?;
        Ok(Self {
            generic: TcActionGeneric::parse(
                &payload[..size_of::<TcActionGenericBuffer>()],
            )?,
            eaction: raw.eaction.into(),
            ifindex: raw.ifindex,
        })
    }
}

impl From<&TcMirror> for TcMirrorBuffer {
    fn from(mirror: &TcMirror) -> Self {
        Self {
            generic: TcActionGenericBuffer::from(&mirror.generic),
            eaction: mirror.eaction.into(),
            ifindex: mirror.ifindex,
        }
    }
}

impl Emitable for TcMirror {
    fn buffer_len(&self) -> usize {
        size_of::<TcMirrorBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = TcMirrorBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}

const TCA_EGRESS_REDIR: i32 = 1;
const TCA_EGRESS_MIRROR: i32 = 2;
const TCA_INGRESS_REDIR: i32 = 3;
const TCA_INGRESS_MIRROR: i32 = 4;

/// Type of mirroring or redirecting action.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[non_exhaustive]
pub enum TcMirrorActionType {
    #[default]
    /// Redirect to the egress pipeline.
    EgressRedir,
    /// Mirror to the egress pipeline.
    EgressMirror,
    /// Redirect to the ingress pipeline.
    IngressRedir,
    /// Mirror to the ingress pipeline.
    IngressMirror,
    /// Other action type unknown at the time of writing.
    Other(i32),
}

impl From<i32> for TcMirrorActionType {
    fn from(d: i32) -> Self {
        match d {
            TCA_EGRESS_REDIR => Self::EgressRedir,
            TCA_EGRESS_MIRROR => Self::EgressMirror,
            TCA_INGRESS_REDIR => Self::IngressRedir,
            TCA_INGRESS_MIRROR => Self::IngressMirror,
            _ => Self::Other(d),
        }
    }
}

impl From<TcMirrorActionType> for i32 {
    fn from(v: TcMirrorActionType) -> i32 {
        match v {
            TcMirrorActionType::EgressRedir => TCA_EGRESS_REDIR,
            TcMirrorActionType::EgressMirror => TCA_EGRESS_MIRROR,
            TcMirrorActionType::IngressRedir => TCA_INGRESS_REDIR,
            TcMirrorActionType::IngressMirror => TCA_INGRESS_MIRROR,
            TcMirrorActionType::Other(d) => d,
        }
    }
}
