use std::ffi::CString;
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::raw::c_char;

use super::sockaddr::{SockaddrRef, compute_sockaddr_len, netmask_ip_autolen, try_mac_from_raw};
use crate::interface::interface::Interface;
use crate::interface::ipv6_addr_flags::get_ipv6_addr_flags;
use crate::interface::mtu::get_mtu;
use crate::interface::state::OperState;
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::os::unix::types::{get_interface_type, interface_name_from_ptr};
use crate::stats::counters::{InterfaceStats, get_stats};

#[cfg(target_os = "android")]
pub fn unix_interfaces() -> Vec<Interface> {
    use crate::os::android;

    if let Some((getifaddrs, freeifaddrs)) = android::get_libc_ifaddrs() {
        return unix_interfaces_inner(getifaddrs, freeifaddrs);
    }
    Vec::new()
}

#[cfg(not(target_os = "android"))]
pub fn unix_interfaces() -> Vec<Interface> {
    unix_interfaces_inner(libc::getifaddrs, libc::freeifaddrs)
}

fn unix_interfaces_inner(
    getifaddrs: unsafe extern "C" fn(*mut *mut libc::ifaddrs) -> libc::c_int,
    freeifaddrs: unsafe extern "C" fn(*mut libc::ifaddrs),
) -> Vec<Interface> {
    let mut ifaces: Vec<Interface> = Vec::new();
    let mut addrs: MaybeUninit<*mut libc::ifaddrs> = MaybeUninit::uninit();
    if unsafe { getifaddrs(addrs.as_mut_ptr()) } != 0 {
        return ifaces;
    }
    let addrs = unsafe { addrs.assume_init() };
    let mut addr = addrs;
    while !addr.is_null() {
        let addr_ref: &libc::ifaddrs = unsafe { &*addr };
        let if_type = get_interface_type(addr_ref);
        let c_str = addr_ref.ifa_name as *const c_char;
        let name = interface_name_from_ptr(c_str);
        let if_index = if_nametoindex_or_zero(&name);
        let cap: libc::socklen_t = super::sockaddr::sockaddr_storage_cap();
        let addr_len_opt = unsafe { compute_sockaddr_len(addr_ref.ifa_addr, None, Some(cap)) };
        let (mac, ip, mut ipv6_scope_id) = match addr_len_opt {
            Some(addr_len) => {
                let mac = unsafe {
                    try_mac_from_raw(addr_ref.ifa_addr as *const libc::sockaddr, addr_len)
                };
                let (ip, scope) = unsafe {
                    match SockaddrRef::from_raw(
                        addr_ref.ifa_addr as *const libc::sockaddr,
                        addr_len,
                    ) {
                        Some(sa_ref) => {
                            let ip = sa_ref.to_ip();
                            let scope = sa_ref.to_ipv6_scope();
                            (Some(ip), scope)
                        }
                        None => (None, None),
                    }
                };

                (mac, ip, scope)
            }
            None => (None, None, None),
        };
        let netmask: Option<IpAddr> =
            unsafe { netmask_ip_autolen(addr_ref.ifa_netmask as *const libc::sockaddr) };
        let stats: Option<InterfaceStats> = get_stats(Some(addr_ref), &name);
        let mut ini_ipv4: Option<Ipv4Net> = None;
        let mut ini_ipv6: Option<Ipv6Net> = None;
        if let Some(ip) = ip {
            match ip {
                IpAddr::V4(ipv4) => {
                    let netmask: Ipv4Addr = match netmask {
                        Some(netmask) => match netmask {
                            IpAddr::V4(netmask) => netmask,
                            IpAddr::V6(_) => Ipv4Addr::UNSPECIFIED,
                        },
                        None => Ipv4Addr::UNSPECIFIED,
                    };
                    match Ipv4Net::with_netmask(ipv4, netmask) {
                        Ok(ipv4_net) => ini_ipv4 = Some(ipv4_net),
                        Err(_) => {}
                    }
                }
                IpAddr::V6(ipv6) => {
                    let netmask: Ipv6Addr = match netmask {
                        Some(netmask) => match netmask {
                            IpAddr::V4(_) => Ipv6Addr::UNSPECIFIED,
                            IpAddr::V6(netmask) => netmask,
                        },
                        None => Ipv6Addr::UNSPECIFIED,
                    };
                    match Ipv6Net::with_netmask(ipv6, netmask) {
                        Ok(ipv6_net) => {
                            let scope_id = resolve_ipv6_scope_id(&ipv6, ipv6_scope_id, if_index);
                            ini_ipv6 = Some(ipv6_net);
                            ipv6_scope_id = Some(scope_id);
                        }
                        Err(_) => {}
                    };
                }
            }
        }

        // Check if there is already an interface with this name (since getifaddrs returns one
        // entry per address, so if the interface has multiple addresses, it returns multiple entries).
        // If so, add the IP addresses from the current entry into the existing interface. Otherwise, add a new interface.
        if let Some(iface) = ifaces.iter_mut().find(|iface| iface.name == name) {
            if let Some(mac) = mac {
                iface.mac_addr = Some(mac);
            }
            if iface.stats.is_none() {
                iface.stats = stats;
            }
            if let Some(ipv4_addr) = ini_ipv4 {
                push_ipv4(&mut iface.ipv4, ipv4_addr);
            }
            if let (Some(ipv6_addr), Some(scope_id)) = (ini_ipv6, ipv6_scope_id) {
                let af = get_ipv6_addr_flags(&iface.name, &ipv6_addr.addr());
                push_ipv6(
                    &mut iface.ipv6,
                    &mut iface.ipv6_scope_ids,
                    &mut iface.ipv6_addr_flags,
                    ipv6_addr,
                    scope_id,
                    af,
                );
            }
        } else {
            let mtu = get_mtu(addr_ref, &name);
            let ini_ipv6_flags = match ini_ipv6.as_ref() {
                Some(ipv6_addr) => vec![get_ipv6_addr_flags(&name, &ipv6_addr.addr())],
                None => Vec::new(),
            };

            let interface: Interface = Interface {
                index: if_index,
                name,
                friendly_name: None,
                description: None,
                if_type: if_type,
                mac_addr: mac,
                ipv4: match ini_ipv4 {
                    Some(ipv4_addr) => vec![ipv4_addr],
                    None => Vec::new(),
                },
                ipv6: match ini_ipv6 {
                    Some(ipv6_addr) => vec![ipv6_addr],
                    None => Vec::new(),
                },
                ipv6_scope_ids: match ipv6_scope_id {
                    Some(scope_id) => vec![scope_id],
                    None => Vec::new(),
                },
                ipv6_addr_flags: ini_ipv6_flags,
                flags: addr_ref.ifa_flags,
                oper_state: OperState::from_if_flags(addr_ref.ifa_flags),
                transmit_speed: None,
                receive_speed: None,
                auto_negotiate: None,
                dhcp_v4_enabled: None,
                dhcp_v6_enabled: None,
                stats,
                #[cfg(feature = "gateway")]
                gateway: None,
                #[cfg(feature = "gateway")]
                dns_servers: Vec::new(),
                mtu,
                #[cfg(feature = "gateway")]
                default: false,
            };
            ifaces.push(interface);
        }
        addr = addr_ref.ifa_next;
    }
    unsafe {
        freeifaddrs(addrs);
    }
    for iface in &mut ifaces {
        if iface.index == 0 {
            iface.index = if_nametoindex_or_zero(&iface.name);
        }

        #[cfg(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        if let Some(ls) = crate::os::unix::link_speed::get_link_speed(&iface.name).ok() {
            iface.transmit_speed = ls.bps;
            iface.receive_speed = ls.bps;

            iface.auto_negotiate = Some(ls.auto_negotiate);
        }
    }
    ifaces
}

fn push_ipv4(v: &mut Vec<Ipv4Net>, net: Ipv4Net) -> bool {
    if v.iter()
        .any(|existing| existing.addr() == net.addr() && existing.prefix_len() == net.prefix_len())
    {
        return false;
    }
    v.push(net);
    true
}

fn push_ipv6(
    addrs: &mut Vec<Ipv6Net>,
    scope_ids: &mut Vec<u32>,
    addr_flags: &mut Vec<crate::interface::ipv6_addr_flags::Ipv6AddrFlags>,
    net: Ipv6Net,
    scope_id: u32,
    flags: crate::interface::ipv6_addr_flags::Ipv6AddrFlags,
) -> bool {
    if addrs
        .iter()
        .any(|existing| existing.addr() == net.addr() && existing.prefix_len() == net.prefix_len())
    {
        return false;
    }
    addrs.push(net);
    scope_ids.push(scope_id);
    addr_flags.push(flags);
    true
}

fn if_nametoindex_or_zero(name: &str) -> u32 {
    match CString::new(name.as_bytes()) {
        Ok(name) => unsafe { libc::if_nametoindex(name.as_ptr()) },
        Err(_) => 0,
    }
}

fn resolve_ipv6_scope_id(addr: &Ipv6Addr, raw_scope_id: Option<u32>, if_index: u32) -> u32 {
    match raw_scope_id {
        Some(scope_id) if scope_id != 0 => scope_id,
        _ if addr.is_unicast_link_local() => if_index,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_ipv6_scope_id;
    use std::net::Ipv6Addr;

    #[test]
    fn preserves_non_zero_ipv6_scope_id() {
        let addr = "fe80::1".parse::<Ipv6Addr>().unwrap();
        assert_eq!(resolve_ipv6_scope_id(&addr, Some(9), 3), 9);
    }

    #[test]
    fn derives_link_local_scope_from_interface_index() {
        let addr = "fe80::1".parse::<Ipv6Addr>().unwrap();
        assert_eq!(resolve_ipv6_scope_id(&addr, None, 7), 7);
        assert_eq!(resolve_ipv6_scope_id(&addr, Some(0), 7), 7);
    }

    #[test]
    fn keeps_global_ipv6_scope_id_zero() {
        let addr = "2001:db8::1".parse::<Ipv6Addr>().unwrap();
        assert_eq!(resolve_ipv6_scope_id(&addr, None, 7), 0);
        assert_eq!(resolve_ipv6_scope_id(&addr, Some(0), 7), 0);
    }

    #[test]
    fn ipv6_addr_flags_aligned_with_addrs() {
        let ifaces = super::unix_interfaces();
        for iface in &ifaces {
            assert_eq!(
                iface.ipv6.len(),
                iface.ipv6_addr_flags.len(),
                "ipv6_addr_flags length mismatch for {}",
                iface.name
            );
            assert_eq!(
                iface.ipv6.len(),
                iface.ipv6_scope_ids.len(),
                "ipv6_scope_ids length mismatch for {}",
                iface.name
            );
        }
    }
}
