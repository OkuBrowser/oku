pub mod flags;
pub mod interface;
pub mod ipv6_addr_flags;
pub mod mtu;
pub mod state;
pub mod types;

use crate::interface::interface::Interface;

#[cfg(feature = "gateway")]
use std::net::IpAddr;

/// Returns the interface currently used for the system's default route.
///
/// This function is available when the `gateway` feature is enabled.
///
/// Returns an error when no default route can be determined or when the local IP address
/// cannot be resolved on the current platform.
#[cfg(feature = "gateway")]
pub fn get_default_interface() -> Result<Interface, String> {
    resolve_default_interface(interfaces())
}

/// Returns a list of the network interfaces.
///
/// Each `Interface` contains the data that could be collected at discovery time.
pub fn get_interfaces() -> Vec<Interface> {
    interfaces()
}

/// Pick the interface index corresponding to the system's default route.
/// Prefers exact IP match; falls back to subnet containment.
#[cfg(feature = "gateway")]
pub(crate) fn pick_default_iface_index(ifaces: &[Interface], local_ip: IpAddr) -> Option<u32> {
    let mut subnet_candidate: Option<u32> = None;

    for iface in ifaces {
        match local_ip {
            IpAddr::V4(ipv4) => {
                if iface.ipv4.iter().any(|x| x.addr() == ipv4) {
                    return Some(iface.index);
                }
                if subnet_candidate.is_none() && iface.ipv4.iter().any(|x| x.contains(&ipv4)) {
                    subnet_candidate = Some(iface.index);
                }
            }
            IpAddr::V6(ipv6) => {
                if iface.ipv6.iter().any(|x| x.addr() == ipv6) {
                    return Some(iface.index);
                }
                if subnet_candidate.is_none() && iface.ipv6.iter().any(|x| x.contains(&ipv6)) {
                    subnet_candidate = Some(iface.index);
                }
            }
        }
    }
    subnet_candidate
}

#[cfg(feature = "gateway")]
pub(crate) fn iface_has_ip(iface: &Interface, local_ip: IpAddr) -> bool {
    match local_ip {
        IpAddr::V4(ipv4) => iface.ipv4.iter().any(|x| x.addr() == ipv4),
        IpAddr::V6(ipv6) => iface.ipv6.iter().any(|x| x.addr() == ipv6),
    }
}

#[cfg(feature = "gateway")]
pub(crate) fn resolve_default_interface(mut ifaces: Vec<Interface>) -> Result<Interface, String> {
    use crate::net::ip::get_local_ipaddr;

    if let Some(pos) = ifaces.iter().position(|iface| iface.default) {
        return Ok(ifaces.swap_remove(pos));
    }

    let local_ip: IpAddr = match get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };

    let idx = pick_default_iface_index(&ifaces, local_ip)
        .ok_or_else(|| String::from("Default interface not found"))?;
    ifaces
        .into_iter()
        .find(|iface| iface.index == idx)
        .ok_or_else(|| String::from("Default interface not found"))
}

pub(crate) fn interfaces() -> Vec<Interface> {
    #[cfg(target_os = "linux")]
    {
        crate::os::linux::interface::interfaces()
    }
    #[cfg(target_os = "android")]
    {
        crate::os::android::interface::interfaces()
    }
    #[cfg(target_os = "windows")]
    {
        crate::os::windows::interface::interfaces()
    }
    #[cfg(target_os = "macos")]
    {
        crate::os::macos::interface::interfaces()
    }
    #[cfg(target_os = "ios")]
    {
        crate::os::ios::interface::interfaces()
    }
    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    {
        crate::os::bsd::interface::interfaces()
    }
}

#[cfg(test)]
mod tests {
    #![cfg(feature = "gateway")]
    use crate::interface::types::InterfaceType;
    use crate::interface::{interface::Interface, pick_default_iface_index};
    use ipnet::{Ipv4Net, Ipv6Net};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn exact_match_and_fallback_v4() {
        let mut a = Interface::dummy();
        a.index = 1;
        a.if_type = InterfaceType::Ethernet;
        a.ipv4 = vec![Ipv4Net::new(Ipv4Addr::new(192, 168, 1, 10), 24).unwrap()];

        let mut b = Interface::dummy();
        b.index = 2;
        b.if_type = InterfaceType::Ethernet;
        b.ipv4 = vec![Ipv4Net::new(Ipv4Addr::new(10, 0, 0, 2), 8).unwrap()];

        // Prefers exact match
        let local = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10));
        assert_eq!(
            pick_default_iface_index(&[a.clone(), b.clone()], local),
            Some(1)
        );

        // Fallback to subnet match
        let in_subnet = IpAddr::V4(Ipv4Addr::new(10, 0, 5, 23));
        assert_eq!(pick_default_iface_index(&[a, b], in_subnet), Some(2));
    }

    #[test]
    fn exact_match_and_fallback_v6() {
        let mut a = Interface::dummy();
        a.index = 11;
        a.if_type = InterfaceType::Ethernet;
        a.ipv6 = vec![Ipv6Net::new("2001:db8::10".parse::<Ipv6Addr>().unwrap(), 64).unwrap()];

        let mut b = Interface::dummy();
        b.index = 22;
        b.if_type = InterfaceType::Ethernet;
        b.ipv6 = vec![Ipv6Net::new("2606:4700::2".parse::<Ipv6Addr>().unwrap(), 32).unwrap()];

        // Prefers exact match
        let local = IpAddr::V6("2001:db8::10".parse().unwrap());
        assert_eq!(
            pick_default_iface_index(&[a.clone(), b.clone()], local),
            Some(11)
        );

        // Fallback to subnet match
        let in_subnet = IpAddr::V6("2606:4700::abcd".parse().unwrap());
        assert_eq!(pick_default_iface_index(&[a, b], in_subnet), Some(22));
    }

    #[test]
    fn no_exact_nor_subnet() {
        let mut a = Interface::dummy();
        a.index = 3;
        a.ipv4 = vec![Ipv4Net::new(Ipv4Addr::new(192, 168, 0, 5), 24).unwrap()];
        let local = IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1));
        assert_eq!(pick_default_iface_index(&[a], local), None);
    }
}
