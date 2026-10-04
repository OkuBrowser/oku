use std::time::SystemTime;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::interface::interface::Interface;

/// Interface traffic statistics at a given point in time.
///
/// The counters are cumulative values reported by the operating system, typically since boot.
/// Availability depends on the target platform and the specific interface driver.
///
/// On macOS and iOS, these are 32-bit `if_data` counters from `getifaddrs`.
/// Byte counters wrap at 4 GiB, so they are not guaranteed to be full totals since boot.
/// Casting them to `u64` does not recover wrapped traffic. No wrap detection or
/// synthesized 64-bit totals are provided.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InterfaceStats {
    /// Received byte counter reported by the OS; subject to the platform limits above.
    pub rx_bytes: u64,
    /// Transmitted byte counter reported by the OS; subject to the platform limits above.
    pub tx_bytes: u64,
    /// Time when this snapshot was collected.
    ///
    /// This is `None` only when the platform-specific collector cannot provide a timestamp.
    pub timestamp: Option<SystemTime>,
}

#[cfg(target_vendor = "apple")]
pub(crate) fn get_stats(ifa: Option<&libc::ifaddrs>, _name: &str) -> Option<InterfaceStats> {
    let ifa = ifa?;
    if ifa.ifa_addr.is_null() || ifa.ifa_data.is_null() {
        return None;
    }
    // getifaddrs supplies if_data only for link-layer entries on Apple platforms.
    if unsafe { (*ifa.ifa_addr).sa_family as libc::c_int } != libc::AF_LINK {
        return None;
    }
    let data = unsafe { &*ifa.ifa_data.cast::<libc::if_data>() };
    Some(InterfaceStats {
        rx_bytes: u64::from(data.ifi_ibytes),
        tx_bytes: u64::from(data.ifi_obytes),
        timestamp: Some(SystemTime::now()),
    })
}

#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
pub(crate) fn get_stats(ifa: Option<&libc::ifaddrs>, _name: &str) -> Option<InterfaceStats> {
    if let Some(ifa) = ifa {
        if !ifa.ifa_data.is_null() {
            let data = unsafe { &*(ifa.ifa_data as *const libc::if_data) };
            Some(InterfaceStats {
                rx_bytes: data.ifi_ibytes as u64,
                tx_bytes: data.ifi_obytes as u64,
                timestamp: Some(SystemTime::now()),
            })
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn get_stats(_ifa: Option<&libc::ifaddrs>, name: &str) -> Option<InterfaceStats> {
    get_stats_from_name(name)
}

#[cfg(target_os = "linux")]
pub(crate) fn get_stats_from_name(name: &str) -> Option<InterfaceStats> {
    use std::fs::read_to_string;
    let rx_path = format!("/sys/class/net/{}/statistics/rx_bytes", name);
    let tx_path = format!("/sys/class/net/{}/statistics/tx_bytes", name);

    let rx_bytes = read_to_string(rx_path).ok()?.trim().parse::<u64>().ok()?;
    let tx_bytes = read_to_string(tx_path).ok()?.trim().parse::<u64>().ok()?;

    Some(InterfaceStats {
        rx_bytes,
        tx_bytes,
        timestamp: Some(SystemTime::now()),
    })
}

#[cfg(target_os = "android")]
pub(crate) fn get_stats_from_name(name: &str) -> Option<InterfaceStats> {
    #[cfg(feature = "android-extra")]
    if let Some(stats) = crate::os::android::api::get_interface_stats(name) {
        return Some(stats);
    }

    use std::fs::read_to_string;
    let rx_path = format!("/sys/class/net/{}/statistics/rx_bytes", name);
    let tx_path = format!("/sys/class/net/{}/statistics/tx_bytes", name);

    let rx_bytes = read_to_string(rx_path).ok()?.trim().parse::<u64>().ok()?;
    let tx_bytes = read_to_string(tx_path).ok()?.trim().parse::<u64>().ok()?;

    Some(InterfaceStats {
        rx_bytes,
        tx_bytes,
        timestamp: Some(SystemTime::now()),
    })
}

#[cfg(target_vendor = "apple")]
fn get_stats_from_name(name: &str) -> Option<InterfaceStats> {
    let mut addrs = std::ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut addrs) } != 0 {
        return None;
    }
    // The list and its associated names, addresses, and data remain valid until freed.
    let result = unsafe { get_stats_from_ifaddrs(addrs, name) };
    if !addrs.is_null() {
        unsafe { libc::freeifaddrs(addrs) };
    }
    result
}

#[cfg(target_vendor = "apple")]
// Safety: addrs must be null or a valid getifaddrs-style list whose storage remains
// alive for this call, including the NUL-terminated names, addresses, and if_data.
unsafe fn get_stats_from_ifaddrs(
    mut addrs: *const libc::ifaddrs,
    name: &str,
) -> Option<InterfaceStats> {
    while !addrs.is_null() {
        let ifa = unsafe { &*addrs };
        addrs = ifa.ifa_next;
        if ifa.ifa_name.is_null()
            || unsafe { std::ffi::CStr::from_ptr(ifa.ifa_name) }.to_bytes() != name.as_bytes()
        {
            continue;
        }
        if let Some(stats) = get_stats(Some(ifa), name) {
            return Some(stats);
        }
    }
    None
}

#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
fn get_stats_from_name(name: &str) -> Option<InterfaceStats> {
    use std::ffi::CStr;
    let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();

    // 1. getifaddrs()
    if unsafe { libc::getifaddrs(&mut ifap) } != 0 {
        return None;
    }

    let mut current = ifap;
    let mut result = None;

    // 2. Iterate through the list of ifaddrs
    while !current.is_null() {
        unsafe {
            let ifa = &*current;

            if ifa.ifa_name.is_null() {
                current = ifa.ifa_next;
                continue;
            }

            let ifa_name = CStr::from_ptr(ifa.ifa_name).to_string_lossy();

            if ifa_name == name {
                if !ifa.ifa_data.is_null() {
                    let data = &*(ifa.ifa_data as *const libc::if_data);
                    result = Some(InterfaceStats {
                        rx_bytes: data.ifi_ibytes as u64,
                        tx_bytes: data.ifi_obytes as u64,
                        timestamp: Some(SystemTime::now()),
                    });
                    break;
                }
            }

            current = ifa.ifa_next;
        }
    }

    // 3. freeifaddrs()
    unsafe {
        libc::freeifaddrs(ifap);
    }

    result
}

#[cfg(target_os = "windows")]
pub(crate) fn get_stats_from_index(index: u32) -> Option<InterfaceStats> {
    use std::mem::zeroed;
    use std::time::SystemTime;
    use windows_sys::Win32::NetworkManagement::IpHelper::{GetIfEntry2, MIB_IF_ROW2};

    let mut row: MIB_IF_ROW2 = unsafe { zeroed() };
    row.InterfaceIndex = index;

    unsafe {
        if GetIfEntry2(&mut row) == 0 {
            Some(InterfaceStats {
                rx_bytes: row.InOctets as u64,
                tx_bytes: row.OutOctets as u64,
                timestamp: Some(SystemTime::now()),
            })
        } else {
            None
        }
    }
}

pub(crate) fn update_interface_stats(iface: &mut Interface) -> std::io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        iface.stats = get_stats_from_name(iface.name.as_str());
    }
    #[cfg(any(
        target_vendor = "apple",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    {
        iface.stats = get_stats_from_name(iface.name.as_str());
    }
    #[cfg(target_os = "windows")]
    {
        iface.stats = get_stats_from_index(iface.index);
    }
    Ok(())
}

#[cfg(all(test, target_vendor = "apple"))]
mod apple_tests {
    use super::*;
    use std::{mem::zeroed, ptr};

    #[test]
    fn selects_only_link_layer_data() {
        let mut data: libc::if_data = unsafe { zeroed() };
        data.ifi_ibytes = 123;
        data.ifi_obytes = u32::MAX;
        let mut addr: libc::sockaddr = unsafe { zeroed() };
        let mut ifa: libc::ifaddrs = unsafe { zeroed() };
        ifa.ifa_data = ptr::from_mut(&mut data).cast();
        for family in [libc::AF_INET, libc::AF_INET6, libc::AF_UNSPEC] {
            addr.sa_family = family as _;
            ifa.ifa_addr = &mut addr;
            assert_eq!(get_stats(Some(&ifa), "en0"), None);
        }
        addr.sa_family = libc::AF_LINK as _;
        ifa.ifa_addr = &mut addr;
        let stats = get_stats(Some(&ifa), "en0").unwrap();
        assert_eq!(stats.rx_bytes, 123);
        assert_eq!(stats.tx_bytes, u64::from(u32::MAX));
        assert!(stats.timestamp.is_some());
    }

    #[test]
    fn preserves_zero_and_missing_data() {
        let mut data: libc::if_data = unsafe { zeroed() };
        let mut addr: libc::sockaddr = unsafe { zeroed() };
        addr.sa_family = libc::AF_LINK as _;
        let mut ifa: libc::ifaddrs = unsafe { zeroed() };
        assert_eq!(get_stats(None, "en0"), None);
        assert_eq!(get_stats(Some(&ifa), "en0"), None);
        ifa.ifa_data = ptr::from_mut(&mut data).cast();
        assert_eq!(get_stats(Some(&ifa), "en0"), None);
        ifa.ifa_addr = &mut addr;
        let stats = get_stats(Some(&ifa), "en0").unwrap();
        assert_eq!((stats.rx_bytes, stats.tx_bytes), (0, 0));
        assert!(stats.timestamp.is_some());
        ifa.ifa_data = ptr::null_mut();
        assert_eq!(get_stats(Some(&ifa), "en0"), None);
    }

    #[test]
    fn name_lookup_skips_missing_and_non_link_entries() {
        let mut data: libc::if_data = unsafe { zeroed() };
        data.ifi_ibytes = 42;
        let mut ip: libc::sockaddr = unsafe { zeroed() };
        ip.sa_family = libc::AF_INET as _;
        let mut link: libc::sockaddr = unsafe { zeroed() };
        link.sa_family = libc::AF_LINK as _;
        let mut entries: [libc::ifaddrs; 6] = unsafe { zeroed() };
        for i in 0..entries.len() {
            entries[i].ifa_name = c"en0".as_ptr().cast_mut();
            entries[i].ifa_addr = &mut link;
            entries[i].ifa_data = ptr::from_mut(&mut data).cast();
            if i + 1 < entries.len() {
                entries[i].ifa_next = &mut entries[i + 1];
            }
        }
        entries[0].ifa_name = ptr::null_mut();
        entries[1].ifa_name = c"en1".as_ptr().cast_mut();
        entries[2].ifa_addr = &mut ip;
        entries[3].ifa_data = ptr::null_mut();
        entries[4].ifa_addr = ptr::null_mut();
        let stats = unsafe { get_stats_from_ifaddrs(entries.as_ptr(), "en0") }.unwrap();
        assert_eq!((stats.rx_bytes, stats.tx_bytes), (42, 0));
        assert!(unsafe { get_stats_from_ifaddrs(entries.as_ptr(), "missing") }.is_none());
        entries[5].ifa_data = ptr::null_mut();
        assert!(unsafe { get_stats_from_ifaddrs(entries.as_ptr(), "en0") }.is_none());
        assert!(unsafe { get_stats_from_ifaddrs(ptr::null(), "en0") }.is_none());
    }

    #[test]
    fn update_clears_unavailable_statistics() {
        let mut iface = Interface::dummy();
        iface.name = "netdev-nonexistent-interface".into();
        iface.stats = Some(InterfaceStats {
            rx_bytes: 1,
            tx_bytes: 2,
            timestamp: Some(SystemTime::now()),
        });
        iface.update_stats().unwrap();
        assert_eq!(iface.stats, None);
    }
}
