use crate::interface::types::InterfaceType;
use std::{ffi::CString, mem};

/// Get the interface type from ifaddrs if_data.ifi_type field.
pub fn get_interface_type(addr_ref: &libc::ifaddrs) -> InterfaceType {
    if !addr_ref.ifa_data.is_null() {
        let if_data = unsafe { &*(addr_ref.ifa_data as *const libc::if_data) };
        InterfaceType::try_from(if_data.ifi_type as u32).unwrap_or(InterfaceType::Unknown)
    } else {
        InterfaceType::Unknown
    }
}

// BSD ioctl encoding (ioccom.h)
const IOC_INOUT: u32 = 0xC000_0000;
const IOCPARM_MASK: u32 = 0x1fff;

const fn ioc(inout: u32, group: u8, num: u8, len: u32) -> u32 {
    inout | ((len & IOCPARM_MASK) << 16) | ((group as u32) << 8) | (num as u32)
}

// #define SIOCGIFFUNCTIONALTYPE _IOWR('i', 173, struct ifreq)
fn siocgiffunctionaltype() -> u64 {
    let len = mem::size_of::<libc::ifreq>() as u32;
    ioc(IOC_INOUT, b'i', 173, len) as u64
}

// Values from <net/if.h> (IFRTYPE_FUNCTIONAL_*)
const IFRTYPE_FUNCTIONAL_UNKNOWN: u32 = 0;
const IFRTYPE_FUNCTIONAL_LOOPBACK: u32 = 1;
const IFRTYPE_FUNCTIONAL_WIRED: u32 = 2;
const IFRTYPE_FUNCTIONAL_WIFI_INFRA: u32 = 3;
const IFRTYPE_FUNCTIONAL_WIFI_AWDL: u32 = 4;
const IFRTYPE_FUNCTIONAL_CELLULAR: u32 = 5;
const IFRTYPE_FUNCTIONAL_INTCOPROC: u32 = 6;
const IFRTYPE_FUNCTIONAL_COMPANIONLINK: u32 = 7;

fn map_to_interface_type(ft: u32) -> InterfaceType {
    match ft {
        IFRTYPE_FUNCTIONAL_UNKNOWN => InterfaceType::Unknown,
        IFRTYPE_FUNCTIONAL_LOOPBACK => InterfaceType::Loopback,
        IFRTYPE_FUNCTIONAL_WIRED => InterfaceType::Ethernet,
        IFRTYPE_FUNCTIONAL_WIFI_INFRA => InterfaceType::Wireless80211,
        IFRTYPE_FUNCTIONAL_WIFI_AWDL => InterfaceType::PeerToPeerWireless,
        IFRTYPE_FUNCTIONAL_CELLULAR => InterfaceType::Wwanpp,
        IFRTYPE_FUNCTIONAL_INTCOPROC => InterfaceType::Unknown,
        IFRTYPE_FUNCTIONAL_COMPANIONLINK => InterfaceType::Unknown,
        _ => InterfaceType::Unknown,
    }
}

/// Get the functional interface type using SIOCGIFFUNCTIONALTYPE ioctl.
pub(crate) fn get_functional_type(name: &str) -> InterfaceType {
    // socket(AF_INET, SOCK_DGRAM, 0) + ioctl(SIOCGIFFUNCTIONALTYPE)
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return InterfaceType::Unknown;
    }

    let c_name = match CString::new(name) {
        Ok(v) => v,
        Err(_) => {
            unsafe { libc::close(fd) };
            return InterfaceType::Unknown;
        }
    };

    // ifreq layout is platform-specific; libc::ifreq exists on macOS/iOS targets.
    let mut ifr: libc::ifreq = unsafe { mem::zeroed() };

    // ifr_name is a fixed-size [c_char; IFNAMSIZ]
    // Copy with truncation; ensure NUL terminated
    unsafe {
        let dst = ifr.ifr_name.as_mut_ptr() as *mut u8;
        let src = c_name.as_bytes_with_nul();
        let copy_len = std::cmp::min(ifr.ifr_name.len() - 1, src.len() - 1);

        std::ptr::copy_nonoverlapping(src.as_ptr(), dst, copy_len);
        *dst.add(copy_len) = 0;
    }

    let req = siocgiffunctionaltype();
    let ok = unsafe { libc::ioctl(fd, req, &mut ifr) } >= 0;
    unsafe { libc::close(fd) };

    if !ok {
        return InterfaceType::Unknown;
    }

    #[allow(clippy::unnecessary_cast)]
    let type_id = unsafe { ifr.ifr_ifru.ifru_functional_type as u32 };

    map_to_interface_type(type_id)
}

pub(crate) fn interface_type_by_name(name: &str) -> Option<InterfaceType> {
    let n = name.as_bytes();

    if n.starts_with(b"awdl") {
        return Some(InterfaceType::PeerToPeerWireless);
    }
    if n.starts_with(b"utun") || n.starts_with(b"gif") || n.starts_with(b"stf") {
        return Some(InterfaceType::Tunnel);
    }
    if n.starts_with(b"bridge") {
        return Some(InterfaceType::Bridge);
    }

    None
}
