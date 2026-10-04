use crate::interface::state::OperState;
use crate::interface::types::InterfaceType;
use std::convert::TryFrom;
use std::fs;
use std::path::{Path, PathBuf};

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

fn exists(path: impl AsRef<Path>) -> bool {
    Path::new(path.as_ref()).exists()
}

/// Check if the interface is a Wi-Fi (802.11) interface.
fn is_wifi_interface(ifname: &str) -> bool {
    let base = PathBuf::from("/sys/class/net").join(ifname);

    // 1) Check uevent file for DEVTYPE=wlan
    if let Some(ue) = read_trimmed(base.join("uevent"))
        && ue.lines().any(|l| l.trim() == "DEVTYPE=wlan")
    {
        return true;
    }

    // 2) Check for wireless or phy80211 directories
    exists(base.join("wireless")) || exists(base.join("phy80211"))
}

/// Check if the interface is a WWAN/mobile broadband interface.
fn is_wwan_interface(ifname: &str) -> bool {
    let base = PathBuf::from("/sys/class/net").join(ifname);

    if let Some(uevent) = read_trimmed(base.join("uevent"))
        && uevent.lines().any(|line| line.trim() == "DEVTYPE=wwan")
    {
        return true;
    }

    is_wwan_name(ifname)
}

fn is_wwan_name(ifname: &str) -> bool {
    ifname == "wwan"
        || ifname.starts_with("wwan")
        || ifname.starts_with("wwp")
        || ifname.starts_with("rmnet")
        || ifname.starts_with("ccmni")
}

/// Check if the interface is a virtual interface.
pub fn is_virtual_interface(ifname: &str) -> bool {
    let dev_link = PathBuf::from("/sys/class/net").join(ifname).join("device");

    // If device symlink is missing, it's likely virtual
    if !dev_link.exists() {
        return true;
    }

    // Follow the symlink to see if it points to /sys/devices/virtual/
    // If we can't resolve the path, be conservative and say it's not virtual
    match dev_link.canonicalize() {
        Ok(real) => real.starts_with("/sys/devices/virtual"),
        Err(_) => false,
    }
}

/// Get the interface type.
pub fn get_interface_type(ifname: &str) -> InterfaceType {
    // Check for Wi-Fi first
    // Since some Wi-Fi interfaces may also be reported as Ethernet,
    // we prioritize Wi-Fi detection.
    if is_wifi_interface(ifname) {
        return InterfaceType::Wireless80211;
    }

    if is_wwan_interface(ifname) {
        return InterfaceType::Wwan;
    }

    // Read the type from sysfs
    let p = PathBuf::from("/sys/class/net").join(ifname).join("type");
    let ty = match read_trimmed(&p).and_then(|s| s.parse::<u32>().ok()) {
        Some(v) => v,
        None => return InterfaceType::Unknown,
    };

    if ty == super::arp::ARPHRD_ETHER {
        InterfaceType::Ethernet
    } else {
        InterfaceType::try_from(ty).unwrap_or(InterfaceType::Unknown)
    }
}

/// Get the interface speed in bps (bits per second).
pub fn get_interface_speed(ifname: &str) -> Option<u64> {
    let p = PathBuf::from("/sys/class/net").join(ifname).join("speed");
    let s = read_trimmed(p)?;
    let mbps: i64 = s.parse().ok()?;
    if mbps <= 0 {
        return None;
    }
    // Convert Mbps to bps
    Some((mbps as u64) * 1_000_000)
}

/// Get the operational state of the interface.
pub fn operstate(ifname: &str) -> OperState {
    let p = PathBuf::from("/sys/class/net")
        .join(ifname)
        .join("operstate");
    read_trimmed(p)
        .and_then(|s| s.parse::<OperState>().ok())
        .unwrap_or(OperState::Unknown)
}
