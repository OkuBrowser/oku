use crate::interface::interface::Interface;
use windows_sys::Win32::NetworkManagement::IpHelper::{GetIfEntry2, MIB_IF_ROW2, MIB_IF_ROW2_0};
use windows_sys::Win32::Networking::WinSock as ws;

pub const IFF_UP: u32 = ws::IFF_UP;
pub const IFF_BROADCAST: u32 = ws::IFF_BROADCAST;
pub const IFF_LOOPBACK: u32 = ws::IFF_LOOPBACK;
pub const IFF_POINTOPOINT: u32 = ws::IFF_POINTTOPOINT;
pub const IFF_MULTICAST: u32 = ws::IFF_MULTICAST;

//const IFF_HARDWARE_INTERFACE: u8 = 0b0000_0001;
//const IFF_FILTER_INTERFACE: u8 = 0b0000_0010;
const IFF_CONNECTOR_PRESENT: u8 = 0b0000_0100;
//const IFF_NOT_AUTHENTICATED: u8 = 0b0000_1000;
//const IFF_NOT_MEDIA_CONNECTED: u8 = 0b0001_0000;
//const IFF_PAUSED: u8 = 0b0010_0000;
//const IFF_LOW_POWER: u8 = 0b0100_0000;
//const IFF_END_POINT_INTERFACE: u8 = 0b1000_0000;

pub fn is_physical_interface(interface: &Interface) -> bool {
    is_connector_present(interface.index)
        || (interface.is_up()
            && interface.is_running()
            && !interface.is_tun()
            && !interface.is_loopback())
}

pub fn is_running(interface: &Interface) -> bool {
    interface.is_up()
}

/// Check if a network interface has a connector present, indicating it is a physical interface.
pub fn is_connector_present(if_index: u32) -> bool {
    // Initialize MIB_IF_ROW2
    let mut row: MIB_IF_ROW2 = unsafe { std::mem::zeroed() };
    row.InterfaceIndex = if_index;
    // Retrieve interface information using GetIfEntry2
    unsafe {
        if GetIfEntry2(&mut row) != 0 {
            eprintln!("Failed to get interface entry for index: {}", if_index);
            return false;
        }
    }
    // Check if the connector is present
    let oper_status_flags: MIB_IF_ROW2_0 = row.InterfaceAndOperStatusFlags;
    oper_status_flags._bitfield & IFF_CONNECTOR_PRESENT != 0
}
