// SPDX-License-Identifier: MIT

use std::fmt;

use netlink_packet_core::DecodeError;

const IF_OPER_UNKNOWN: u8 = 0;
const IF_OPER_NOTPRESENT: u8 = 1;
const IF_OPER_DOWN: u8 = 2;
const IF_OPER_LOWERLAYERDOWN: u8 = 3;
const IF_OPER_TESTING: u8 = 4;
const IF_OPER_DORMANT: u8 = 5;
const IF_OPER_UP: u8 = 6;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum State {
    /// Status can't be determined
    Unknown,
    /// Some component is missing
    NotPresent,
    /// Down
    Down,
    /// Down due to state of lower layer
    LowerLayerDown,
    /// In some test mode
    Testing,
    /// Not up but pending an external event
    Dormant,
    /// Up, ready to send packets
    Up,
    /// Place holder for new state introduced by kernel when current crate does
    /// not support so.
    Other(u8),
}

impl From<u8> for State {
    fn from(value: u8) -> Self {
        match value {
            IF_OPER_UNKNOWN => Self::Unknown,
            IF_OPER_NOTPRESENT => Self::NotPresent,
            IF_OPER_DOWN => Self::Down,
            IF_OPER_LOWERLAYERDOWN => Self::LowerLayerDown,
            IF_OPER_TESTING => Self::Testing,
            IF_OPER_DORMANT => Self::Dormant,
            IF_OPER_UP => Self::Up,
            _ => Self::Other(value),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "UNKNOWN"),
            Self::NotPresent => write!(f, "NOTPRESENT"),
            Self::Down => write!(f, "DOWN"),
            Self::LowerLayerDown => write!(f, "LOWERLAYERDOWN"),
            Self::Testing => write!(f, "TESTING"),
            Self::Dormant => write!(f, "DORMANT"),
            Self::Up => write!(f, "UP"),
            Self::Other(v) => write!(f, "{v}"),
        }
    }
}

impl std::str::FromStr for State {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s.eq_ignore_ascii_case("unknown") => Ok(Self::Unknown),
            s if s.eq_ignore_ascii_case("notpresent") => Ok(Self::NotPresent),
            s if s.eq_ignore_ascii_case("down") => Ok(Self::Down),
            s if s.eq_ignore_ascii_case("lowerlayerdown") => {
                Ok(Self::LowerLayerDown)
            }
            s if s.eq_ignore_ascii_case("testing") => Ok(Self::Testing),
            s if s.eq_ignore_ascii_case("dormant") => Ok(Self::Dormant),
            s if s.eq_ignore_ascii_case("up") => Ok(Self::Up),
            _ => Err(format!("Invalid operstate: {s}").into()),
        }
    }
}

impl From<State> for u8 {
    fn from(value: State) -> Self {
        match value {
            State::Unknown => IF_OPER_UNKNOWN,
            State::NotPresent => IF_OPER_NOTPRESENT,
            State::Down => IF_OPER_DOWN,
            State::LowerLayerDown => IF_OPER_LOWERLAYERDOWN,
            State::Testing => IF_OPER_TESTING,
            State::Dormant => IF_OPER_DORMANT,
            State::Up => IF_OPER_UP,
            State::Other(other) => other,
        }
    }
}
