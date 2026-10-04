use super::netlink;
use crate::interface::interface::Interface;
use crate::interface::state::OperState;
use crate::interface::types::InterfaceType;
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::net::mac::MacAddr;
use std::net::{Ipv4Addr, Ipv6Addr};

#[cfg(feature = "gateway")]
use crate::net::device::NetworkDevice;
#[cfg(feature = "gateway")]
use crate::os::unix::dns::get_system_dns_conf;

use crate::os::unix::interface::unix_interfaces;

fn push_ipv4(v: &mut Vec<Ipv4Net>, add: (Ipv4Addr, u8)) {
    if v.iter()
        .any(|n| n.addr() == add.0 && n.prefix_len() == add.1)
    {
        return;
    }
    if let Ok(net) = Ipv4Net::new(add.0, add.1) {
        v.push(net);
    }
}

fn push_ipv6(
    addrs: &mut Vec<Ipv6Net>,
    scope_ids: &mut Vec<u32>,
    addr_flags: &mut Vec<crate::interface::ipv6_addr_flags::Ipv6AddrFlags>,
    add: (Ipv6Addr, u8),
    scope_id: u32,
    flags: crate::interface::ipv6_addr_flags::Ipv6AddrFlags,
) -> bool {
    if addrs
        .iter()
        .any(|n| n.addr() == add.0 && n.prefix_len() == add.1)
    {
        return false;
    }
    if let Ok(net) = Ipv6Net::new(add.0, add.1) {
        addrs.push(net);
        scope_ids.push(scope_id);
        addr_flags.push(flags);
        return true;
    }
    false
}

#[inline]
fn calc_v6_scope_id(addr: &Ipv6Addr, ifindex: u32) -> u32 {
    if addr.is_unicast_link_local() {
        ifindex
    } else {
        0
    }
}

fn type_is_ambiguous(if_type: InterfaceType) -> bool {
    matches!(
        if_type,
        InterfaceType::Unknown | InterfaceType::UnknownWithValue(_) | InterfaceType::Ethernet
    )
}

fn type_is_more_specific(current: InterfaceType, candidate: InterfaceType) -> bool {
    if candidate == current {
        return false;
    }

    match (current, candidate) {
        (InterfaceType::Unknown, _) | (InterfaceType::UnknownWithValue(_), _) => true,
        (InterfaceType::Ethernet, candidate) => {
            matches!(
                candidate,
                InterfaceType::Loopback
                    | InterfaceType::Wireless80211
                    | InterfaceType::Tunnel
                    | InterfaceType::Wwan
                    | InterfaceType::Wwanpp
                    | InterfaceType::Wwanpp2
                    | InterfaceType::Bridge
                    | InterfaceType::PeerToPeerWireless
                    | InterfaceType::ProprietaryVirtual
            )
        }
        (InterfaceType::Wwan, candidate) => {
            matches!(candidate, InterfaceType::Wwanpp | InterfaceType::Wwanpp2)
        }
        _ => false,
    }
}

#[cfg(feature = "android-extra")]
fn finalize_interface(iface: &mut Interface, extras: Option<&super::api::InterfaceExtras>) {
    if let Some(sysfs_type) = super::sysfs::get_interface_type(&iface.name) {
        if type_is_more_specific(iface.if_type, sysfs_type) {
            iface.if_type = sysfs_type;
        }
    }

    if type_is_ambiguous(iface.if_type)
        && let Some(guessed_type) = super::types::guess_type_by_name(&iface.name)
    {
        iface.if_type = guessed_type;
    }

    if let Some(extra) = extras {
        if iface.transmit_speed.is_none() {
            iface.transmit_speed = extra.transmit_speed;
        }
        if iface.receive_speed.is_none() {
            iface.receive_speed = extra.receive_speed;
        }
        if iface.auto_negotiate.is_none() {
            iface.auto_negotiate = extra.auto_negotiate;
        }
        if iface.stats.is_none() {
            iface.stats = extra.stats.clone();
        }
        if iface.dhcp_v4_enabled.is_none() {
            iface.dhcp_v4_enabled = extra.dhcp_v4_enabled;
        }
        if iface.dhcp_v6_enabled.is_none() {
            iface.dhcp_v6_enabled = extra.dhcp_v6_enabled;
        }
        #[cfg(feature = "gateway")]
        if iface.dns_servers.is_empty() {
            iface.dns_servers = extra.dns_servers.clone();
        }
    }

    if iface.transmit_speed.is_none() || iface.receive_speed.is_none() {
        let speed = super::sysfs::get_interface_speed(&iface.name);
        if iface.transmit_speed.is_none() {
            iface.transmit_speed = speed;
        }
        if iface.receive_speed.is_none() {
            iface.receive_speed = speed;
        }
    }

    if iface.stats.is_none() {
        iface.stats = crate::stats::counters::get_stats_from_name(&iface.name);
    }

    if iface.mtu.is_none() {
        iface.mtu = crate::os::linux::mtu::get_mtu(&iface.name);
    }
}

#[cfg(not(feature = "android-extra"))]
fn finalize_interface(iface: &mut Interface) {
    if let Some(sysfs_type) = super::sysfs::get_interface_type(&iface.name) {
        if type_is_more_specific(iface.if_type, sysfs_type) {
            iface.if_type = sysfs_type;
        }
    }

    if type_is_ambiguous(iface.if_type)
        && let Some(guessed_type) = super::types::guess_type_by_name(&iface.name)
    {
        iface.if_type = guessed_type;
    }

    if iface.transmit_speed.is_none() || iface.receive_speed.is_none() {
        let speed = super::sysfs::get_interface_speed(&iface.name);
        if iface.transmit_speed.is_none() {
            iface.transmit_speed = speed;
        }
        if iface.receive_speed.is_none() {
            iface.receive_speed = speed;
        }
    }

    if iface.stats.is_none() {
        iface.stats = crate::stats::counters::get_stats_from_name(&iface.name);
    }

    if iface.mtu.is_none() {
        iface.mtu = crate::os::linux::mtu::get_mtu(&iface.name);
    }
}

pub fn interfaces() -> Vec<Interface> {
    let mut ifaces: Vec<Interface> = Vec::new();

    match netlink::collect_interfaces() {
        Ok(rows) => {
            for r in rows {
                let name = r.name.clone();
                let mut iface = Interface {
                    index: r.index,
                    name: name.clone(),
                    friendly_name: None,
                    description: None,
                    if_type: r.if_type,
                    mac_addr: r.mac.map(MacAddr::from_octets),
                    ipv4: Vec::new(),
                    ipv6: Vec::new(),
                    ipv6_scope_ids: Vec::new(),
                    ipv6_addr_flags: Vec::new(),
                    flags: r.flags,
                    oper_state: OperState::from_if_flags(r.flags),
                    transmit_speed: None,
                    receive_speed: None,
                    auto_negotiate: None,
                    dhcp_v4_enabled: None,
                    dhcp_v6_enabled: None,
                    stats: r.stats.clone(),
                    #[cfg(feature = "gateway")]
                    gateway: None,
                    #[cfg(feature = "gateway")]
                    dns_servers: Vec::new(),
                    mtu: r.mtu,
                    #[cfg(feature = "gateway")]
                    default: false,
                };

                for (a, p) in r.ipv4 {
                    push_ipv4(&mut iface.ipv4, (a, p));
                }
                for (i, (a, p)) in r.ipv6.into_iter().enumerate() {
                    let raw = r.ipv6_addr_flags.get(i).copied().unwrap_or(0);
                    let flags = crate::os::linux::ipv6_addr_flags::from_netlink_flags(raw);
                    let scope_id = calc_v6_scope_id(&a, iface.index);
                    push_ipv6(
                        &mut iface.ipv6,
                        &mut iface.ipv6_scope_ids,
                        &mut iface.ipv6_addr_flags,
                        (a, p),
                        scope_id,
                        flags,
                    );
                }

                ifaces.push(iface);
            }
        }
        Err(_) => {
            // fallback: unix ifaddrs
            ifaces = unix_interfaces();
        }
    }

    #[cfg(feature = "android-extra")]
    {
        let interface_names: Vec<String> = ifaces.iter().map(|iface| iface.name.clone()).collect();
        let extras = super::api::collect_interface_extras(&interface_names);

        for iface in &mut ifaces {
            finalize_interface(iface, extras.get(&iface.name));
        }
    }

    #[cfg(not(feature = "android-extra"))]
    {
        for iface in &mut ifaces {
            finalize_interface(iface);
        }
    }

    // Fill gateway info
    #[cfg(feature = "gateway")]
    {
        if let Ok(mut gmap) = netlink::collect_routes() {
            for iface in &mut ifaces {
                if iface.index == 0 {
                    continue;
                }
                if let Some(row) = gmap.remove(&iface.index) {
                    iface.gateway = Some(NetworkDevice {
                        mac_addr: row.mac.map(MacAddr::from_octets).unwrap_or(MacAddr::zero()),
                        ipv4: row.gw_v4,
                        ipv6: row.gw_v6,
                    });
                }
            }
        }

        if let Some(local_ip) = crate::net::ip::get_local_ipaddr() {
            if let Some(idx) = crate::interface::pick_default_iface_index(&ifaces, local_ip) {
                if let Some(iface) = ifaces.iter_mut().find(|it| it.index == idx) {
                    iface.default = true;
                    if iface.dns_servers.is_empty() {
                        iface.dns_servers = get_system_dns_conf();
                    }
                }
            }
        }
    }

    ifaces
}

#[cfg(test)]
mod tests {
    use super::{calc_v6_scope_id, push_ipv4, push_ipv6, type_is_ambiguous, type_is_more_specific};
    use crate::interface::ipv6_addr_flags::Ipv6AddrFlags;
    use crate::interface::types::InterfaceType;
    use crate::ipnet::{Ipv4Net, Ipv6Net};
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn calculates_link_local_scope_id() {
        let addr = "fe80::1".parse::<Ipv6Addr>().unwrap();
        assert_eq!(calc_v6_scope_id(&addr, 42), 42);
    }

    #[test]
    fn keeps_global_ipv6_scope_id_zero() {
        let addr = "2001:db8::1".parse::<Ipv6Addr>().unwrap();
        assert_eq!(calc_v6_scope_id(&addr, 42), 0);
    }

    #[test]
    fn deduplicates_ipv4_addresses() {
        let mut addrs = Vec::<Ipv4Net>::new();
        push_ipv4(&mut addrs, (Ipv4Addr::new(192, 0, 2, 10), 24));
        push_ipv4(&mut addrs, (Ipv4Addr::new(192, 0, 2, 10), 24));

        assert_eq!(addrs.len(), 1);
    }

    #[test]
    fn deduplicates_ipv6_addresses() {
        let mut addrs = Vec::<Ipv6Net>::new();
        let mut scope_ids = Vec::new();
        let mut addr_flags = Vec::new();
        let first = "fe80::1".parse::<Ipv6Addr>().unwrap();
        let second = "2001:db8::1".parse::<Ipv6Addr>().unwrap();

        assert!(push_ipv6(
            &mut addrs,
            &mut scope_ids,
            &mut addr_flags,
            (first, 64),
            7,
            Ipv6AddrFlags {
                temporary: true,
                ..Ipv6AddrFlags::default()
            },
        ));
        assert!(!push_ipv6(
            &mut addrs,
            &mut scope_ids,
            &mut addr_flags,
            (first, 64),
            99,
            Ipv6AddrFlags {
                deprecated: true,
                ..Ipv6AddrFlags::default()
            },
        ));
        assert!(push_ipv6(
            &mut addrs,
            &mut scope_ids,
            &mut addr_flags,
            (second, 64),
            0,
            Ipv6AddrFlags {
                permanent: true,
                ..Ipv6AddrFlags::default()
            },
        ));

        assert_eq!(addrs.len(), 2);
        assert_eq!(scope_ids, vec![7, 0]);
        assert_eq!(addr_flags.len(), 2);
        assert!(addr_flags[0].temporary);
        assert!(addr_flags[1].permanent);
    }

    #[test]
    fn prefers_more_specific_sysfs_types() {
        assert!(type_is_more_specific(
            InterfaceType::Ethernet,
            InterfaceType::Wireless80211
        ));
        assert!(type_is_more_specific(
            InterfaceType::Wwan,
            InterfaceType::Wwanpp
        ));
        assert!(!type_is_more_specific(
            InterfaceType::Tunnel,
            InterfaceType::Wireless80211
        ));
    }

    #[test]
    fn marks_ambiguous_types() {
        assert!(type_is_ambiguous(InterfaceType::Unknown));
        assert!(type_is_ambiguous(InterfaceType::UnknownWithValue(999)));
        assert!(type_is_ambiguous(InterfaceType::Ethernet));
        assert!(!type_is_ambiguous(InterfaceType::Tunnel));
    }
}
