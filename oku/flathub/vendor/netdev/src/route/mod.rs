use crate::interface::interface::Interface;
use crate::net::device::NetworkDevice;
use std::net::IpAddr;

/// Returns the default gateway associated with the active default interface.
///
/// Returns an error when the local IP address cannot be determined, when no matching interface
/// is found, or when the platform does not provide gateway information for the default route.
pub fn get_default_gateway() -> Result<NetworkDevice, String> {
    let local_ip: IpAddr = match crate::net::ip::get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    let interfaces: Vec<Interface> = crate::interface::get_interfaces();
    for iface in interfaces {
        if crate::interface::iface_has_ip(&iface, local_ip) {
            if let Some(gateway) = iface.gateway {
                return Ok(gateway);
            }
        }
    }
    Err(String::from("Default Gateway not found"))
}
