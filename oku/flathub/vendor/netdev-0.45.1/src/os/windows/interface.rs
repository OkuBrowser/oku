use std::net::IpAddr;
use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_GATEWAYS, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    IP_ADAPTER_DHCP_ENABLED,
};
use windows_sys::Win32::NetworkManagement::Ndis::NET_IF_OPER_STATUS_UP;
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, IpDadStateDeprecated, IpDadStateDuplicate, IpDadStateTentative,
    IpSuffixOriginRandom, SOCKADDR_INET, SOCKET_ADDRESS,
};

use super::flags;
use super::macros::linked_list_iter;
use crate::interface::interface::Interface;
use crate::interface::ipv6_addr_flags::Ipv6AddrFlags;
use crate::interface::state::OperState;
use crate::interface::types::InterfaceType;
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::net::mac::MacAddr;
use crate::stats::counters::InterfaceStats;
use std::ffi::CStr;

#[cfg(feature = "gateway")]
use crate::net::device::NetworkDevice;
#[cfg(feature = "gateway")]
use crate::net::ip::get_local_ipaddr;
#[cfg(feature = "gateway")]
use std::mem::MaybeUninit;
#[cfg(feature = "gateway")]
use std::net::Ipv4Addr;
#[cfg(feature = "gateway")]
use windows_sys::Win32::NetworkManagement::IpHelper::SendARP;

fn sanitize_u64(val: u64) -> Option<u64> {
    if val == u64::MAX { None } else { Some(val) }
}

#[cfg(feature = "gateway")]
fn get_mac_through_arp(src_ip: Ipv4Addr, dst_ip: Ipv4Addr) -> MacAddr {
    let src_ip_int = u32::from_ne_bytes(src_ip.octets());
    let dst_ip_int = u32::from_ne_bytes(dst_ip.octets());
    let mut out_buf_len = 6;
    let mut target_mac_addr = MaybeUninit::<[u8; 6]>::uninit();
    let res = unsafe {
        SendARP(
            dst_ip_int,
            src_ip_int,
            target_mac_addr.as_mut_ptr().cast(),
            &mut out_buf_len,
        )
    };
    if res == NO_ERROR && out_buf_len == 6 {
        MacAddr::from_octets(unsafe { target_mac_addr.assume_init() })
    } else {
        MacAddr::zero()
    }
}

// Convert a socket address into a Rust IpAddr object and also a scope ID if it's an
// IPv6 address
unsafe fn socket_address_to_ipaddr(addr: &SOCKET_ADDRESS) -> (Option<IpAddr>, Option<u32>) {
    match unsafe { addr.lpSockaddr.cast::<SOCKADDR_INET>().as_ref() } {
        None => (None, None),
        Some(sockaddr) => match unsafe { sockaddr.si_family } {
            AF_INET => {
                let addr: IpAddr = unsafe { sockaddr.Ipv4.sin_addr.S_un.S_addr }
                    .to_ne_bytes()
                    .into();
                (Some(addr), None)
            }
            AF_INET6 => {
                let addr: IpAddr = unsafe { sockaddr.Ipv6.sin6_addr.u.Byte }.into();
                let scope_id = unsafe { sockaddr.Ipv6.Anonymous.sin6_scope_id };
                (Some(addr), Some(scope_id))
            }
            _ => (None, None),
        },
    }
}

unsafe fn from_wide_string(ptr: *const u16) -> String {
    let mut len = 0;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(ptr, len) })
}

// Get network interfaces using the IP Helper API
// Reference: https://docs.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses
pub fn interfaces() -> Vec<Interface> {
    #[cfg(feature = "gateway")]
    let local_ip: IpAddr = match get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    // "The recommended method of calling the GetAdaptersAddresses function is to pre-allocate a 15KB working buffer pointed to by the AdapterAddresses parameter."
    // (c) https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses
    let mut mem = Vec::<u8>::with_capacity(15000);
    let mut retries = 3;
    loop {
        let mut dwsize = mem.capacity() as u32;
        let ret_val = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_GATEWAYS,
                std::ptr::null_mut(),
                mem.as_mut_ptr().cast(),
                &mut dwsize,
            )
        };
        match ret_val {
            NO_ERROR => {
                unsafe {
                    mem.set_len(dwsize as usize);
                }
                break;
            }
            ERROR_BUFFER_OVERFLOW if retries > 0 => {
                mem.reserve(dwsize as usize);
                retries -= 1;
            }
            _ => {
                // TODO: return errors as a Result someday?
                return vec![];
            }
        }
    }
    // Enumerate all adapters
    let mem = mem.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    unsafe { linked_list_iter!(&mem) }
        .filter_map(|cur| {
            let if_type = InterfaceType::try_from(cur.IfType).ok()?;
            // Index
            let index = {
                let anon1 = cur.Anonymous1;
                let anon = unsafe { &anon1.Anonymous };
                anon.IfIndex
            };
            // Flags and Status
            let mut flags: u32 = 0;
            if cur.OperStatus == NET_IF_OPER_STATUS_UP {
                flags |= flags::IFF_UP;
            }
            match if_type {
                InterfaceType::Ethernet
                | InterfaceType::TokenRing
                | InterfaceType::Wireless80211
                | InterfaceType::HighPerformanceSerialBus => {
                    flags |= flags::IFF_BROADCAST | flags::IFF_MULTICAST;
                }
                InterfaceType::Ppp | InterfaceType::Tunnel => {
                    flags |= flags::IFF_POINTOPOINT | flags::IFF_MULTICAST;
                }
                InterfaceType::Loopback => {
                    flags |= flags::IFF_LOOPBACK | flags::IFF_MULTICAST;
                }
                InterfaceType::Atm => {
                    flags |= flags::IFF_BROADCAST | flags::IFF_POINTOPOINT | flags::IFF_MULTICAST;
                }
                _ => {}
            }

            let oper_state: OperState = match cur.OperStatus {
                1 => OperState::Up,
                2 => OperState::Down,
                3 => OperState::Testing,
                4 => OperState::Unknown,
                5 => OperState::Dormant,
                6 => OperState::NotPresent,
                7 => OperState::LowerLayerDown,
                _ => OperState::Unknown,
            };

            // Name
            let adapter_name = unsafe { CStr::from_ptr(cur.AdapterName.cast()) }
                .to_string_lossy()
                .into_owned();
            // MAC address
            let mac_addr = if cur.PhysicalAddressLength == 6 {
                Some(MacAddr::from_octets(
                    cur.PhysicalAddress[..6].try_into().unwrap(),
                ))
            } else {
                None
            };
            let mut ipv4_vec: Vec<Ipv4Net> = vec![];
            let mut ipv6_vec: Vec<Ipv6Net> = vec![];
            let mut ipv6_scope_id_vec: Vec<u32> = vec![];
            let mut ipv6_flags_vec: Vec<Ipv6AddrFlags> = vec![];
            // Enumerate all IPs
            for cur_a in unsafe { linked_list_iter!(&cur.FirstUnicastAddress) } {
                let (ip_addr, ipv6_scope_id) = unsafe { socket_address_to_ipaddr(&cur_a.Address) };

                let prefix_len = cur_a.OnLinkPrefixLength;
                match ip_addr {
                    Some(IpAddr::V4(ipv4)) => match Ipv4Net::new(ipv4, prefix_len) {
                        Ok(ipv4_net) => ipv4_vec.push(ipv4_net),
                        Err(_) => {}
                    },
                    Some(IpAddr::V6(ipv6)) => match Ipv6Net::new(ipv6, prefix_len) {
                        Ok(ipv6_net) => {
                            ipv6_vec.push(ipv6_net);
                            ipv6_scope_id_vec.push(ipv6_scope_id.unwrap());

                            ipv6_flags_vec.push(Ipv6AddrFlags {
                                deprecated: cur_a.DadState == IpDadStateDeprecated,
                                tentative: cur_a.DadState == IpDadStateTentative,
                                duplicated: cur_a.DadState == IpDadStateDuplicate,
                                temporary: cur_a.SuffixOrigin == IpSuffixOriginRandom,
                                permanent: false,
                            });
                        }
                        Err(_) => {}
                    },
                    None => {}
                }
            }
            // Gateway
            #[cfg(feature = "gateway")]
            let gateway_ips: Vec<IpAddr> = unsafe { linked_list_iter!(&cur.FirstGatewayAddress) }
                .filter_map(|cur_g| unsafe { socket_address_to_ipaddr(&cur_g.Address).0 })
                .collect();
            #[cfg(feature = "gateway")]
            let mut default_gateway: NetworkDevice = NetworkDevice::new();
            #[cfg(feature = "gateway")]
            if flags & flags::IFF_UP != 0 {
                for gateway_ip in gateway_ips {
                    match gateway_ip {
                        IpAddr::V4(ipv4) => {
                            if let Some(ip_net) = ipv4_vec.first() {
                                let mac_addr = get_mac_through_arp(ip_net.addr(), ipv4);
                                default_gateway.mac_addr = mac_addr;
                                default_gateway.ipv4.push(ipv4);
                            }
                        }
                        IpAddr::V6(ipv6) => {
                            if !ipv6_vec.is_empty() {
                                default_gateway.ipv6.push(ipv6);
                            }
                        }
                    }
                }
            }
            // DNS Servers
            #[cfg(feature = "gateway")]
            let dns_servers: Vec<IpAddr> = unsafe { linked_list_iter!(&cur.FirstDnsServerAddress) }
                .filter_map(|cur_d| unsafe { socket_address_to_ipaddr(&cur_d.Address).0 })
                .collect();
            #[cfg(feature = "gateway")]
            let default: bool = match local_ip {
                IpAddr::V4(local_ipv4) => ipv4_vec.iter().any(|x| x.addr() == local_ipv4),
                IpAddr::V6(local_ipv6) => ipv6_vec.iter().any(|x| x.addr() == local_ipv6),
            };
            let stats: Option<InterfaceStats> = super::stats::get_stats_from_index(index);
            let interface: Interface = Interface {
                index,
                name: adapter_name,
                friendly_name: Some(unsafe { from_wide_string(cur.FriendlyName) }),
                description: Some(unsafe { from_wide_string(cur.Description) }),
                if_type,
                mac_addr,
                ipv4: ipv4_vec,
                ipv6: ipv6_vec,
                ipv6_scope_ids: ipv6_scope_id_vec,
                ipv6_addr_flags: ipv6_flags_vec,
                flags,
                oper_state,
                transmit_speed: sanitize_u64(cur.TransmitLinkSpeed),
                receive_speed: sanitize_u64(cur.ReceiveLinkSpeed),
                auto_negotiate: None,
                dhcp_v4_enabled: Some(
                    unsafe { cur.Anonymous2.Flags } & IP_ADAPTER_DHCP_ENABLED != 0,
                ),
                dhcp_v6_enabled: None,
                stats,
                #[cfg(feature = "gateway")]
                gateway: if default_gateway.mac_addr == MacAddr::zero() {
                    None
                } else {
                    Some(default_gateway)
                },
                #[cfg(feature = "gateway")]
                dns_servers,
                mtu: Some(cur.Mtu),
                #[cfg(feature = "gateway")]
                default,
            };
            Some(interface)
        })
        .collect()
}
