// SPDX-License-Identifier: MIT

use std::fmt::Display;

const IN6_ADDR_GEN_MODE_EUI64: u8 = 0;
const IN6_ADDR_GEN_MODE_NONE: u8 = 1;
const IN6_ADDR_GEN_MODE_STABLE_PRIVACY: u8 = 2;
const IN6_ADDR_GEN_MODE_RANDOM: u8 = 3;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum In6AddrGenMode {
    #[default]
    Eui64,
    None,
    StablePrivacy,
    Random,
    Other(u8),
}

impl From<u8> for In6AddrGenMode {
    fn from(d: u8) -> Self {
        match d {
            IN6_ADDR_GEN_MODE_EUI64 => Self::Eui64,
            IN6_ADDR_GEN_MODE_NONE => Self::None,
            IN6_ADDR_GEN_MODE_STABLE_PRIVACY => Self::StablePrivacy,
            IN6_ADDR_GEN_MODE_RANDOM => Self::Random,
            _ => Self::Other(d),
        }
    }
}
impl From<&In6AddrGenMode> for u8 {
    fn from(v: &In6AddrGenMode) -> Self {
        match v {
            In6AddrGenMode::Eui64 => IN6_ADDR_GEN_MODE_EUI64,
            In6AddrGenMode::None => IN6_ADDR_GEN_MODE_NONE,
            In6AddrGenMode::StablePrivacy => IN6_ADDR_GEN_MODE_STABLE_PRIVACY,
            In6AddrGenMode::Random => IN6_ADDR_GEN_MODE_RANDOM,
            In6AddrGenMode::Other(d) => *d,
        }
    }
}

impl Display for In6AddrGenMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eui64 => write!(f, "eui64"),
            Self::None => write!(f, "none"),
            // https://github.com/iproute2/iproute2/blob/afbfd2f2b0a633d068990775f8e1b73b8ee83733/ip/ipaddress.c#L325-L329
            Self::StablePrivacy => write!(f, "stable_secret"),
            Self::Random => write!(f, "random"),
            Self::Other(d) => write!(f, "other({d})"),
        }
    }
}
