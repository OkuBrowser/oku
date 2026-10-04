// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_i32, emit_u16, emit_u32, parse_i32, parse_mac, parse_u16, parse_u32,
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer,
    NlasIterator, Parseable,
};

const IFLA_MACVLAN_MODE: u16 = 1;
const IFLA_MACVLAN_FLAGS: u16 = 2;
const IFLA_MACVLAN_MACADDR_MODE: u16 = 3;
const IFLA_MACVLAN_MACADDR: u16 = 4;
const IFLA_MACVLAN_MACADDR_DATA: u16 = 5;
const IFLA_MACVLAN_MACADDR_COUNT: u16 = 6;
const IFLA_MACVLAN_BC_QUEUE_LEN: u16 = 7;
const IFLA_MACVLAN_BC_QUEUE_LEN_USED: u16 = 8;
const IFLA_MACVLAN_BC_CUTOFF: u16 = 9;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoMacVlan {
    Mode(MacVlanMode),
    Flags(MacVlanFlags),
    MacAddrMode(MacVlanMacAddressMode),
    MacAddr([u8; 6]),
    /// A list of InfoMacVlan::MacAddr
    MacAddrData(Vec<InfoMacVlan>),
    MacAddrCount(u32),
    BcQueueLen(u32),
    BcQueueLenUsed(u32),
    BcCutoff(i32),
    Other(DefaultNla),
}

impl Nla for InfoMacVlan {
    fn value_len(&self) -> usize {
        match self {
            Self::Mode(_) => 4,
            Self::Flags(_) => 2,
            Self::MacAddrMode(_) => 4,
            Self::MacAddr(_) => 6,
            Self::MacAddrData(ref nlas) => nlas.as_slice().buffer_len(),
            Self::MacAddrCount(_) => 4,
            Self::BcQueueLen(_) => 4,
            Self::BcQueueLenUsed(_) => 4,
            Self::BcCutoff(_) => 4,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Mode(value) => emit_u32(buffer, (*value).into()).unwrap(),
            Self::Flags(value) => emit_u16(buffer, value.bits()).unwrap(),
            Self::MacAddrMode(value) => {
                emit_u32(buffer, (*value).into()).unwrap()
            }
            Self::MacAddr(bytes) => buffer.copy_from_slice(bytes),
            Self::MacAddrData(ref nlas) => nlas.as_slice().emit(buffer),
            Self::MacAddrCount(value) => emit_u32(buffer, *value).unwrap(),
            Self::BcQueueLen(value) => emit_u32(buffer, *value).unwrap(),
            Self::BcQueueLenUsed(value) => emit_u32(buffer, *value).unwrap(),
            Self::BcCutoff(value) => emit_i32(buffer, *value).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Mode(_) => IFLA_MACVLAN_MODE,
            Self::Flags(_) => IFLA_MACVLAN_FLAGS,
            Self::MacAddrMode(_) => IFLA_MACVLAN_MACADDR_MODE,
            Self::MacAddr(_) => IFLA_MACVLAN_MACADDR,
            Self::MacAddrData(_) => IFLA_MACVLAN_MACADDR_DATA,
            Self::MacAddrCount(_) => IFLA_MACVLAN_MACADDR_COUNT,
            Self::BcQueueLen(_) => IFLA_MACVLAN_BC_QUEUE_LEN,
            Self::BcQueueLenUsed(_) => IFLA_MACVLAN_BC_QUEUE_LEN_USED,
            Self::BcCutoff(_) => IFLA_MACVLAN_BC_CUTOFF,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoMacVlan {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_MACVLAN_MODE => Self::Mode(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_MODE value")?
                    .into(),
            ),
            IFLA_MACVLAN_FLAGS => Self::Flags(MacVlanFlags::from_bits_retain(
                parse_u16(payload)
                    .context("invalid IFLA_MACVLAN_FLAGS value")?,
            )),
            IFLA_MACVLAN_MACADDR_MODE => Self::MacAddrMode(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_MACADDR_MODE value")?
                    .into(),
            ),
            IFLA_MACVLAN_MACADDR => Self::MacAddr(
                parse_mac(payload)
                    .context("invalid IFLA_MACVLAN_MACADDR value")?,
            ),
            IFLA_MACVLAN_MACADDR_DATA => {
                let mut mac_data = Vec::new();
                let err = "failed to parse IFLA_MACVLAN_MACADDR_DATA";
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(err)?;
                    let parsed = InfoMacVlan::parse(nla).context(err)?;
                    mac_data.push(parsed);
                }
                Self::MacAddrData(mac_data)
            }
            IFLA_MACVLAN_MACADDR_COUNT => Self::MacAddrCount(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_MACADDR_COUNT value")?,
            ),
            IFLA_MACVLAN_BC_QUEUE_LEN => Self::BcQueueLen(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_BC_QUEUE_LEN value")?,
            ),
            IFLA_MACVLAN_BC_QUEUE_LEN_USED => Self::BcQueueLenUsed(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_BC_QUEUE_LEN_USED value")?,
            ),
            IFLA_MACVLAN_BC_CUTOFF => Self::BcCutoff(
                parse_i32(payload)
                    .context("invalid IFLA_MACVLAN_BC_CUTOFF value")?,
            ),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(mac_vlan)"
            ))?),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoMacVtap {
    Mode(MacVtapMode),
    Flags(MacVtapFlags),
    MacAddrMode(MacVtapMacAddressMode),
    MacAddr([u8; 6]),
    MacAddrData(Vec<InfoMacVtap>),
    MacAddrCount(u32),
    BcQueueLen(u32),
    BcQueueLenUsed(u32),
    BcCutoff(i32),
    Other(DefaultNla),
}

impl Nla for InfoMacVtap {
    fn value_len(&self) -> usize {
        match self {
            Self::Mode(_) => 4,
            Self::Flags(_) => 2,
            Self::MacAddrMode(_) => 4,
            Self::MacAddr(_) => 6,
            Self::MacAddrData(ref nlas) => nlas.as_slice().buffer_len(),
            Self::MacAddrCount(_) => 4,
            Self::BcQueueLen(_) => 4,
            Self::BcQueueLenUsed(_) => 4,
            Self::BcCutoff(_) => 4,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Mode(value) => emit_u32(buffer, (*value).into()).unwrap(),
            Self::Flags(value) => emit_u16(buffer, value.bits()).unwrap(),
            Self::MacAddrMode(value) => {
                emit_u32(buffer, (*value).into()).unwrap()
            }
            Self::MacAddr(bytes) => buffer.copy_from_slice(bytes),
            Self::MacAddrData(ref nlas) => nlas.as_slice().emit(buffer),
            Self::MacAddrCount(value) => emit_u32(buffer, *value).unwrap(),
            Self::BcQueueLen(value) => emit_u32(buffer, *value).unwrap(),
            Self::BcQueueLenUsed(value) => emit_u32(buffer, *value).unwrap(),
            Self::BcCutoff(value) => emit_i32(buffer, *value).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Mode(_) => IFLA_MACVLAN_MODE,
            Self::Flags(_) => IFLA_MACVLAN_FLAGS,
            Self::MacAddrMode(_) => IFLA_MACVLAN_MACADDR_MODE,
            Self::MacAddr(_) => IFLA_MACVLAN_MACADDR,
            Self::MacAddrData(_) => IFLA_MACVLAN_MACADDR_DATA,
            Self::MacAddrCount(_) => IFLA_MACVLAN_MACADDR_COUNT,
            Self::BcQueueLen(_) => IFLA_MACVLAN_BC_QUEUE_LEN,
            Self::BcQueueLenUsed(_) => IFLA_MACVLAN_BC_QUEUE_LEN_USED,
            Self::BcCutoff(_) => IFLA_MACVLAN_BC_CUTOFF,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoMacVtap {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_MACVLAN_MODE => Self::Mode(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_MODE value")?
                    .into(),
            ),
            IFLA_MACVLAN_FLAGS => Self::Flags(MacVtapFlags::from_bits_retain(
                parse_u16(payload)
                    .context("invalid IFLA_MACVLAN_FLAGS value")?,
            )),
            IFLA_MACVLAN_MACADDR_MODE => Self::MacAddrMode(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_MACADDR_MODE value")?
                    .into(),
            ),
            IFLA_MACVLAN_MACADDR => Self::MacAddr(
                parse_mac(payload)
                    .context("invalid IFLA_MACVLAN_MACADDR value")?,
            ),
            IFLA_MACVLAN_MACADDR_DATA => {
                let mut mac_data = Vec::new();
                let err = "failed to parse IFLA_MACVLAN_MACADDR_DATA";
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(err)?;
                    let parsed = InfoMacVtap::parse(nla).context(err)?;
                    mac_data.push(parsed);
                }
                Self::MacAddrData(mac_data)
            }
            IFLA_MACVLAN_MACADDR_COUNT => Self::MacAddrCount(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_MACADDR_COUNT value")?,
            ),
            IFLA_MACVLAN_BC_QUEUE_LEN => Self::BcQueueLen(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_BC_QUEUE_LEN value")?,
            ),
            IFLA_MACVLAN_BC_QUEUE_LEN_USED => Self::BcQueueLenUsed(
                parse_u32(payload)
                    .context("invalid IFLA_MACVLAN_BC_QUEUE_LEN_USED value")?,
            ),
            IFLA_MACVLAN_BC_CUTOFF => Self::BcCutoff(
                parse_i32(payload)
                    .context("invalid IFLA_MACVLAN_BC_CUTOFF value")?,
            ),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(mac_vtap)"
            ))?),
        })
    }
}

const MACVLAN_MODE_PRIVATE: u32 = 1;
const MACVLAN_MODE_VEPA: u32 = 2;
const MACVLAN_MODE_BRIDGE: u32 = 4;
const MACVLAN_MODE_PASSTHRU: u32 = 8;
const MACVLAN_MODE_SOURCE: u32 = 16;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum MacVlanMode {
    Private,
    Vepa,
    Bridge,
    Passthrough,
    Source,
    Other(u32),
}

pub type MacVtapMode = MacVlanMode;
pub type MacVtapFlags = MacVlanFlags;

impl From<u32> for MacVlanMode {
    fn from(d: u32) -> Self {
        match d {
            MACVLAN_MODE_PRIVATE => Self::Private,
            MACVLAN_MODE_VEPA => Self::Vepa,
            MACVLAN_MODE_BRIDGE => Self::Bridge,
            MACVLAN_MODE_PASSTHRU => Self::Passthrough,
            MACVLAN_MODE_SOURCE => Self::Source,
            _ => {
                log::warn!("Unknown MAC VLAN mode {d}");
                Self::Other(d)
            }
        }
    }
}

impl From<MacVlanMode> for u32 {
    fn from(v: MacVlanMode) -> u32 {
        match v {
            MacVlanMode::Private => MACVLAN_MODE_PRIVATE,
            MacVlanMode::Vepa => MACVLAN_MODE_VEPA,
            MacVlanMode::Bridge => MACVLAN_MODE_BRIDGE,
            MacVlanMode::Passthrough => MACVLAN_MODE_PASSTHRU,
            MacVlanMode::Source => MACVLAN_MODE_SOURCE,
            MacVlanMode::Other(d) => d,
        }
    }
}

impl std::fmt::Display for MacVlanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Private => f.write_str("private"),
            Self::Vepa => f.write_str("vepa"),
            Self::Bridge => f.write_str("bridge"),
            Self::Passthrough => f.write_str("passthru"),
            Self::Source => f.write_str("source"),
            Self::Other(d) => write!(f, "{d}"),
        }
    }
}

impl std::str::FromStr for MacVlanMode {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "private" => Self::Private,
            "vepa" => Self::Vepa,
            "bridge" => Self::Bridge,
            "passthru" => Self::Passthrough,
            "source" => Self::Source,
            _ => {
                return Err(DecodeError::from(format!(
                    "unknown MAC VLAN mode: {s}"
                )))
            }
        })
    }
}

const MACVLAN_FLAG_NOPROMISC: u16 = 1;
const MACVLAN_FLAG_NODST: u16 = 2;

bitflags! {
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct MacVlanFlags: u16 {
        const NoPromisc = MACVLAN_FLAG_NOPROMISC;
        const NoDst = MACVLAN_FLAG_NODST;
        const _ = !0;
    }
}

const MACVLAN_MACADDR_ADD: u32 = 0;
const MACVLAN_MACADDR_DEL: u32 = 1;
const MACVLAN_MACADDR_FLUSH: u32 = 2;
const MACVLAN_MACADDR_SET: u32 = 3;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum MacVlanMacAddressMode {
    Add,
    Del,
    Flush,
    Set,
    Other(u32),
}

pub type MacVtapMacAddressMode = MacVlanMacAddressMode;

impl From<u32> for MacVlanMacAddressMode {
    fn from(d: u32) -> Self {
        match d {
            MACVLAN_MACADDR_ADD => Self::Add,
            MACVLAN_MACADDR_DEL => Self::Del,
            MACVLAN_MACADDR_FLUSH => Self::Flush,
            MACVLAN_MACADDR_SET => Self::Set,
            _ => Self::Other(d),
        }
    }
}

impl From<MacVlanMacAddressMode> for u32 {
    fn from(v: MacVlanMacAddressMode) -> u32 {
        match v {
            MacVlanMacAddressMode::Add => MACVLAN_MACADDR_ADD,
            MacVlanMacAddressMode::Del => MACVLAN_MACADDR_DEL,
            MacVlanMacAddressMode::Flush => MACVLAN_MACADDR_FLUSH,
            MacVlanMacAddressMode::Set => MACVLAN_MACADDR_SET,
            MacVlanMacAddressMode::Other(d) => d,
        }
    }
}
