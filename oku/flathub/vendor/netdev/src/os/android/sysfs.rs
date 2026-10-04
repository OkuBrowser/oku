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

fn is_wifi_interface(ifname: &str) -> bool {
    let base = PathBuf::from("/sys/class/net").join(ifname);

    if let Some(uevent) = read_trimmed(base.join("uevent")) {
        if uevent.lines().any(|line| line.trim() == "DEVTYPE=wlan") {
            return true;
        }
    }

    exists(base.join("wireless")) || exists(base.join("phy80211"))
}

pub fn get_interface_type(ifname: &str) -> Option<InterfaceType> {
    if is_wifi_interface(ifname) {
        return Some(InterfaceType::Wireless80211);
    }

    let path = PathBuf::from("/sys/class/net").join(ifname).join("type");
    let value = read_trimmed(path)?.parse::<u32>().ok()?;

    if value == crate::os::linux::arp::ARPHRD_ETHER {
        Some(InterfaceType::Ethernet)
    } else {
        InterfaceType::try_from(value).ok()
    }
}

pub fn get_interface_speed(ifname: &str) -> Option<u64> {
    let path = PathBuf::from("/sys/class/net").join(ifname).join("speed");
    let speed_mbps = read_trimmed(path)?.parse::<i64>().ok()?;
    if speed_mbps <= 0 {
        return None;
    }

    Some((speed_mbps as u64) * 1_000_000)
}
