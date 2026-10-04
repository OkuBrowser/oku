pub(crate) fn get_mtu(ifa: &libc::ifaddrs, _name: &str) -> Option<u32> {
    if !ifa.ifa_data.is_null() {
        let data = unsafe { &*(ifa.ifa_data as *mut libc::if_data) };
        Some(data.ifi_mtu as u32)
    } else {
        None
    }
}
