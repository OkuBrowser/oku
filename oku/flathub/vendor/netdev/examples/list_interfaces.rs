// This example shows all interfaces and their properties.

fn main() {
    let interfaces = netdev::get_interfaces();
    for interface in interfaces {
        println!("Interface:");
        println!("\tIndex: {}", interface.index);
        println!("\tName: {}", interface.name);
        println!("\tFriendly Name: {:?}", interface.friendly_name);
        println!("\tDescription: {:?}", interface.description);
        println!("\tType: {}", interface.if_type.name());
        println!("\tFlags: {:?}", interface.flags);
        println!("\t\tis UP {}", interface.is_up());
        println!("\t\tis LOOPBACK {}", interface.is_loopback());
        println!("\t\tis MULTICAST {}", interface.is_multicast());
        println!("\t\tis BROADCAST {}", interface.is_broadcast());
        println!("\t\tis POINT TO POINT {}", interface.is_point_to_point());
        println!("\t\tis TUN {}", interface.is_tun());
        println!("\t\tis RUNNING {}", interface.is_running());
        println!("\t\tis PHYSICAL {}", interface.is_physical());
        println!("\tOperational state: {:?}", interface.oper_state);
        if let Some(mac_addr) = interface.mac_addr {
            println!("\tMAC Address: {}", mac_addr);
        } else {
            println!("\tMAC Address: (Failed to get mac address)");
        }
        println!("\tIPv4: {:?}", interface.ipv4);

        // Print IPv6 addresses with scope ID and per-address flags
        for (i, ipv6) in interface.ipv6.iter().enumerate() {
            let scope_id = interface.ipv6_scope_ids.get(i).copied().unwrap_or(0);
            let flags = interface
                .ipv6_addr_flags
                .get(i)
                .copied()
                .unwrap_or_default();
            let mut flag_strs = Vec::new();
            if flags.deprecated {
                flag_strs.push("deprecated");
            }
            if flags.temporary {
                flag_strs.push("temporary");
            }
            if flags.tentative {
                flag_strs.push("tentative");
            }
            if flags.duplicated {
                flag_strs.push("duplicated");
            }
            if flags.permanent {
                flag_strs.push("permanent");
            }
            let flag_str = if flag_strs.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flag_strs.join(", "))
            };
            println!("\tIPv6: {:?}%{}{}", ipv6, scope_id, flag_str);
        }

        println!("\tTransmit Speed: {:?}", interface.transmit_speed);
        println!("\tReceive Speed: {:?}", interface.receive_speed);
        println!("\tAuto-negotiate: {:?}", interface.auto_negotiate);
        println!("\tDHCPv4 enabled: {:?}", interface.dhcp_v4_enabled);
        println!("\tDHCPv6 enabled: {:?}", interface.dhcp_v6_enabled);
        println!("\tStats: {:?}", interface.stats);
        #[cfg(feature = "gateway")]
        if let Some(gateway) = interface.gateway {
            println!("Gateway");
            println!("\tMAC Address: {}", gateway.mac_addr);
            println!("\tIPv4 Address: {:?}", gateway.ipv4);
            println!("\tIPv6 Address: {:?}", gateway.ipv6);
        } else {
            println!("Gateway: (Not found)");
        }
        #[cfg(feature = "gateway")]
        println!("DNS Servers: {:?}", interface.dns_servers);
        println!("MTU: {:?}", interface.mtu);
        #[cfg(feature = "gateway")]
        println!("Default: {}", interface.default);
        println!();
    }
}
