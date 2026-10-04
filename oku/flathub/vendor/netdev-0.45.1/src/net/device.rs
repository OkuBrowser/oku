#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::mac::MacAddr;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Address information for a related network device.
///
/// This type is currently used for devices associated with an interface, such as a default
/// gateway.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NetworkDevice {
    /// Link-layer address of the device.
    ///
    /// When the MAC address cannot be resolved, this field may contain the zero address.
    pub mac_addr: MacAddr,
    /// IPv4 addresses associated with the device.
    pub ipv4: Vec<Ipv4Addr>,
    /// IPv6 addresses associated with the device.
    pub ipv6: Vec<Ipv6Addr>,
}

impl NetworkDevice {
    /// Creates an empty device record.
    pub fn new() -> NetworkDevice {
        NetworkDevice {
            mac_addr: MacAddr::zero(),
            ipv4: Vec::new(),
            ipv6: Vec::new(),
        }
    }
}
