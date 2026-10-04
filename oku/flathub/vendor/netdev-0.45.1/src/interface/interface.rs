use crate::interface::ipv6_addr_flags::Ipv6AddrFlags;
use crate::interface::state::OperState;
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::net::ip::{is_global_ip, is_global_ipv4, is_global_ipv6};
use crate::stats::counters::InterfaceStats;
use crate::{interface::types::InterfaceType, net::mac::MacAddr};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[cfg(feature = "gateway")]
use crate::net::device::NetworkDevice;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A network interface.
///
/// Values are collected from platform-specific system APIs.
/// Some metadata is optional.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Interface {
    /// OS-assigned index of network interface. This is an integer which uniquely identifies the interface
    /// on this machine.
    pub index: u32,
    /// System name of the interface.
    ///
    /// On Unix-like systems this is usually a BSD or kernel name such as `eth0`, `en0`, or
    /// `wlan0`. On Windows this is the adapter name GUID string.
    pub name: String,
    /// Human-readable interface name, when the platform provides one.
    ///
    /// Examples include `Wi-Fi` or `Ethernet` on Windows and display names on macOS.
    /// This field is commonly `None` on Linux, Android, iOS, and BSD systems.
    pub friendly_name: Option<String>,
    /// Adapter description, when the platform provides one.
    ///
    /// On Windows this is usually the adapter model or driver description.
    /// This field is generally `None` on non-Windows platforms.
    pub description: Option<String>,
    /// Interface classification.
    ///
    /// The value is derived from platform-specific type identifiers and may be
    /// `InterfaceType::Unknown` when the OS does not expose a recognizable type.
    pub if_type: InterfaceType,
    /// Link-layer address of the interface, when available.
    ///
    /// This field may be `None` for interfaces without a MAC address, for virtual interfaces,
    /// or on platforms that do not expose the address through the available APIs.
    pub mac_addr: Option<MacAddr>,
    /// IPv4 addresses assigned to the interface, including prefix length.
    ///
    /// The vector is empty when the interface has no IPv4 addresses or when they could not be read.
    pub ipv4: Vec<Ipv4Net>,
    /// IPv6 addresses assigned to the interface, including prefix length.
    ///
    /// The vector is empty when the interface has no IPv6 addresses or when they could not be read.
    pub ipv6: Vec<Ipv6Net>,
    /// IPv6 scope IDs aligned with entries in `Interface::ipv6`.
    ///
    /// Scope IDs are primarily relevant for link-local IPv6 addresses and may also be called
    /// zone indexes. A value can be `0` when no scope is needed or when the platform did not
    /// provide one.
    pub ipv6_scope_ids: Vec<u32>,
    /// Per-address IPv6 flags, aligned with entries in `Interface::ipv6`.
    pub ipv6_addr_flags: Vec<Ipv6AddrFlags>,
    /// Raw interface flags.
    ///
    /// Bit meanings are platform-specific.
    pub flags: u32,
    /// Operational state at the time the interface snapshot was collected.
    pub oper_state: OperState,
    /// Transmit link speed in bits per second.
    ///
    /// This field is usually available on Linux, Android, BSD, Apple targets, and Windows.
    /// It may be `None` for virtual adapters, unsupported drivers, or platforms that do not
    /// expose link speed.
    pub transmit_speed: Option<u64>,
    /// Reported receive link speed in bits per second.
    ///
    /// This field follows the same availability rules as `Interface::transmit_speed`.
    pub receive_speed: Option<u64>,
    /// Whether this interface was configured to auto-negotiate the link speed.
    ///
    /// It may `None` if reading this information has not been implemented for a specific OS.
    pub auto_negotiate: Option<bool>,
    /// Whether this interface is configured to use DHCP for IPv4.
    ///
    /// This may be `None` if reading this information is not available on the
    /// current platform, not implemented, or not applicable to the interface.
    pub dhcp_v4_enabled: Option<bool>,
    /// Whether this interface is configured to use DHCP for IPv6.
    ///
    /// This may be `None` if reading this information is not available on the
    /// current platform, not implemented, or not applicable to the interface.
    pub dhcp_v6_enabled: Option<bool>,
    /// Traffic counters captured when the interface snapshot was collected.
    ///
    /// The counters are cumulative totals reported by the OS, typically since boot.
    /// This field may be `None` when the current platform or adapter does not expose statistics.
    /// Use `Interface::update_stats` to refresh the snapshot in place.
    pub stats: Option<InterfaceStats>,
    /// Default gateway associated with this interface, when known.
    ///
    /// This field is available only with the `gateway` feature. It may be `None` when the
    /// interface is not the default route, when the gateway has no link-layer address available,
    /// or when the platform cannot resolve gateway information.
    #[cfg(feature = "gateway")]
    pub gateway: Option<NetworkDevice>,
    /// DNS resolver addresses associated with this interface.
    ///
    /// This field is available only with the `gateway` feature.
    #[cfg(feature = "gateway")]
    pub dns_servers: Vec<IpAddr>,
    /// Maximum transmission unit in bytes, when available.
    ///
    /// This field may be `None` when the platform API does not provide the MTU or the lookup fails.
    pub mtu: Option<u32>,
    /// Whether this interface was identified as the default route.
    ///
    /// This field is available only with the `gateway` feature.
    #[cfg(feature = "gateway")]
    pub default: bool,
}

impl Interface {
    /// Returns the interface currently selected as the system default route.
    #[cfg(feature = "gateway")]
    pub fn default() -> Result<Interface, String> {
        super::resolve_default_interface(super::interfaces())
    }
    /// Returns an empty placeholder interface.
    ///
    /// This constructor is mainly useful for tests and internal assembly of interface data.
    pub fn dummy() -> Interface {
        Interface {
            index: 0,
            name: String::new(),
            friendly_name: None,
            description: None,
            if_type: InterfaceType::Unknown,
            mac_addr: None,
            ipv4: Vec::new(),
            ipv6: Vec::new(),
            ipv6_scope_ids: Vec::new(),
            ipv6_addr_flags: Vec::new(),
            flags: 0,
            oper_state: OperState::Unknown,
            transmit_speed: None,
            receive_speed: None,
            auto_negotiate: None,
            dhcp_v4_enabled: None,
            dhcp_v6_enabled: None,
            stats: None,
            #[cfg(feature = "gateway")]
            gateway: None,
            #[cfg(feature = "gateway")]
            dns_servers: Vec::new(),
            mtu: None,
            #[cfg(feature = "gateway")]
            default: false,
        }
    }
    /// Returns `true` when the interface has the OS `UP` flag set.
    pub fn is_up(&self) -> bool {
        self.flags & (super::flags::IFF_UP as u32) != 0
    }
    /// Returns `true` when the interface is marked as loopback.
    pub fn is_loopback(&self) -> bool {
        self.flags & (super::flags::IFF_LOOPBACK as u32) != 0
    }
    /// Returns `true` when the interface is marked as point-to-point.
    pub fn is_point_to_point(&self) -> bool {
        self.flags & (super::flags::IFF_POINTOPOINT as u32) != 0
    }
    /// Returns `true` when the interface supports multicast according to its flags.
    pub fn is_multicast(&self) -> bool {
        self.flags & (super::flags::IFF_MULTICAST as u32) != 0
    }
    /// Returns `true` when the interface supports broadcast according to its flags.
    pub fn is_broadcast(&self) -> bool {
        self.flags & (super::flags::IFF_BROADCAST as u32) != 0
    }
    /// Returns `true` for interfaces that look like TUN-style point-to-point devices.
    ///
    /// This is a heuristic based on interface flags and is not guaranteed to identify every
    /// virtual tunnel interface on every platform.
    pub fn is_tun(&self) -> bool {
        self.is_up() && self.is_point_to_point() && !self.is_broadcast() && !self.is_loopback()
    }
    /// Returns `true` when the platform reports the interface as able to pass traffic.
    ///
    /// The exact definition depends on the operating system.
    pub fn is_running(&self) -> bool {
        super::flags::is_running(&self)
    }
    /// Returns `true` when the interface appears to be backed by physical hardware.
    pub fn is_physical(&self) -> bool {
        use crate::net::db::oui;
        super::flags::is_physical_interface(&self)
            && !oui::is_virtual_mac(&self.mac_addr.unwrap_or(MacAddr::zero()))
            && !oui::is_known_loopback_mac(&self.mac_addr.unwrap_or(MacAddr::zero()))
    }
    /// Returns the cached operational state.
    pub fn oper_state(&self) -> OperState {
        self.oper_state
    }
    /// Returns `true` when `Interface::oper_state` is `OperState::Up`.
    pub fn is_oper_up(&self) -> bool {
        self.oper_state == OperState::Up
    }
    /// Refreshes `Interface::oper_state` from the operating system.
    pub fn update_oper_state(&mut self) {
        self.oper_state = super::state::operstate(&self.name);
    }
    /// Returns the IPv4 addresses assigned to this interface.
    ///
    /// Prefix lengths are discarded. Use `Interface::ipv4` when the network prefix is needed.
    pub fn ipv4_addrs(&self) -> Vec<Ipv4Addr> {
        self.ipv4.iter().map(|net| net.addr()).collect()
    }
    /// Returns the IPv6 host addresses assigned to this interface.
    ///
    /// Prefix lengths are discarded. Use `Interface::ipv6` when the network prefix is needed.
    pub fn ipv6_addrs(&self) -> Vec<Ipv6Addr> {
        self.ipv6.iter().map(|net| net.addr()).collect()
    }
    /// Returns all IPv4 and IPv6 host addresses assigned to this interface.
    pub fn ip_addrs(&self) -> Vec<IpAddr> {
        self.ipv4_addrs()
            .into_iter()
            .map(IpAddr::V4)
            .chain(self.ipv6_addrs().into_iter().map(IpAddr::V6))
            .collect()
    }
    /// Returns `true` when at least one IPv4 address is present.
    pub fn has_ipv4(&self) -> bool {
        !self.ipv4.is_empty()
    }
    /// Returns `true` when at least one IPv6 address is present.
    pub fn has_ipv6(&self) -> bool {
        !self.ipv6.is_empty()
    }
    /// Returns `true` when at least one assigned IPv4 address appears globally routable.
    pub fn has_global_ipv4(&self) -> bool {
        self.ipv4_addrs().iter().any(|ip| is_global_ipv4(ip))
    }
    /// Returns `true` when at least one assigned IPv6 address appears globally routable.
    pub fn has_global_ipv6(&self) -> bool {
        self.ipv6_addrs().iter().any(|ip| is_global_ipv6(ip))
    }
    /// Returns `true` when at least one assigned IPv4 or IPv6 address appears globally routable.
    pub fn has_global_ip(&self) -> bool {
        self.ip_addrs().iter().any(|ip| is_global_ip(ip))
    }
    /// Returns IPv4 addresses that appear globally routable.
    pub fn global_ipv4_addrs(&self) -> Vec<Ipv4Addr> {
        self.ipv4_addrs()
            .into_iter()
            .filter(|ip| is_global_ipv4(ip))
            .collect()
    }
    /// Returns IPv6 addresses that appear globally routable.
    pub fn global_ipv6_addrs(&self) -> Vec<Ipv6Addr> {
        self.ipv6_addrs()
            .into_iter()
            .filter(|ip| is_global_ipv6(ip))
            .collect()
    }
    /// Returns IPv4 and IPv6 addresses that appear globally routable.
    pub fn global_ip_addrs(&self) -> Vec<IpAddr> {
        self.ip_addrs()
            .into_iter()
            .filter(|ip| is_global_ip(ip))
            .collect()
    }
    /// Refreshes `Interface::stats` for this interface.
    ///
    /// On supported platforms this updates the byte counters and timestamp with a new snapshot.
    pub fn update_stats(&mut self) -> std::io::Result<()> {
        crate::stats::counters::update_interface_stats(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::interface::interface::Interface;
    use ipnet::{Ipv4Net, Ipv6Net};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn dummy_initializes_dhcp_state_as_unknown() {
        assert_eq!(Interface::dummy().dhcp_v4_enabled, None);
        assert_eq!(Interface::dummy().dhcp_v6_enabled, None);
    }

    #[test]
    fn global_helpers_filter() {
        let mut itf = Interface::dummy();
        itf.ipv4 = vec![
            Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 1), 8).unwrap(), // private
            Ipv4Net::new(Ipv4Addr::new(1, 1, 1, 1), 32).unwrap(), // global
        ];
        itf.ipv6 = vec![
            Ipv6Net::new(Ipv6Addr::LOCALHOST, 128).unwrap(), // loopback
            Ipv6Net::new("2606:4700:4700::1111".parse().unwrap(), 128).unwrap(), // global
        ];

        // Check global_ip_addrs() fillters correctly
        let globals = itf.global_ip_addrs();
        assert!(globals.contains(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        assert!(globals.contains(&IpAddr::V6("2606:4700:4700::1111".parse().unwrap())));
        assert!(!globals.contains(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(!globals.contains(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }
}
