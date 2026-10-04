use crate::interface::types::InterfaceType;

pub fn get_interface_type(addr_ref: &libc::ifaddrs) -> InterfaceType {
    if !addr_ref.ifa_data.is_null() {
        let if_data = unsafe { &*(addr_ref.ifa_data as *const libc::if_data) };
        InterfaceType::try_from(if_data.ifi_type as u32).unwrap_or(InterfaceType::Unknown)
    } else {
        InterfaceType::Unknown
    }
}
