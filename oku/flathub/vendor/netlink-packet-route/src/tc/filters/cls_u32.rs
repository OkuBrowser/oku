// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::ErrorContext;
/// U32 filter
///
/// In its simplest form the U32 filter is a list of records, each
/// consisting of two fields: a selector and an action. The selectors,
/// described below, are compared with the currently processed IP packet
/// until the first match occurs, and then the associated action is
/// performed.
use netlink_packet_core::{
    emit_u32, parse_u32, DecodeError, Emitable, Parseable,
    {DefaultNla, Nla, NlaBuffer, NlasIterator},
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use super::u32_flags::{TcU32OptionFlags, TcU32SelectorFlags};
use crate::tc::{TcAction, TcHandle};

const TCA_U32_CLASSID: u16 = 1;
const TCA_U32_HASH: u16 = 2;
const TCA_U32_LINK: u16 = 3;
const TCA_U32_DIVISOR: u16 = 4;
const TCA_U32_SEL: u16 = 5;
const TCA_U32_POLICE: u16 = 6;
const TCA_U32_ACT: u16 = 7;
const TCA_U32_INDEV: u16 = 8;
const TCA_U32_PCNT: u16 = 9;
const TCA_U32_MARK: u16 = 10;
const TCA_U32_FLAGS: u16 = 11;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub struct TcFilterU32 {}

impl TcFilterU32 {
    pub const KIND: &'static str = "u32";
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum TcFilterU32Option {
    ClassId(TcHandle),
    Hash(u32),
    Link(u32),
    Divisor(u32),
    Selector(TcU32Selector),
    Police(Vec<u8>),
    Action(Vec<TcAction>),
    Indev(Vec<u8>),
    Pnct(Vec<u8>),
    Mark(Vec<u8>),
    Flags(TcU32OptionFlags),
    Other(DefaultNla),
}

impl Nla for TcFilterU32Option {
    fn value_len(&self) -> usize {
        match self {
            Self::Police(b)
            | Self::Indev(b)
            | Self::Pnct(b)
            | Self::Mark(b) => b.len(),
            Self::Hash(_)
            | Self::Link(_)
            | Self::Divisor(_)
            | Self::Flags(_) => 4,
            Self::ClassId(_) => 4,
            Self::Selector(s) => s.buffer_len(),
            Self::Action(acts) => acts.as_slice().buffer_len(),
            Self::Other(attr) => attr.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Police(b)
            | Self::Indev(b)
            | Self::Pnct(b)
            | Self::Mark(b) => buffer.copy_from_slice(b.as_slice()),
            Self::Hash(i) | Self::Link(i) | Self::Divisor(i) => {
                emit_u32(buffer, *i).unwrap()
            }
            Self::Flags(f) => emit_u32(buffer, f.bits()).unwrap(),
            Self::ClassId(i) => emit_u32(buffer, (*i).into()).unwrap(),
            Self::Selector(s) => s.emit(buffer),
            Self::Action(acts) => acts.as_slice().emit(buffer),
            Self::Other(attr) => attr.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::ClassId(_) => TCA_U32_CLASSID,
            Self::Hash(_) => TCA_U32_HASH,
            Self::Link(_) => TCA_U32_LINK,
            Self::Divisor(_) => TCA_U32_DIVISOR,
            Self::Selector(_) => TCA_U32_SEL,
            Self::Police(_) => TCA_U32_POLICE,
            Self::Action(_) => TCA_U32_ACT,
            Self::Indev(_) => TCA_U32_INDEV,
            Self::Pnct(_) => TCA_U32_PCNT,
            Self::Mark(_) => TCA_U32_MARK,
            Self::Flags(_) => TCA_U32_FLAGS,
            Self::Other(attr) => attr.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for TcFilterU32Option
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            TCA_U32_CLASSID => Self::ClassId(TcHandle::from(
                parse_u32(payload)
                    .context("failed to parse TCA_U32_CLASSID")?,
            )),
            TCA_U32_HASH => Self::Hash(
                parse_u32(payload).context("failed to parse TCA_U32_HASH")?,
            ),
            TCA_U32_LINK => Self::Link(
                parse_u32(payload).context("failed to parse TCA_U32_LINK")?,
            ),
            TCA_U32_DIVISOR => Self::Divisor(
                parse_u32(payload)
                    .context("failed to parse TCA_U32_DIVISOR")?,
            ),
            TCA_U32_SEL => Self::Selector(
                TcU32Selector::parse(payload)
                    .context("failed to parse TCA_U32_SEL")?,
            ),
            TCA_U32_POLICE => Self::Police(payload.to_vec()),
            TCA_U32_ACT => {
                let mut acts = vec![];
                for act in NlasIterator::new(payload) {
                    let act = act.context("invalid TCA_U32_ACT")?;
                    acts.push(
                        TcAction::parse(&act)
                            .context("failed to parse TCA_U32_ACT")?,
                    );
                }
                Self::Action(acts)
            }
            TCA_U32_INDEV => Self::Indev(payload.to_vec()),
            TCA_U32_PCNT => Self::Pnct(payload.to_vec()),
            TCA_U32_MARK => Self::Mark(payload.to_vec()),
            TCA_U32_FLAGS => Self::Flags(TcU32OptionFlags::from_bits_retain(
                parse_u32(payload).context("failed to parse TCA_U32_FLAGS")?,
            )),
            _ => Self::Other(
                DefaultNla::parse(buf).context("failed to parse u32 nla")?,
            ),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct TcU32Selector {
    pub flags: TcU32SelectorFlags,
    pub offshift: u8,
    pub nkeys: u8,
    pub offmask: u16,
    pub off: u16,
    pub offoff: u16,
    pub hoff: u16,
    pub hmask: u32,
    pub keys: Vec<TcU32Key>,
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
pub struct TcU32SelectorBuffer {
    flags: u8,
    offshift: u8,
    nkeys: u8,
    _pad: u8,
    offmask: u16,
    off: u16,
    offoff: u16,
    hoff: u16,
    hmask: u32,
}

impl TcU32SelectorBuffer {
    pub fn new_checked(buffer: &[u8]) -> Result<&Self, DecodeError> {
        let len = buffer.len();
        let sel_len = size_of::<Self>();
        if len < sel_len {
            return Err(format!(
                "invalid TcU32SelectorBuffer: length {len} < {sel_len}"
            )
            .into());
        }
        let raw = Self::ref_from_prefix(buffer)
            .map_err(|_| DecodeError::buffer_too_small(len, sel_len))?
            .0;
        let nkeys = raw.nkeys;
        // Expect the buffer to be large enough to hold `nkeys`.
        let expected_len =
            (nkeys as usize * size_of::<TcU32KeyBuffer>()) + sel_len;
        if len < expected_len {
            return Err(format!(
                "invalid TcU32SelectorBuffer: length {len} < {expected_len}",
            )
            .into());
        }
        Ok(raw)
    }
}

impl From<&TcU32Selector> for TcU32SelectorBuffer {
    fn from(selector: &TcU32Selector) -> Self {
        Self {
            flags: selector.flags.bits(),
            offshift: selector.offshift,
            nkeys: selector.nkeys,
            _pad: 0,
            offmask: selector.offmask,
            off: selector.off,
            offoff: selector.offoff,
            hoff: selector.hoff,
            hmask: selector.hmask,
        }
    }
}

impl Emitable for TcU32Selector {
    fn buffer_len(&self) -> usize {
        size_of::<TcU32SelectorBuffer>()
            + (self.nkeys as usize * size_of::<TcU32KeyBuffer>())
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = TcU32SelectorBuffer::from(self);
        let sel_len = size_of::<TcU32SelectorBuffer>();
        let key_len = size_of::<TcU32KeyBuffer>();
        buffer[..sel_len].copy_from_slice(raw.as_bytes());
        for (i, k) in self.keys.iter().enumerate() {
            k.emit(
                &mut buffer
                    [(sel_len + i * key_len)..(sel_len + (i + 1) * key_len)],
            );
        }
    }
}

impl TcU32Selector {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let raw = TcU32SelectorBuffer::new_checked(payload)?;
        let nkeys = raw.nkeys;
        let key_len = size_of::<TcU32KeyBuffer>();
        let key_payload = &payload[size_of::<TcU32SelectorBuffer>()..];
        let mut keys = Vec::<TcU32Key>::with_capacity(nkeys.into());
        for i in 0..nkeys {
            let i = i as usize;
            keys.push(
                TcU32Key::parse(
                    &key_payload[(i * key_len)..((i + 1) * key_len)],
                )
                .context("failed to parse u32 key")?,
            );
        }

        Ok(Self {
            flags: TcU32SelectorFlags::from_bits_retain(raw.flags),
            offshift: raw.offshift,
            nkeys,
            offmask: raw.offmask,
            off: raw.off,
            offoff: raw.offoff,
            hoff: raw.hoff,
            hmask: raw.hmask,
            keys,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct TcU32Key {
    pub mask: u32,
    pub val: u32,
    pub off: i32,
    pub offmask: i32,
}

// Wire format of a `struct tc_u32_key`: `mask` and `val` are `__be32`
// (always big-endian on the wire), while `off` and `offmask` are plain
// native `int`.
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
pub struct TcU32KeyBuffer {
    mask: u32,
    val: u32,
    off: i32,
    offmask: i32,
}

impl From<&TcU32Key> for TcU32KeyBuffer {
    fn from(key: &TcU32Key) -> Self {
        Self {
            mask: key.mask.to_be(),
            val: key.val.to_be(),
            off: key.off,
            offmask: key.offmask,
        }
    }
}

impl Emitable for TcU32Key {
    fn buffer_len(&self) -> usize {
        size_of::<TcU32KeyBuffer>()
    }
    fn emit(&self, buffer: &mut [u8]) {
        let raw = TcU32KeyBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}

impl TcU32Key {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            TcU32KeyBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<TcU32KeyBuffer>(),
                )
            })?;
        Ok(Self {
            mask: u32::from_be(raw.mask),
            val: u32::from_be(raw.val),
            off: raw.off,
            offmask: raw.offmask,
        })
    }
}
