#![allow(non_camel_case_types)]

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "netbsd")]
mod netbsd;
#[cfg(target_os = "openbsd")]
mod openbsd;

use libc::{c_int, size_t};
use std::{
    collections::HashMap,
    ffi::c_void,
    io, mem,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
};

#[cfg(target_os = "freebsd")]
use self::freebsd::{SOCKADDR_ALIGN, message_header_len, rt_msghdr};
#[cfg(target_os = "netbsd")]
use self::netbsd::{SOCKADDR_ALIGN, message_header_len, rt_msghdr};
#[cfg(target_os = "openbsd")]
use self::openbsd::{SOCKADDR_ALIGN, message_header_len, rt_msghdr};

use crate::net::{device::NetworkDevice, mac::MacAddr};

const CTL_NET: c_int = libc::CTL_NET;
const NET_RT_DUMP: c_int = 1;
const NET_RT_FLAGS: c_int = 2;

const RTF_LLINFO: u32 = 1024;
#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
const RTM_VERSION: u8 = 5;
#[cfg(target_os = "netbsd")]
const RTM_VERSION: u8 = 4;

const RTAX_DST: usize = 0;
const RTAX_GATEWAY: usize = 1;
const RTAX_NETMASK: usize = 2;

#[cfg(target_os = "freebsd")]
const RTAX_MAX: usize = 8;
#[cfg(target_os = "netbsd")]
const RTAX_MAX: usize = 9;
#[cfg(target_os = "openbsd")]
const RTAX_MAX: usize = 15;

unsafe extern "C" {
    fn sysctl(
        name: *mut c_int,
        namelen: u32,
        oldp: *mut c_void,
        oldlenp: *mut size_t,
        newp: *mut c_void,
        newlen: size_t,
    ) -> c_int;
}

/// Fetches a sysctl value into a Vec<u8>.
fn sysctl_vec(mib: &mut [c_int]) -> io::Result<Vec<u8>> {
    let mut len: size_t = 0;
    let mut r = unsafe {
        sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            ptr::null_mut(),
            &mut len,
            ptr::null_mut(),
            0,
        )
    };
    if r < 0 {
        return Err(io::Error::last_os_error());
    }

    let mut buf = vec![0u8; len as usize];
    r = unsafe {
        sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            buf.as_mut_ptr() as *mut c_void,
            &mut len,
            ptr::null_mut(),
            0,
        )
    };
    if r < 0 {
        // If the value grew, kernel returns ENOMEM. Retry once.
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ENOMEM) {
            let mut len2: size_t = 0;
            let r2 = unsafe {
                sysctl(
                    mib.as_mut_ptr(),
                    mib.len() as u32,
                    ptr::null_mut(),
                    &mut len2,
                    ptr::null_mut(),
                    0,
                )
            };
            if r2 < 0 {
                return Err(io::Error::last_os_error());
            }
            buf.resize(len2 as usize, 0);
            let r3 = unsafe {
                sysctl(
                    mib.as_mut_ptr(),
                    mib.len() as u32,
                    buf.as_mut_ptr() as *mut c_void,
                    &mut len2,
                    ptr::null_mut(),
                    0,
                )
            };
            if r3 < 0 {
                return Err(io::Error::last_os_error());
            }
            buf.truncate(len2 as usize);
            return Ok(buf);
        }
        return Err(err);
    }
    buf.truncate(len as usize);
    Ok(buf)
}

#[inline]
fn roundup(len: usize) -> usize {
    if len == 0 {
        SOCKADDR_ALIGN
    } else {
        (len + (SOCKADDR_ALIGN - 1)) & !(SOCKADDR_ALIGN - 1)
    }
}

#[inline]
fn normalize_scoped_v6(gw: Ipv6Addr) -> Ipv6Addr {
    // Unicast link-local: fe80::/10 (in practice often fe80::/64)
    let is_unicast_ll = gw.segments()[0] == 0xfe80;

    // Multicast check (ff00::/8) and local scopes: 0x1 (node-local) or 0x2 (link-local).
    let oct = gw.octets();
    let is_multicast = oct[0] == 0xff;
    let mscope = oct[1] & 0x0f;
    let is_local_scope_mc = is_multicast && (mscope == 0x1 || mscope == 0x2);

    if is_unicast_ll || is_local_scope_mc {
        let segs = gw.segments();
        Ipv6Addr::new(
            segs[0], 0, segs[2], segs[3], segs[4], segs[5], segs[6], segs[7],
        )
    } else {
        gw
    }
}

#[inline]
fn normalize_gateway(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => IpAddr::V6(normalize_scoped_v6(v6)),
    }
}

/// Parse an IP address from a `sockaddr`
fn ip_from_sockaddr(sa: &libc::sockaddr) -> Option<IpAddr> {
    unsafe {
        match sa.sa_family as libc::c_int {
            x if x == libc::AF_INET => {
                let want = core::mem::size_of::<libc::sockaddr_in>();
                if (sa.sa_len as usize) < want {
                    return None;
                }
                let sin = &*(sa as *const _ as *const libc::sockaddr_in);
                let n = u32::from_be(sin.sin_addr.s_addr as u32);
                Some(IpAddr::V4(Ipv4Addr::from(n)))
            }
            x if x == libc::AF_INET6 => {
                let want = core::mem::size_of::<libc::sockaddr_in6>();
                if (sa.sa_len as usize) < want {
                    return None;
                }
                let sin6 = &*(sa as *const _ as *const libc::sockaddr_in6);
                // `s6_addr` is raw big-endian bytes; `Ipv6Addr::from([u8;16])` expects octets.
                Some(IpAddr::V6(Ipv6Addr::from(sin6.sin6_addr.s6_addr)))
            }
            _ => None,
        }
    }
}

fn masklen_from_sockaddr(dst: IpAddr, mask_sa: &libc::sockaddr) -> u8 {
    unsafe {
        let sa_len = mask_sa.sa_len as usize;
        if sa_len == 0 {
            return 0;
        }

        match dst {
            IpAddr::V4(_) => {
                const OFF: usize = 4;
                if sa_len <= OFF {
                    return 0;
                }
                let n = (sa_len - OFF).min(4);

                let base = (mask_sa as *const _ as *const u8).add(OFF);
                let mut bytes = [0u8; 4];
                core::ptr::copy_nonoverlapping(base, bytes.as_mut_ptr(), n);
                u32::from_be_bytes(bytes).leading_ones() as u8
            }
            IpAddr::V6(_) => {
                const OFF: usize = 8;
                if sa_len <= OFF {
                    return 0;
                }
                let n = (sa_len - OFF).min(16);

                let base = (mask_sa as *const _ as *const u8).add(OFF);
                let mut bytes = [0u8; 16];
                core::ptr::copy_nonoverlapping(base, bytes.as_mut_ptr(), n);
                u128::from_be_bytes(bytes).leading_ones() as u8
            }
        }
    }
}

fn code_to_error(err: i32) -> io::Error {
    let kind = match err {
        x if x == libc::EEXIST => io::ErrorKind::AlreadyExists,
        x if x == libc::ESRCH => io::ErrorKind::NotFound,
        x if x == libc::ENOBUFS => io::ErrorKind::OutOfMemory,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, format!("rtm_errno {}", err))
}

/// Extract `(IP, MAC)` pair from a routing message's address block.
fn message_to_arppair(msg: &[u8]) -> Option<(IpAddr, MacAddr)> {
    let mut off = 0usize;
    let mut ip: Option<Ipv4Addr> = None;
    let mut mac: Option<MacAddr> = None;

    while off + core::mem::size_of::<libc::sockaddr>() <= msg.len() {
        let sa = unsafe { &*(msg[off..].as_ptr() as *const libc::sockaddr) };
        let sa_len = sa.sa_len as usize;

        if sa_len == 0 {
            off += roundup(0);
            continue;
        }
        if off + sa_len > msg.len() {
            off += roundup(sa_len);
            continue;
        }

        match sa.sa_family as c_int {
            x if x == libc::AF_INET => {
                if let Some(IpAddr::V4(v4)) = ip_from_sockaddr(sa) {
                    ip = Some(v4);
                    if let (Some(v4), Some(m)) = (ip, mac) {
                        return Some((IpAddr::V4(v4), m));
                    }
                }
            }
            x if x == libc::AF_LINK => {
                let sdl = unsafe { &*(sa as *const _ as *const libc::sockaddr_dl) };
                let nlen = sdl.sdl_nlen as usize;
                let alen = sdl.sdl_alen as usize;
                let total = sdl.sdl_len as usize;

                if total >= core::mem::size_of::<libc::sockaddr_dl>()
                    && alen >= 6
                    && sa_len >= total
                {
                    let base = sa as *const _ as *const u8;
                    let data_base = &sdl.sdl_data as *const _ as *const u8;
                    let data_off = unsafe { data_base.offset_from(base) } as usize;

                    if data_off + nlen + alen <= total {
                        let mac_ptr = unsafe { data_base.add(nlen) };
                        let m = MacAddr::from_octets(unsafe {
                            [
                                *mac_ptr.add(0),
                                *mac_ptr.add(1),
                                *mac_ptr.add(2),
                                *mac_ptr.add(3),
                                *mac_ptr.add(4),
                                *mac_ptr.add(5),
                            ]
                        });
                        mac = Some(m);
                        if let (Some(v4), Some(m)) = (ip, mac) {
                            return Some((IpAddr::V4(v4), m));
                        }
                    }
                }
            }
            _ => {}
        }

        off += roundup(sa_len);
    }

    None
}

#[derive(Debug)]
struct RawRoute {
    #[allow(dead_code)]
    dst: IpAddr,
    prefix: u8,
    gateway: Option<IpAddr>,
    ifindex: u32,
    #[allow(dead_code)]
    flags: c_int,
}

fn parse_one_route(hdr: &rt_msghdr, addr_block: &[u8]) -> Option<RawRoute> {
    let mut addrs: [Option<*const libc::sockaddr>; RTAX_MAX] = [None; RTAX_MAX];
    let mut off = 0usize;

    for idx in 0..RTAX_MAX {
        if (hdr.rtm_addrs & (1 << idx)) != 0 {
            if off + mem::size_of::<libc::sockaddr>() > addr_block.len() {
                return None;
            }
            let sa = unsafe { &*(addr_block[off..].as_ptr() as *const libc::sockaddr) };
            addrs[idx] = Some(sa as *const libc::sockaddr);

            let sa_len = sa.sa_len as usize;
            let step = roundup(if sa_len == 0 { 0 } else { sa_len });
            if off + step > addr_block.len() {
                return None;
            }
            off += step;
        }
    }

    let dptr = addrs[RTAX_DST]? as *const libc::sockaddr;
    let dst_sa = unsafe { &*dptr };
    let dst_ip = ip_from_sockaddr(dst_sa)?;
    let mut prefix: u8 = match dst_ip {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if let Some(mptr) = addrs[RTAX_NETMASK] {
        let m_sa = unsafe { &*mptr };
        prefix = if m_sa.sa_len == 0 {
            0
        } else {
            masklen_from_sockaddr(dst_ip, m_sa)
        };
    } else if (hdr.rtm_flags & libc::RTF_HOST) != 0 {
        prefix = match dst_ip {
            IpAddr::V4(_) => 32,
            _ => 128,
        };
    }

    let gateway = if let Some(gptr) = addrs[RTAX_GATEWAY] {
        let g_sa = unsafe { &*gptr };
        ip_from_sockaddr(g_sa).map(normalize_gateway)
    } else {
        None
    };

    Some(RawRoute {
        dst: dst_ip,
        prefix,
        gateway,
        ifindex: hdr.rtm_index as u32,
        flags: hdr.rtm_flags,
    })
}

/// Build an ARP/Neighbor table from the BSD/Darwin routing socket via `sysctl`.
fn get_arp_table() -> io::Result<HashMap<IpAddr, MacAddr>> {
    let mut arp_map: HashMap<IpAddr, MacAddr> = HashMap::new();

    let mut mib = [
        CTL_NET,
        libc::PF_ROUTE,
        0,
        libc::AF_INET,
        NET_RT_FLAGS,
        RTF_LLINFO as i32, // ARP/neighbor entries
    ];
    let buf = sysctl_vec(&mut mib)?;

    let mut off = 0usize;
    while off + mem::size_of::<rt_msghdr>() <= buf.len() {
        let hdr = unsafe { &*(buf[off..].as_ptr() as *const rt_msghdr) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen == 0 || off + msglen > buf.len() {
            break;
        }

        if hdr.rtm_version != RTM_VERSION {
            off += msglen;
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(code_to_error(hdr.rtm_errno));
        }

        let hdrlen = message_header_len(hdr);
        if hdrlen < mem::size_of::<rt_msghdr>() || off + hdrlen > off + msglen {
            break;
        }
        let addr_block = &buf[off + hdrlen..off + msglen];
        if let Some((ip, mac)) = message_to_arppair(addr_block) {
            arp_map.insert(ip, mac);
        }

        off += msglen;
    }

    Ok(arp_map)
}

/// Dump the routing table via `sysctl` on BSD/Darwin and parse each `rt_msghdr`.
fn list_routes() -> io::Result<Vec<RawRoute>> {
    let mut mib = [
        CTL_NET,
        libc::PF_ROUTE,
        0,
        0,           // all families
        NET_RT_DUMP, // dump routes
        0,
    ];
    let buf = sysctl_vec(&mut mib)?;

    let mut out = Vec::<RawRoute>::new();
    let mut off = 0usize;

    while off + mem::size_of::<rt_msghdr>() <= buf.len() {
        let hdr = unsafe { &*(buf[off..].as_ptr() as *const rt_msghdr) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen == 0 || off + msglen > buf.len() {
            break;
        }

        if hdr.rtm_version != RTM_VERSION {
            off += msglen;
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(code_to_error(hdr.rtm_errno));
        }

        let hdrlen = message_header_len(hdr);
        if hdrlen < mem::size_of::<rt_msghdr>() || off + hdrlen > off + msglen {
            break;
        }
        let addr_block = &buf[off + hdrlen..off + msglen];
        if let Some(rr) = parse_one_route(hdr, addr_block) {
            out.push(rr);
        }

        off += msglen;
    }

    Ok(out)
}

/// Build a map `ifindex -> NetworkDevice` for default gateways on BSD/Darwin.
pub fn get_gateway_map() -> HashMap<u32, NetworkDevice> {
    // Fetch routes; on failure just return an empty map.
    let routes = match list_routes() {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    // ARP cache: IP -> MAC (empty if ARP cannot be read)
    let arp_map = get_arp_table().unwrap_or_default();

    // Accumulator: ifindex -> (optional MAC candidate, v4 list, v6 list)
    #[derive(Default)]
    struct Acc {
        mac: Option<MacAddr>,
        v4: Vec<Ipv4Addr>,
        v6: Vec<Ipv6Addr>,
    }
    let mut acc: HashMap<u32, Acc> = HashMap::new();

    // Small helpers to avoid duplicates
    fn push_v4(vec: &mut Vec<Ipv4Addr>, ip: Ipv4Addr) {
        if !vec.iter().any(|x| *x == ip) {
            vec.push(ip);
        }
    }
    fn push_v6(vec: &mut Vec<Ipv6Addr>, ip: Ipv6Addr) {
        if !vec.iter().any(|x| *x == ip) {
            vec.push(ip);
        }
    }

    // Collect gateway candidates per outgoing interface
    for r in routes.into_iter() {
        // Only gateway routes (prefix==0) and those flagged as GATEWAY
        if r.prefix != 0 || (r.flags & libc::RTF_GATEWAY) == 0 {
            continue;
        }
        let gw = match r.gateway {
            Some(ip) => ip,
            None => continue,
        };

        let entry = acc.entry(r.ifindex).or_default();

        // If this is an IPv4 gateway and ARP has the MAC, record it.
        if let Some(mac) = arp_map.get(&gw).copied() {
            entry.mac = Some(mac);
        }

        match gw {
            IpAddr::V4(v4) => {
                push_v4(&mut entry.v4, v4);
            }
            IpAddr::V6(v6) => {
                push_v6(&mut entry.v6, v6);
            }
        }
    }

    // Shape the final output: ifindex -> NetworkDevice
    let mut out: HashMap<u32, NetworkDevice> = HashMap::new();
    for (ifindex, a) in acc {
        // If MAC is still unknown, use a zero MAC
        // TODO: Implement NDP lookup for IPv6
        let mac = a.mac.unwrap_or_else(|| MacAddr::zero());
        out.insert(
            ifindex,
            NetworkDevice {
                mac_addr: mac,
                ipv4: a.v4,
                ipv6: a.v6,
            },
        );
    }

    out
}
