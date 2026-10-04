// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u16, parse_u16, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    Parseable,
};

const IFLA_IPVLAN_MODE: u16 = 1;
const IFLA_IPVLAN_FLAGS: u16 = 2;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoIpVlan {
    Mode(IpVlanMode),
    Flags(IpVlanFlags),
    Other(DefaultNla),
}

impl Nla for InfoIpVlan {
    fn value_len(&self) -> usize {
        match self {
            Self::Mode(_) | Self::Flags(_) => 2,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Mode(value) => emit_u16(buffer, (*value).into()).unwrap(),
            Self::Flags(f) => emit_u16(buffer, f.bits()).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Mode(_) => IFLA_IPVLAN_MODE,
            Self::Flags(_) => IFLA_IPVLAN_FLAGS,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoIpVlan {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_IPVLAN_MODE => Self::Mode(
                parse_u16(payload)
                    .context("invalid IFLA_IPVLAN_MODE value")?
                    .into(),
            ),
            IFLA_IPVLAN_FLAGS => Self::Flags(IpVlanFlags::from_bits_retain(
                parse_u16(payload)
                    .context("failed to parse IFLA_IPVLAN_FLAGS")?,
            )),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(ipvlan)"
            ))?),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoIpVtap {
    Mode(IpVtapMode),
    Flags(IpVtapFlags),
    Other(DefaultNla),
}

impl Nla for InfoIpVtap {
    fn value_len(&self) -> usize {
        match self {
            Self::Mode(_) | Self::Flags(_) => 2,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Mode(value) => emit_u16(buffer, (*value).into()).unwrap(),
            Self::Flags(f) => emit_u16(buffer, f.bits()).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Mode(_) => IFLA_IPVLAN_MODE,
            Self::Flags(_) => IFLA_IPVLAN_FLAGS,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoIpVtap {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_IPVLAN_MODE => Self::Mode(
                parse_u16(payload)
                    .context("invalid IFLA_IPVLAN_MODE value")?
                    .into(),
            ),
            IFLA_IPVLAN_FLAGS => Self::Flags(IpVtapFlags::from_bits_retain(
                parse_u16(payload)
                    .context("failed to parse IFLA_IPVLAN_FLAGS")?,
            )),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(ipvlan)"
            ))?),
        })
    }
}

const IPVLAN_MODE_L2: u16 = 0;
const IPVLAN_MODE_L3: u16 = 1;
const IPVLAN_MODE_L3S: u16 = 2;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum IpVlanMode {
    L2,
    L3,
    L3S,
    Other(u16),
}

pub type IpVtapMode = IpVlanMode;

impl From<u16> for IpVlanMode {
    fn from(d: u16) -> Self {
        match d {
            IPVLAN_MODE_L2 => Self::L2,
            IPVLAN_MODE_L3 => Self::L3,
            IPVLAN_MODE_L3S => Self::L3S,
            _ => {
                log::warn!("Unknown IP VLAN mode {d}");
                Self::Other(d)
            }
        }
    }
}

impl From<IpVlanMode> for u16 {
    fn from(v: IpVlanMode) -> u16 {
        match v {
            IpVlanMode::L2 => IPVLAN_MODE_L2,
            IpVlanMode::L3 => IPVLAN_MODE_L3,
            IpVlanMode::L3S => IPVLAN_MODE_L3S,
            IpVlanMode::Other(d) => d,
        }
    }
}

impl std::fmt::Display for IpVlanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::L2 => f.write_str("l2"),
            Self::L3 => f.write_str("l3"),
            Self::L3S => f.write_str("l3s"),
            Self::Other(d) => write!(f, "{d}"),
        }
    }
}

impl std::str::FromStr for IpVlanMode {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "l2" => Self::L2,
            "l3" => Self::L3,
            "l3s" => Self::L3S,
            _ => {
                return Err(DecodeError::from(format!(
                    "unknown IP VLAN mode: {s}"
                )))
            }
        })
    }
}

const IPVLAN_F_PRIVATE: u16 = 0x01;
const IPVLAN_F_VEPA: u16 = 0x02;

bitflags! {
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct IpVlanFlags: u16 {
        const Private = IPVLAN_F_PRIVATE;
        const Vepa = IPVLAN_F_VEPA;
        const _ = !0;
    }
}

impl Default for IpVlanFlags {
    fn default() -> Self {
        Self::empty()
    }
}

pub type IpVtapFlags = IpVlanFlags;
