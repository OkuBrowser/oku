use crate::net::mac::MacAddr;

/// Known MAC address prefixes commonly assigned to virtualized environments.
pub const KNOWN_VM_MAC_PREFIXES: &[&str] = &[
    "00:05:69", // VMware
    "00:0C:29", // VMware
    "00:1C:14", // VMware
    "00:50:56", // VMware
    "00:03:FF", // Microsoft Hyper-V
    "00:1C:42", // Parallels Desktop
    "00:0F:4B", // Virtual Iron 4
    "00:16:3E", // Xen or Oracle VM
    "08:00:27", // VirtualBox
    "02:42:AC", // Docker Container
];

/// Known MAC addresses used by loopback-style adapters.
pub const KNOWN_LOOPBACK_MAC_ADDRESSES: &[&str] = &[
    "00:00:00:00:00:00", // Default
    "02:00:4C:4F:4F:50", // Npcap Loopback Adapter, Microsoft Loopback Adapter
];

/// Returns `true` when the MAC address matches a known virtual-machine prefix.
///
/// This is a heuristic and may not cover every virtualization platform.
pub fn is_virtual_mac(mac: &MacAddr) -> bool {
    let mac = mac.address();
    let prefix = mac[0..8].to_uppercase();
    KNOWN_VM_MAC_PREFIXES.contains(&prefix.as_str())
}

/// Returns `true` when the MAC address matches a known loopback adapter address.
pub fn is_known_loopback_mac(mac: &MacAddr) -> bool {
    let mac = mac.address();
    KNOWN_LOOPBACK_MAC_ADDRESSES.contains(&mac.to_uppercase().as_str())
}

#[cfg(test)]
mod tests {
    use crate::net::db::oui;
    use crate::net::mac::MacAddr;

    #[test]
    fn detects_loopback() {
        let mac: MacAddr = "00:00:00:00:00:00".parse().unwrap();
        assert!(oui::is_known_loopback_mac(&mac));
    }

    #[test]
    fn detects_virtual() {
        let mac: MacAddr = "00:50:56:AA:BB:CC".parse().unwrap();
        assert!(oui::is_virtual_mac(&mac));
    }
}
