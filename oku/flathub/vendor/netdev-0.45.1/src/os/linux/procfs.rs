use crate::net::device::NetworkDevice;
use crate::net::mac::MacAddr;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

const PATH_PROC_NET_ROUTE: &str = "/proc/net/route";
const PATH_PROC_NET_IPV6_ROUTE: &str = "/proc/net/ipv6_route";
const PATH_PROC_NET_ARP: &str = "/proc/net/arp";

fn convert_hex_ipv4(hex_ip: &str) -> Ipv4Addr {
    if hex_ip.len() != 8 {
        return Ipv4Addr::UNSPECIFIED;
    }
    let o1: u8 = u8::from_str_radix(&hex_ip[6..8], 0x10).unwrap_or(0);
    let o2: u8 = u8::from_str_radix(&hex_ip[4..6], 0x10).unwrap_or(0);
    let o3: u8 = u8::from_str_radix(&hex_ip[2..4], 0x10).unwrap_or(0);
    let o4: u8 = u8::from_str_radix(&hex_ip[0..2], 0x10).unwrap_or(0);
    Ipv4Addr::new(o1, o2, o3, o4)
}

fn convert_hex_ipv6(hex_ip: &str) -> Ipv6Addr {
    if hex_ip.len() != 32 {
        return Ipv6Addr::UNSPECIFIED;
    }
    let h1: u16 = u16::from_str_radix(&hex_ip[0..4], 0x10).unwrap_or(0);
    let h2: u16 = u16::from_str_radix(&hex_ip[4..8], 0x10).unwrap_or(0);
    let h3: u16 = u16::from_str_radix(&hex_ip[8..12], 0x10).unwrap_or(0);
    let h4: u16 = u16::from_str_radix(&hex_ip[12..16], 0x10).unwrap_or(0);
    let h5: u16 = u16::from_str_radix(&hex_ip[16..20], 0x10).unwrap_or(0);
    let h6: u16 = u16::from_str_radix(&hex_ip[20..24], 0x10).unwrap_or(0);
    let h7: u16 = u16::from_str_radix(&hex_ip[24..28], 0x10).unwrap_or(0);
    let h8: u16 = u16::from_str_radix(&hex_ip[28..32], 0x10).unwrap_or(0);
    Ipv6Addr::new(h1, h2, h3, h4, h5, h6, h7, h8)
}

fn get_arp_map() -> HashMap<Ipv4Addr, MacAddr> {
    let mut arp_map: HashMap<Ipv4Addr, MacAddr> = HashMap::new();
    if let Ok(arp_text) = read_to_string(PATH_PROC_NET_ARP) {
        for row in arp_text.lines() {
            let mut fields = row.split_whitespace();
            let Some(ip_addr) = fields.next() else {
                continue;
            };
            let _hw_type = fields.next();
            let _flags = fields.next();
            let Some(mac_addr) = fields.next() else {
                continue;
            };
            if let Ok(ipv4_addr) = Ipv4Addr::from_str(ip_addr) {
                arp_map.insert(ipv4_addr, MacAddr::from_hex_format(mac_addr));
            }
        }
    }
    arp_map
}

fn get_ipv4_gateway_map() -> HashMap<String, Ipv4Addr> {
    let mut ipv4_gateway_map: HashMap<String, Ipv4Addr> = HashMap::new();
    if let Ok(route_text) = read_to_string(PATH_PROC_NET_ROUTE) {
        for row in route_text.lines() {
            let mut fields = row.split_whitespace();
            let Some(if_name) = fields.next() else {
                continue;
            };
            let _destination = fields.next();
            let Some(gateway) = fields.next() else {
                continue;
            };
            if gateway != "00000000" {
                ipv4_gateway_map.insert(if_name.to_owned(), convert_hex_ipv4(gateway));
            }
        }
    }
    ipv4_gateway_map
}

fn get_ipv6_gateway_map() -> HashMap<String, Ipv6Addr> {
    let mut ipv6_gateway_map: HashMap<String, Ipv6Addr> = HashMap::new();
    if let Ok(route_text) = read_to_string(PATH_PROC_NET_IPV6_ROUTE) {
        for row in route_text.lines() {
            let fields: Vec<&str> = row.split_whitespace().collect();
            if fields.len() < 10 {
                continue;
            }
            // default route has zero destination and zero prefix length
            if fields[0] == "00000000000000000000000000000000"
                && fields[1] == "00"
                && fields[4] != "00000000000000000000000000000000"
            {
                ipv6_gateway_map.insert(fields[9].to_owned(), convert_hex_ipv6(fields[4]));
            }
        }
    }
    ipv6_gateway_map
}

pub fn get_gateway_map() -> HashMap<String, NetworkDevice> {
    let mut gateway_map: HashMap<String, NetworkDevice> = HashMap::new();
    let arp_map: HashMap<Ipv4Addr, MacAddr> = get_arp_map();
    for (if_name, ipv4_addr) in get_ipv4_gateway_map() {
        let gateway = gateway_map.entry(if_name).or_insert(NetworkDevice::new());
        if let Some(mac_addr) = arp_map.get(&ipv4_addr) {
            gateway.mac_addr = *mac_addr;
        }
        gateway.ipv4.push(ipv4_addr);
    }
    for (if_name, ipv6_addr) in get_ipv6_gateway_map() {
        let gateway = gateway_map.entry(if_name).or_insert(NetworkDevice::new());
        gateway.ipv6.push(ipv6_addr);
    }
    gateway_map
}
