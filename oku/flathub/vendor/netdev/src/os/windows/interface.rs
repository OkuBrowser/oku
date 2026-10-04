use std::net::IpAddr;
use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_GATEWAYS, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    IP_ADAPTER_DHCP_ENABLED,
};
use windows_sys::Win32::NetworkManagement::Ndis::NET_IF_OPER_STATUS_UP;
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, IpDadStateDeprecated, IpDadStateDuplicate, IpDadStateTentative,
    IpSuffixOriginRandom, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_INET, SOCKET_ADDRESS,
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
use std::sync::OnceLock;
#[cfg(feature = "gateway")]
use windows_sys::Win32::Foundation::WIN32_ERROR;
#[cfg(feature = "gateway")]
use windows_sys::Win32::NetworkManagement::IpHelper::{MIB_IPNET_ROW2, SendARP};
#[cfg(feature = "gateway")]
use windows_sys::Win32::NetworkManagement::Ndis::NET_LUID_LH;
#[cfg(feature = "gateway")]
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

fn sanitize_u64(val: u64) -> Option<u64> {
    if val == u64::MAX { None } else { Some(val) }
}

#[cfg(feature = "gateway")]
fn get_mac_through_arp(src_ip: Ipv4Addr, dst_ip: Ipv4Addr) -> Option<MacAddr> {
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
        Some(MacAddr::from_octets(unsafe {
            target_mac_addr.assume_init()
        }))
    } else {
        None
    }
}

#[derive(Clone, Copy)]
struct ParsedSocketAddress {
    ip_addr: IpAddr,
    ipv6_scope_id: Option<u32>,
    sockaddr: SOCKADDR_INET,
}

// Copy the family-specific value because an IPv4 SOCKET_ADDRESS can be shorter than
// SOCKADDR_INET, while an IPv6 gateway also needs to retain its scope ID.
unsafe fn parse_socket_address(addr: &SOCKET_ADDRESS) -> Option<ParsedSocketAddress> {
    if addr.lpSockaddr.is_null()
        || addr.iSockaddrLength < std::mem::size_of::<u16>().try_into().unwrap()
    {
        return None;
    }

    let family = unsafe { std::ptr::read_unaligned(addr.lpSockaddr.cast::<u16>()) };
    match family {
        AF_INET
            if usize::try_from(addr.iSockaddrLength).ok()?
                >= std::mem::size_of::<SOCKADDR_IN>() =>
        {
            let ipv4 = unsafe { std::ptr::read_unaligned(addr.lpSockaddr.cast::<SOCKADDR_IN>()) };
            Some(ParsedSocketAddress {
                ip_addr: IpAddr::V4(unsafe { ipv4.sin_addr.S_un.S_addr }.to_ne_bytes().into()),
                ipv6_scope_id: None,
                sockaddr: SOCKADDR_INET { Ipv4: ipv4 },
            })
        }
        AF_INET6
            if usize::try_from(addr.iSockaddrLength).ok()?
                >= std::mem::size_of::<SOCKADDR_IN6>() =>
        {
            let ipv6 = unsafe { std::ptr::read_unaligned(addr.lpSockaddr.cast::<SOCKADDR_IN6>()) };
            Some(ParsedSocketAddress {
                ip_addr: IpAddr::V6(unsafe { ipv6.sin6_addr.u.Byte }.into()),
                ipv6_scope_id: Some(unsafe { ipv6.Anonymous.sin6_scope_id }),
                sockaddr: SOCKADDR_INET { Ipv6: ipv6 },
            })
        }
        _ => None,
    }
}

unsafe fn socket_address_to_ipaddr(addr: &SOCKET_ADDRESS) -> (Option<IpAddr>, Option<u32>) {
    match unsafe { parse_socket_address(addr) } {
        Some(parsed) => (Some(parsed.ip_addr), parsed.ipv6_scope_id),
        None => (None, None),
    }
}

#[cfg(feature = "gateway")]
fn physical_address_to_mac(address: &[u8], length: u32) -> Option<MacAddr> {
    if length != 6 || address.len() < 6 {
        return None;
    }
    Some(MacAddr::from_octets(address[..6].try_into().unwrap()))
}

#[cfg(feature = "gateway")]
type GetIpNetEntry2 = unsafe extern "system" fn(*mut MIB_IPNET_ROW2) -> WIN32_ERROR;

/// Resolve `GetIpNetEntry2`, which some `iphlpapi.dll` implementations do not export.
#[cfg(feature = "gateway")]
fn get_ip_net_entry2() -> Option<GetIpNetEntry2> {
    static ENTRY: OnceLock<Option<GetIpNetEntry2>> = OnceLock::new();
    *ENTRY.get_or_init(|| {
        // `iphlpapi.dll` is already loaded: this module statically imports `GetAdaptersAddresses`.
        let module = unsafe { GetModuleHandleA(c"iphlpapi.dll".as_ptr().cast()) };
        if module.is_null() {
            return None;
        }
        let proc = unsafe { GetProcAddress(module, c"GetIpNetEntry2".as_ptr().cast()) }?;
        // SAFETY: the export matches the `GetIpNetEntry2` signature.
        Some(unsafe {
            std::mem::transmute::<unsafe extern "system" fn() -> isize, GetIpNetEntry2>(proc)
        })
    })
}

#[cfg(feature = "gateway")]
fn get_neighbor_mac(address: SOCKADDR_INET, interface_luid: NET_LUID_LH) -> Option<MacAddr> {
    let get_ip_net_entry2 = get_ip_net_entry2()?;
    let mut row = MIB_IPNET_ROW2 {
        Address: address,
        InterfaceLuid: interface_luid,
        ..Default::default()
    };
    let result = unsafe { get_ip_net_entry2(&mut row) };
    if result != NO_ERROR {
        return None;
    }
    physical_address_to_mac(&row.PhysicalAddress, row.PhysicalAddressLength)
}

#[cfg(feature = "gateway")]
#[derive(Default)]
struct GatewayCandidates {
    ipv4: Vec<Ipv4Addr>,
    ipv6: Vec<std::net::Ipv6Addr>,
    ipv4_mac: Option<MacAddr>,
    ipv6_mac: Option<MacAddr>,
}

#[cfg(feature = "gateway")]
impl GatewayCandidates {
    fn add_ipv4(&mut self, address: Ipv4Addr, mac: Option<MacAddr>) {
        if !self.ipv4.contains(&address) {
            self.ipv4.push(address);
        }
        if self.ipv4_mac.is_none() {
            self.ipv4_mac = mac;
        }
    }

    fn add_ipv6(&mut self, address: std::net::Ipv6Addr, mac: Option<MacAddr>) {
        if !self.ipv6.contains(&address) {
            self.ipv6.push(address);
        }
        if self.ipv6_mac.is_none() {
            self.ipv6_mac = mac;
        }
    }

    fn into_device(self) -> Option<NetworkDevice> {
        if self.ipv4.is_empty() && self.ipv6.is_empty() {
            return None;
        }
        Some(NetworkDevice {
            mac_addr: self
                .ipv4_mac
                .or(self.ipv6_mac)
                .unwrap_or_else(MacAddr::zero),
            ipv4: self.ipv4,
            ipv6: self.ipv6,
        })
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
            let gateway_addresses: Vec<ParsedSocketAddress> =
                unsafe { linked_list_iter!(&cur.FirstGatewayAddress) }
                    .filter_map(|cur_g| unsafe { parse_socket_address(&cur_g.Address) })
                    .collect();
            #[cfg(feature = "gateway")]
            let mut gateway_candidates = GatewayCandidates::default();
            #[cfg(feature = "gateway")]
            if flags & flags::IFF_UP != 0 {
                for gateway in gateway_addresses {
                    let neighbor_mac = get_neighbor_mac(gateway.sockaddr, cur.Luid);
                    match gateway.ip_addr {
                        IpAddr::V4(ipv4) => {
                            let mac = neighbor_mac.or_else(|| {
                                ipv4_vec
                                    .first()
                                    .and_then(|source| get_mac_through_arp(source.addr(), ipv4))
                            });
                            gateway_candidates.add_ipv4(ipv4, mac);
                        }
                        IpAddr::V6(ipv6) => {
                            gateway_candidates.add_ipv6(ipv6, neighbor_mac);
                        }
                    }
                }
            }
            #[cfg(feature = "gateway")]
            let default_gateway = gateway_candidates.into_device();
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
                gateway: default_gateway,
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

#[cfg(all(test, feature = "gateway"))]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use windows_sys::Win32::Networking::WinSock::{IN6_ADDR, SOCKADDR, SOCKADDR_IN6_0};

    const IPV4_MAC: [u8; 6] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
    const IPV6_MAC: [u8; 6] = [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb];

    #[test]
    fn builds_ipv6_only_gateway_with_resolved_mac() {
        let address = "fe80::1".parse().unwrap();
        let mac = MacAddr::from_octets(IPV6_MAC);
        let mut candidates = GatewayCandidates::default();
        candidates.add_ipv6(address, Some(mac));

        let gateway = candidates.into_device().unwrap();
        assert_eq!(gateway.mac_addr, mac);
        assert!(gateway.ipv4.is_empty());
        assert_eq!(gateway.ipv6, vec![address]);
    }

    #[test]
    fn preserves_unresolved_ipv6_only_gateway() {
        let address = "fe80::1".parse().unwrap();
        let mut candidates = GatewayCandidates::default();
        candidates.add_ipv6(address, None);

        let gateway = candidates.into_device().unwrap();
        assert_eq!(gateway.mac_addr, MacAddr::zero());
        assert!(gateway.ipv4.is_empty());
        assert_eq!(gateway.ipv6, vec![address]);
    }

    #[test]
    fn prefers_ipv4_mac_regardless_of_enumeration_order() {
        let ipv4 = Ipv4Addr::new(192, 0, 2, 1);
        let ipv6 = "fe80::1".parse().unwrap();
        let ipv4_mac = MacAddr::from_octets(IPV4_MAC);
        let ipv6_mac = MacAddr::from_octets(IPV6_MAC);
        let mut candidates = GatewayCandidates::default();
        candidates.add_ipv6(ipv6, Some(ipv6_mac));
        candidates.add_ipv4(ipv4, Some(ipv4_mac));

        let gateway = candidates.into_device().unwrap();
        assert_eq!(gateway.mac_addr, ipv4_mac);
        assert_eq!(gateway.ipv4, vec![ipv4]);
        assert_eq!(gateway.ipv6, vec![ipv6]);
    }

    #[test]
    fn builds_ipv4_only_gateway() {
        let address = Ipv4Addr::new(192, 0, 2, 1);
        let mac = MacAddr::from_octets(IPV4_MAC);
        let mut candidates = GatewayCandidates::default();
        candidates.add_ipv4(address, Some(mac));

        let gateway = candidates.into_device().unwrap();
        assert_eq!(gateway.mac_addr, mac);
        assert_eq!(gateway.ipv4, vec![address]);
        assert!(gateway.ipv6.is_empty());
    }

    #[test]
    fn deduplicates_gateway_addresses() {
        let ipv4 = Ipv4Addr::new(192, 0, 2, 1);
        let ipv6 = "fe80::1".parse().unwrap();
        let mut candidates = GatewayCandidates::default();
        candidates.add_ipv4(ipv4, None);
        candidates.add_ipv4(ipv4, None);
        candidates.add_ipv6(ipv6, None);
        candidates.add_ipv6(ipv6, None);

        let gateway = candidates.into_device().unwrap();
        assert_eq!(gateway.ipv4, vec![ipv4]);
        assert_eq!(gateway.ipv6, vec![ipv6]);
    }

    #[test]
    fn rejects_invalid_physical_address_lengths() {
        assert_eq!(physical_address_to_mac(&IPV4_MAC, 5), None);
        assert_eq!(physical_address_to_mac(&IPV4_MAC, 7), None);
        assert_eq!(physical_address_to_mac(&IPV4_MAC[..5], 6), None);
        assert_eq!(
            physical_address_to_mac(&IPV4_MAC, 6),
            Some(MacAddr::from_octets(IPV4_MAC))
        );
    }

    #[test]
    fn retains_scoped_link_local_ipv6_sockaddr() {
        let address: Ipv6Addr = "fe80::1".parse().unwrap();
        let mut sockaddr = SOCKADDR_IN6 {
            sin6_family: AF_INET6,
            sin6_addr: IN6_ADDR {
                u: windows_sys::Win32::Networking::WinSock::IN6_ADDR_0 {
                    Byte: address.octets(),
                },
            },
            Anonymous: SOCKADDR_IN6_0 { sin6_scope_id: 17 },
            ..Default::default()
        };
        let socket_address = SOCKET_ADDRESS {
            lpSockaddr: (&mut sockaddr as *mut SOCKADDR_IN6).cast::<SOCKADDR>(),
            iSockaddrLength: std::mem::size_of::<SOCKADDR_IN6>() as i32,
        };

        let parsed = unsafe { parse_socket_address(&socket_address) }.unwrap();
        assert_eq!(parsed.ip_addr, IpAddr::V6(address));
        assert_eq!(parsed.ipv6_scope_id, Some(17));
        assert_eq!(unsafe { parsed.sockaddr.Ipv6.Anonymous.sin6_scope_id }, 17);
    }
}
