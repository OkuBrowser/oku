use std::fmt;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Operational state of a network interface.
///
/// This enum models the common states used by Linux and maps equivalent states from
/// other platforms when possible.
///
/// See also: <https://www.kernel.org/doc/Documentation/networking/operstates.txt>
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OperState {
    /// Interface state could not be determined.
    Unknown,
    /// Interface is not present.
    NotPresent,
    /// Interface is administratively or otherwise down.
    Down,
    /// Interface is down because a lower layer is down.
    LowerLayerDown,
    /// Interface is in testing state.
    Testing,
    /// Interface is dormant.
    Dormant,
    /// Interface is operational.
    Up,
}

impl OperState {
    /// Returns the lowercase representation used by Linux `operstate` files.
    pub fn as_str(&self) -> &'static str {
        match self {
            OperState::Unknown => "unknown",
            OperState::NotPresent => "notpresent",
            OperState::Down => "down",
            OperState::LowerLayerDown => "lowerlayerdown",
            OperState::Testing => "testing",
            OperState::Dormant => "dormant",
            OperState::Up => "up",
        }
    }

    /// Derives an operational state from raw interface flags.
    ///
    /// This is used as a fallback when no dedicated operstate API is available.
    /// The result is necessarily less precise than native platform state reporting.
    pub fn from_if_flags(if_flags: u32) -> Self {
        #[cfg(not(target_os = "windows"))]
        {
            if if_flags & super::flags::IFF_UP as u32 != 0 {
                if if_flags & super::flags::IFF_RUNNING as u32 != 0 {
                    OperState::Up
                } else {
                    OperState::Dormant
                }
            } else {
                OperState::Down
            }
        }

        #[cfg(target_os = "windows")]
        {
            if if_flags & super::flags::IFF_UP as u32 != 0 {
                OperState::Up
            } else {
                OperState::Down
            }
        }
    }
}

impl fmt::Display for OperState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for OperState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unknown" => Ok(OperState::Unknown),
            "notpresent" => Ok(OperState::NotPresent),
            "down" => Ok(OperState::Down),
            "lowerlayerdown" => Ok(OperState::LowerLayerDown),
            "testing" => Ok(OperState::Testing),
            "dormant" => Ok(OperState::Dormant),
            "up" => Ok(OperState::Up),
            _ => Err(()),
        }
    }
}

/// Reads the current operational state for the named interface.
///
/// This function performs a fresh OS query and does not rely on a previously collected
/// snapshot.
pub fn operstate(if_name: &str) -> OperState {
    #[cfg(target_os = "linux")]
    {
        crate::os::linux::state::operstate(if_name)
    }
    #[cfg(target_os = "android")]
    {
        crate::os::android::state::operstate(if_name)
    }
    #[cfg(target_vendor = "apple")]
    {
        crate::os::darwin::state::operstate(if_name)
    }
    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    {
        crate::os::bsd::state::operstate(if_name)
    }
    #[cfg(target_os = "windows")]
    {
        crate::os::windows::state::operstate(if_name)
    }
}
