use std::convert::TryFrom;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Cross-platform classification of a network interface.
///
/// The variants normalize platform-specific type identifiers.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InterfaceType {
    /// Interface type could not be determined.
    Unknown,
    /// Ethernet interface.
    Ethernet,
    /// Token Ring interface.
    TokenRing,
    /// Fiber Distributed Data Interface (FDDI).
    Fddi,
    /// Basic-rate ISDN interface.
    BasicIsdn,
    /// Primary-rate ISDN interface.
    PrimaryIsdn,
    /// Point-to-Point Protocol (PPP) interface.
    Ppp,
    /// Loopback interface.
    Loopback,
    /// Legacy 3 Mbps Ethernet interface.
    Ethernet3Megabit,
    /// Serial Line Internet Protocol (SLIP) interface.
    Slip,
    /// Asynchronous Transfer Mode (ATM) interface.
    Atm,
    /// Generic modem interface.
    GenericModem,
    /// Proprietary virtual or internal interface.
    ProprietaryVirtual,
    /// Fast Ethernet over twisted pair.
    FastEthernetT,
    /// ISDN/X.25 interface.
    Isdn,
    /// Fast Ethernet over fiber.
    FastEthernetFx,
    /// IEEE 802.11 wireless LAN interface.
    Wireless80211,
    /// Asymmetric DSL interface.
    AsymmetricDsl,
    /// Rate-adaptive DSL interface.
    RateAdaptDsl,
    /// Symmetric DSL interface.
    SymmetricDsl,
    /// Very-high-bit-rate DSL interface.
    VeryHighSpeedDsl,
    /// IP over ATM interface.
    IPOverAtm,
    /// Gigabit Ethernet interface.
    GigabitEthernet,
    /// Tunnel interface.
    Tunnel,
    /// Multirate symmetric DSL interface.
    MultiRateSymmetricDsl,
    /// High-performance serial bus interface.
    HighPerformanceSerialBus,
    /// Mobile broadband interface for WiMAX devices.
    Wman,
    /// Wireless wide area network interface, such as a mobile broadband or cellular modem.
    Wwan,
    /// Mobile broadband interface for GSM-based devices.
    Wwanpp,
    /// Mobile broadband interface for CDMA-based devices.
    Wwanpp2,
    /// Transparent bridge interface.
    Bridge,
    /// Controller Area Network (CAN) interface.
    Can,
    /// Peer-to-peer wireless interface, such as Wi-Fi Direct or AWDL.
    PeerToPeerWireless,
    /// Unrecognized platform-specific type value.
    UnknownWithValue(u32),
}

impl InterfaceType {
    /// Returns the native numeric type identifier for the current target platform.
    ///
    /// For variants that have no direct mapping on the current platform, this method returns
    /// `u32::MAX`.
    #[cfg(target_os = "windows")]
    pub fn value(&self) -> u32 {
        match *self {
            InterfaceType::Unknown => 1,
            InterfaceType::Ethernet => 6,
            InterfaceType::TokenRing => 9,
            InterfaceType::Fddi => 15,
            InterfaceType::BasicIsdn => 20,
            InterfaceType::PrimaryIsdn => 21,
            InterfaceType::Ppp => 23,
            InterfaceType::Loopback => 24,
            InterfaceType::Ethernet3Megabit => 26,
            InterfaceType::Slip => 28,
            InterfaceType::Atm => 37,
            InterfaceType::GenericModem => 48,
            InterfaceType::ProprietaryVirtual => 53,
            InterfaceType::FastEthernetT => 62,
            InterfaceType::Isdn => 63,
            InterfaceType::FastEthernetFx => 69,
            InterfaceType::Wireless80211 => 71,
            InterfaceType::AsymmetricDsl => 94,
            InterfaceType::RateAdaptDsl => 95,
            InterfaceType::SymmetricDsl => 96,
            InterfaceType::VeryHighSpeedDsl => 97,
            InterfaceType::IPOverAtm => 114,
            InterfaceType::GigabitEthernet => 117,
            InterfaceType::Tunnel => 131,
            InterfaceType::MultiRateSymmetricDsl => 143,
            InterfaceType::HighPerformanceSerialBus => 144,
            InterfaceType::Wman => 237,
            InterfaceType::Wwanpp => 243,
            InterfaceType::Wwanpp2 => 244,
            InterfaceType::UnknownWithValue(v) => v,
            _ => u32::MAX,
        }
    }
    /// Returns the native numeric type identifier for the current target platform.
    ///
    /// For variants that have no direct mapping on the current platform, this method returns
    /// `u32::MAX`.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn value(&self) -> u32 {
        use crate::os::linux::arp;

        match *self {
            InterfaceType::Ethernet => arp::ARPHRD_ETHER,
            InterfaceType::TokenRing => arp::ARPHRD_IEEE802,
            InterfaceType::Fddi => arp::ARPHRD_FDDI,
            InterfaceType::Ppp => arp::ARPHRD_PPP,
            InterfaceType::Loopback => arp::ARPHRD_LOOPBACK,
            InterfaceType::Ethernet3Megabit => arp::ARPHRD_EETHER,
            InterfaceType::Slip => arp::ARPHRD_SLIP,
            InterfaceType::Atm => arp::ARPHRD_ATM,
            InterfaceType::Wireless80211 => arp::ARPHRD_IEEE80211,
            InterfaceType::Tunnel => arp::ARPHRD_TUNNEL,
            InterfaceType::Isdn => arp::ARPHRD_X25,
            InterfaceType::HighPerformanceSerialBus => arp::ARPHRD_IEEE1394,
            InterfaceType::Can => arp::ARPHRD_CAN,
            InterfaceType::UnknownWithValue(v) => v,
            _ => u32::MAX,
        }
    }
    /// Returns the native numeric type identifier for the current target platform.
    ///
    /// For variants that have no direct mapping on the current platform, this method returns
    /// `u32::MAX`.
    #[cfg(any(
        target_vendor = "apple",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    pub fn value(&self) -> u32 {
        match *self {
            InterfaceType::Ethernet => 0x6,
            InterfaceType::TokenRing => 0x9,
            InterfaceType::Fddi => 0xf,
            InterfaceType::Isdn => 0x14,
            InterfaceType::PrimaryIsdn => 0x15,
            InterfaceType::Ppp => 0x17,
            InterfaceType::Loopback => 0x18,
            InterfaceType::Ethernet3Megabit => 0x1a,
            InterfaceType::Slip => 0x1c,
            InterfaceType::Atm => 0x25,
            InterfaceType::GenericModem => 0x30,
            InterfaceType::Wireless80211 => 0x47,
            InterfaceType::AsymmetricDsl => 0x95,
            InterfaceType::RateAdaptDsl => 0x96,
            InterfaceType::SymmetricDsl => 0x97,
            InterfaceType::IPOverAtm => 0x31,
            InterfaceType::HighPerformanceSerialBus => 0x90,
            InterfaceType::UnknownWithValue(v) => v,
            _ => u32::MAX,
        }
    }
    /// Returns a human-readable name for the interface type.
    pub fn name(&self) -> String {
        match *self {
            InterfaceType::Unknown => String::from("Unknown"),
            InterfaceType::Ethernet => String::from("Ethernet"),
            InterfaceType::TokenRing => String::from("Token Ring"),
            InterfaceType::Fddi => String::from("FDDI"),
            InterfaceType::BasicIsdn => String::from("Basic ISDN"),
            InterfaceType::PrimaryIsdn => String::from("Primary ISDN"),
            InterfaceType::Ppp => String::from("PPP"),
            InterfaceType::Loopback => String::from("Loopback"),
            InterfaceType::Ethernet3Megabit => String::from("Ethernet 3 megabit"),
            InterfaceType::Slip => String::from("SLIP"),
            InterfaceType::Atm => String::from("ATM"),
            InterfaceType::GenericModem => String::from("Generic Modem"),
            InterfaceType::ProprietaryVirtual => String::from("Proprietary Virtual/Internal"),
            InterfaceType::FastEthernetT => String::from("Fast Ethernet T"),
            InterfaceType::Isdn => String::from("ISDN"),
            InterfaceType::FastEthernetFx => String::from("Fast Ethernet FX"),
            InterfaceType::Wireless80211 => String::from("Wireless IEEE 802.11"),
            InterfaceType::AsymmetricDsl => String::from("Asymmetric DSL"),
            InterfaceType::RateAdaptDsl => String::from("Rate Adaptive DSL"),
            InterfaceType::SymmetricDsl => String::from("Symmetric DSL"),
            InterfaceType::VeryHighSpeedDsl => String::from("Very High Data Rate DSL"),
            InterfaceType::IPOverAtm => String::from("IP over ATM"),
            InterfaceType::GigabitEthernet => String::from("Gigabit Ethernet"),
            InterfaceType::Tunnel => String::from("Tunnel"),
            InterfaceType::MultiRateSymmetricDsl => String::from("Multi-Rate Symmetric DSL"),
            InterfaceType::HighPerformanceSerialBus => String::from("High Performance Serial Bus"),
            InterfaceType::Bridge => String::from("Bridge"),
            InterfaceType::Wman => String::from("WMAN"),
            InterfaceType::Wwan => String::from("WWAN"),
            InterfaceType::Wwanpp => String::from("WWANPP"),
            InterfaceType::Wwanpp2 => String::from("WWANPP2"),
            InterfaceType::Can => String::from("CAN"),
            InterfaceType::PeerToPeerWireless => String::from("Peer-to-Peer Wireless"),
            InterfaceType::UnknownWithValue(v) => format!("Unknown ({})", v),
        }
    }
}

impl TryFrom<u32> for InterfaceType {
    type Error = ();
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == InterfaceType::Unknown.value() => Ok(InterfaceType::Unknown),
            x if x == InterfaceType::Ethernet.value() => Ok(InterfaceType::Ethernet),
            x if x == InterfaceType::TokenRing.value() => Ok(InterfaceType::TokenRing),
            x if x == InterfaceType::Fddi.value() => Ok(InterfaceType::Fddi),
            x if x == InterfaceType::BasicIsdn.value() => Ok(InterfaceType::BasicIsdn),
            x if x == InterfaceType::PrimaryIsdn.value() => Ok(InterfaceType::PrimaryIsdn),
            x if x == InterfaceType::Ppp.value() => Ok(InterfaceType::Ppp),
            x if x == InterfaceType::Loopback.value() => Ok(InterfaceType::Loopback),
            x if x == InterfaceType::Ethernet3Megabit.value() => {
                Ok(InterfaceType::Ethernet3Megabit)
            }
            x if x == InterfaceType::Slip.value() => Ok(InterfaceType::Slip),
            x if x == InterfaceType::Atm.value() => Ok(InterfaceType::Atm),
            x if x == InterfaceType::GenericModem.value() => Ok(InterfaceType::GenericModem),
            x if x == InterfaceType::ProprietaryVirtual.value() => {
                Ok(InterfaceType::ProprietaryVirtual)
            }
            x if x == InterfaceType::FastEthernetT.value() => Ok(InterfaceType::FastEthernetT),
            x if x == InterfaceType::Isdn.value() => Ok(InterfaceType::Isdn),
            x if x == InterfaceType::FastEthernetFx.value() => Ok(InterfaceType::FastEthernetFx),
            x if x == InterfaceType::Wireless80211.value() => Ok(InterfaceType::Wireless80211),
            x if x == InterfaceType::AsymmetricDsl.value() => Ok(InterfaceType::AsymmetricDsl),
            x if x == InterfaceType::RateAdaptDsl.value() => Ok(InterfaceType::RateAdaptDsl),
            x if x == InterfaceType::SymmetricDsl.value() => Ok(InterfaceType::SymmetricDsl),
            x if x == InterfaceType::VeryHighSpeedDsl.value() => {
                Ok(InterfaceType::VeryHighSpeedDsl)
            }
            x if x == InterfaceType::IPOverAtm.value() => Ok(InterfaceType::IPOverAtm),
            x if x == InterfaceType::GigabitEthernet.value() => Ok(InterfaceType::GigabitEthernet),
            x if x == InterfaceType::Tunnel.value() => Ok(InterfaceType::Tunnel),
            x if x == InterfaceType::MultiRateSymmetricDsl.value() => {
                Ok(InterfaceType::MultiRateSymmetricDsl)
            }
            x if x == InterfaceType::HighPerformanceSerialBus.value() => {
                Ok(InterfaceType::HighPerformanceSerialBus)
            }
            x if x == InterfaceType::Wman.value() => Ok(InterfaceType::Wman),
            x if x == InterfaceType::Wwanpp.value() => Ok(InterfaceType::Wwanpp),
            x if x == InterfaceType::Wwanpp2.value() => Ok(InterfaceType::Wwanpp2),
            x if x == InterfaceType::Can.value() => Ok(InterfaceType::Can),
            _ => Ok(InterfaceType::UnknownWithValue(v)),
        }
    }
}
