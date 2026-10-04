// SPDX-License-Identifier: MIT

const IF_LINK_MODE_DEFAULT: u8 = 0;
const IF_LINK_MODE_DORMANT: u8 = 1;
const IF_LINK_MODE_TESTING: u8 = 2;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[non_exhaustive]
pub enum LinkMode {
    #[default]
    Default,
    ///limit upward transition to dormant
    Dormant,
    ///limit upward transition to testing
    Testing,
    Other(u8),
}

impl From<u8> for LinkMode {
    fn from(d: u8) -> Self {
        match d {
            IF_LINK_MODE_DEFAULT => Self::Default,
            IF_LINK_MODE_DORMANT => Self::Dormant,
            IF_LINK_MODE_TESTING => Self::Testing,
            _ => Self::Other(d),
        }
    }
}

impl From<LinkMode> for u8 {
    fn from(v: LinkMode) -> u8 {
        match v {
            LinkMode::Default => IF_LINK_MODE_DEFAULT,
            LinkMode::Dormant => IF_LINK_MODE_DORMANT,
            LinkMode::Testing => IF_LINK_MODE_TESTING,
            LinkMode::Other(d) => d,
        }
    }
}

impl std::fmt::Display for LinkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "DEFAULT"),
            Self::Dormant => write!(f, "DORMANT"),
            Self::Testing => write!(f, "TESTING"),
            Self::Other(d) => write!(f, "UNKNOWN:{d}"),
        }
    }
}
