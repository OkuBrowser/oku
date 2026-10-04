use super::netlink;
use crate::interface::interface::Interface;
use crate::interface::state::OperState;
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::net::mac::MacAddr;
use crate::os::unix::interface::unix_interfaces;
use std::net::{Ipv4Addr, Ipv6Addr};

#[cfg(feature = "gateway")]
use crate::net::device::NetworkDevice;
#[cfg(feature = "gateway")]
use crate::os::unix::dns::get_system_dns_conf;
#[cfg(feature = "gateway")]
use std::collections::HashMap;

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

fn push_ipv6(v: &mut Vec<Ipv6Net>, add: (Ipv6Addr, u8)) -> bool {
    if v.iter()
        .any(|n| n.addr() == add.0 && n.prefix_len() == add.1)
    {
        return false;
    }
    if let Ok(net) = Ipv6Net::new(add.0, add.1) {
        v.push(net);
        return true;
    }
    false
}

#[inline]
fn calc_v6_scope_id(addr: &Ipv6Addr, ifindex: u32) -> u32 {
    let seg0 = addr.segments()[0];
    if (seg0 & 0xffc0) == 0xfe80 {
        ifindex
    } else {
        0
    }
}

pub fn interfaces() -> Vec<Interface> {
    let mut ifaces = Vec::new();
    // Fill ifaces via netlink first
    // If netlink fails, fallback to unix_interfaces
    match netlink::collect_interfaces() {
        Ok(rows) => {
            for r in rows {
                let name = r.name.clone();
                let mut iface = Interface {
                    index: r.index,
                    name: name.clone(),
                    friendly_name: None,
                    description: None,
                    if_type: super::sysfs::get_interface_type(&name),
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
                    stats: None,
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
                    if push_ipv6(&mut iface.ipv6, (a, p)) {
                        iface.ipv6_scope_ids.push(calc_v6_scope_id(&a, iface.index));
                        let raw = r.ipv6_addr_flags.get(i).copied().unwrap_or(0);
                        iface
                            .ipv6_addr_flags
                            .push(super::ipv6_addr_flags::from_netlink_flags(raw));
                    }
                }

                ifaces.push(iface);
            }
        }
        Err(_) => {
            // Fallback: unix ifaddrs
            ifaces = unix_interfaces();
        }
    }

    // Fill gateway info if feature enabled
    #[cfg(feature = "gateway")]
    match netlink::collect_routes() {
        Ok(mut gmap) => {
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
        Err(_) => {
            // Fallback: procfs
            let mut gateway_map: HashMap<String, NetworkDevice> = super::procfs::get_gateway_map();
            for iface in &mut ifaces {
                if let Some(gateway) = gateway_map.remove(&iface.name) {
                    iface.gateway = Some(gateway);
                }
            }
        }
    }

    // Fill other info
    for iface in &mut ifaces {
        iface.if_type = super::sysfs::get_interface_type(&iface.name);
        let if_speed = super::sysfs::get_interface_speed(&iface.name);
        iface.transmit_speed = if_speed;
        iface.receive_speed = if_speed;
        let dhcp = super::dhcp::dhcp_state(&iface.name, iface.index);
        iface.dhcp_v4_enabled = dhcp.v4;
        iface.dhcp_v6_enabled = dhcp.v6;
        iface.oper_state = super::sysfs::operstate(&iface.name);

        if iface.stats.is_none() {
            iface.stats = crate::stats::counters::get_stats_from_name(&iface.name);
        }

        if iface.mtu.is_none() {
            iface.mtu = super::mtu::get_mtu(&iface.name);
        }
    }
    #[cfg(feature = "gateway")]
    {
        if let Some(local_ip) = crate::net::ip::get_local_ipaddr() {
            if let Some(idx) = crate::interface::pick_default_iface_index(&ifaces, local_ip) {
                if let Some(iface) = ifaces.iter_mut().find(|it| it.index == idx) {
                    iface.default = true;
                    iface.dns_servers = get_system_dns_conf();
                }
            }
        }
    }
    ifaces
}
