#![allow(non_camel_case_types)]

use libc::{c_char, c_int, c_uchar, pid_t, size_t};
use std::{
    collections::HashMap,
    ffi::c_void,
    io, mem,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
};

use crate::net::{device::NetworkDevice, mac::MacAddr};

const CTL_NET: c_int = 4;
#[allow(dead_code)]
const AF_ROUTE: c_int = 17;
const PF_ROUTE: c_int = 17;
const AF_LINK: c_int = 18;
const AF_INET: c_int = 2;
const AF_INET6: c_int = 30;

const NET_RT_DUMP: c_int = 1;
const NET_RT_FLAGS: c_int = 2;

const RTM_VERSION: c_uchar = 5;
const RTF_LLINFO: c_int = 1024;

// rtm_flags
#[allow(dead_code)]
const RTF_UP: c_int = 0x0001;
const RTF_GATEWAY: c_int = 0x0002;
const RTF_HOST: c_int = 0x0004;
#[allow(dead_code)]
const RTF_REJECT: c_int = 0x0008;
#[allow(dead_code)]
const RTF_STATIC: c_int = 0x0800;
const RTF_WASCLONED: c_int = 0x20000;

// rtm_addrs
const RTAX_DST: usize = 0;
const RTAX_GATEWAY: usize = 1;
const RTAX_NETMASK: usize = 2;
const RTAX_MAX: usize = 8;

// sockaddr alignment
const SA_ALIGN: usize = 4;

#[repr(C)]
#[derive(Copy, Clone)]
struct sockaddr {
    sa_len: c_uchar,
    sa_family: c_uchar,
    sa_data: [c_char; 14],
}
#[repr(C)]
#[derive(Copy, Clone)]
struct in_addr {
    s_addr: u32,
}
#[repr(C)]
#[derive(Copy, Clone)]
struct in6_addr {
    __u6_addr: in6_addr_bind,
}
#[repr(C)]
#[derive(Copy, Clone)]
union in6_addr_bind {
    __u6_addr8: [u8; 16],
    __u6_addr16: [u16; 8],
    __u6_addr32: [u32; 4],
}
#[repr(C)]
#[derive(Copy, Clone)]
struct sockaddr_in {
    sin_len: c_uchar,
    sin_family: c_uchar,
    sin_port: u16,
    sin_addr: in_addr,
    sin_zero: [c_char; 8],
}
#[repr(C)]
#[derive(Copy, Clone)]
struct sockaddr_in6 {
    sin6_len: c_uchar,
    sin6_family: c_uchar,
    sin6_port: u16,
    sin6_flowinfo: u32,
    sin6_addr: in6_addr,
    sin6_scope_id: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct rt_metrics {
    rmx_locks: u32,
    rmx_mtu: u32,
    rmx_hopcount: u32,
    rmx_expire: i32,
    rmx_recvpipe: u32,
    rmx_sendpipe: u32,
    rmx_ssthresh: u32,
    rmx_rtt: u32,
    rmx_rttvar: u32,
    rmx_pksent: u32,
    rmx_state: u32,
    rmx_filler: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct rt_msghdr {
    rtm_msglen: u16,
    rtm_version: u8,
    rtm_type: u8,
    rtm_index: u16,
    rtm_flags: c_int,
    rtm_addrs: c_int,
    rtm_pid: pid_t,
    rtm_seq: c_int,
    rtm_errno: c_int,
    rtm_use: c_int,
    rtm_inits: u32,
    rtm_rmx: rt_metrics,
}

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
        SA_ALIGN
    } else {
        (len + (SA_ALIGN - 1)) & !(SA_ALIGN - 1)
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
fn ip_from_sockaddr(sa: &sockaddr) -> Option<IpAddr> {
    unsafe {
        match sa.sa_family as c_int {
            AF_INET => {
                let sin = &*(sa as *const _ as *const libc::sockaddr_in);
                let n = u32::from_be(sin.sin_addr.s_addr as u32);
                Some(IpAddr::V4(Ipv4Addr::from(n)))
            }
            AF_INET6 => {
                // Require the full `sockaddr_in6` to be present.
                let sin6 = &*(sa as *const _ as *const libc::sockaddr_in6);
                let want = core::mem::size_of::<libc::sockaddr_in6>();
                if (sa.sa_len as usize) < want {
                    // prevent reading a truncated variable-length sockaddr
                    return None;
                }
                // `s6_addr` is raw big-endian bytes; `Ipv6Addr::from([u8;16])` expects octets.
                let addr_bytes = (*sin6).sin6_addr.s6_addr;
                Some(IpAddr::V6(Ipv6Addr::from(addr_bytes)))
            }
            _ => None,
        }
    }
}

fn masklen_from_sockaddr(dst: IpAddr, mask_sa: &sockaddr) -> u8 {
    unsafe {
        match dst {
            IpAddr::V4(_) => {
                let m: &sockaddr_in = &*(mask_sa as *const _ as *const sockaddr_in);
                u32::from_be(m.sin_addr.s_addr).leading_ones() as u8
            }
            IpAddr::V6(_) => {
                let m: &sockaddr_in6 = &*(mask_sa as *const _ as *const sockaddr_in6);
                let b = m.sin6_addr.__u6_addr.__u6_addr8;
                let v = u128::from_be_bytes(b);
                v.leading_ones() as u8
            }
        }
    }
}

fn code_to_error(err: i32) -> io::Error {
    let kind = match err {
        17 => io::ErrorKind::AlreadyExists, // EEXIST
        3 => io::ErrorKind::NotFound,       // ESRCH
        3436 => io::ErrorKind::OutOfMemory, // ENOBUFS
        _ => io::ErrorKind::Other,
    };

    io::Error::new(kind, format!("rtm_errno {}", err))
}

/// Extract `(IP, MAC)` pair from a routing message's address block.
fn message_to_arppair(msg: &[u8]) -> Option<(IpAddr, MacAddr)> {
    let mut off = 0usize;
    let mut ip: Option<Ipv4Addr> = None;
    let mut mac: Option<MacAddr> = None;
    // Walk `sockaddr` records while there is room for a header.
    while off + core::mem::size_of::<sockaddr>() <= msg.len() {
        // Read the sockaddr header
        let sa = unsafe { &*(msg[off..].as_ptr() as *const sockaddr) };
        let sa_len = sa.sa_len as usize;

        // `sa_len == 0` can appear as "no address" (alignment-only slot).
        // Advance by the platform's alignment unit (4 on BSD/Darwin).
        if sa_len == 0 {
            off += roundup(0);
            continue;
        }
        // If the element claims to extend past the buffer, skip it conservatively.
        if off + sa_len > msg.len() {
            off += roundup(sa_len);
            continue;
        }

        match sa.sa_family as c_int {
            AF_INET => {
                // Target IPv4 of ARP. `sockaddr_in` and `sockaddr_inarp` share the initial layout,
                // so `sin_addr` sits at the same position.
                if let Some(IpAddr::V4(v4)) = ip_from_sockaddr(sa) {
                    ip = Some(v4);
                    if let (Some(v4), Some(m)) = (ip, mac) {
                        return Some((IpAddr::V4(v4), m));
                    }
                }
            }
            AF_LINK => {
                // Extract LLADDR from `sockaddr_dl`.
                let sdl = unsafe { &*(sa as *const _ as *const libc::sockaddr_dl) };
                let nlen = sdl.sdl_nlen as usize;
                let alen = sdl.sdl_alen as usize;
                let total = sdl.sdl_len as usize;

                // Validate against the *actual* struct length (`sdl_len`), and also
                // make sure the caller-provided `sa_len` is at least that long.
                if total >= core::mem::size_of::<libc::sockaddr_dl>()
                    && alen >= 6
                    && sa_len >= total
                {
                    let base = sa as *const _ as *const u8;
                    let data_base = &sdl.sdl_data as *const _ as *const u8;
                    let data_off = unsafe { data_base.offset_from(base) } as usize;

                    // LLADDR is at `sdl_data + sdl_nlen`.
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

        // Advance to the next record; BSD/Darwin sockaddrs are 4-byte aligned.
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
    let mut addrs: [Option<*const sockaddr>; RTAX_MAX] = [None; RTAX_MAX];
    let mut off = 0usize;

    for idx in 0..RTAX_MAX {
        if (hdr.rtm_addrs & (1 << idx)) != 0 {
            if off + mem::size_of::<sockaddr>() > addr_block.len() {
                return None;
            }
            let sa = unsafe { &*(addr_block[off..].as_ptr() as *const sockaddr) };
            addrs[idx] = Some(sa as *const sockaddr);

            let sa_len = sa.sa_len as usize;
            let step = roundup(if sa_len == 0 { 0 } else { sa_len });
            if off + step > addr_block.len() {
                return None;
            }
            off += step;
        }
    }

    let dptr = addrs[RTAX_DST]? as *const sockaddr;
    let dst_sa = unsafe { &*dptr };
    let dst_ip = ip_from_sockaddr(dst_sa)?;
    let mut prefix: u8 = match dst_ip {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if let Some(mptr) = addrs[RTAX_NETMASK] {
        let m_sa = unsafe { &*mptr };
        // sa_len==0 is possible for default route
        prefix = if m_sa.sa_len == 0 {
            0
        } else {
            masklen_from_sockaddr(dst_ip, m_sa)
        };
    } else if (hdr.rtm_flags & RTF_HOST) != 0 {
        prefix = match dst_ip {
            IpAddr::V4(_) => 32,
            _ => 128,
        };
    }

    // gateway
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
    // sysctl net.route dump for ARP/neighbor entries (IPv4 only here).
    let mut mib = [
        CTL_NET,      // net
        PF_ROUTE,     // route
        0,            // 0
        AF_INET,      // IPv4
        NET_RT_FLAGS, // flags
        RTF_LLINFO,   // ARP/neighbor entries
    ];
    // Includes ENOMEM retry internally; length is truncated to actual bytes read.
    let buf = sysctl_vec(&mut mib)?;

    let mut off = 0usize;
    // Each record starts with `rt_msghdr` followed by a variable-length sockaddr block.
    while off + mem::size_of::<rt_msghdr>() <= buf.len() {
        // Header view (no copy).
        let hdr = unsafe { &*(buf[off..].as_ptr() as *const rt_msghdr) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen == 0 || off + msglen > buf.len() {
            break;
        }

        // Version mismatch: skip the record but keep reading.
        if hdr.rtm_version != RTM_VERSION {
            off += msglen;
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(code_to_error(hdr.rtm_errno));
        }

        // Parse the sockaddr block right after the header.
        let addr_block = &buf[off + mem::size_of::<rt_msghdr>()..off + msglen];
        if let Some((ip, mac)) = message_to_arppair(addr_block) {
            arp_map.insert(ip, mac);
        }

        off += msglen;
    }

    Ok(arp_map)
}

/// Dump the routing table via `sysctl` on BSD/Darwin and parse each `rt_msghdr`.
fn list_routes() -> io::Result<Vec<RawRoute>> {
    // sysctl net.route dump (all families)
    let mut mib = [
        CTL_NET,     // net
        PF_ROUTE,    // route
        0,           // 0
        0,           // all families (0)
        NET_RT_DUMP, // dump routes
        0,
    ];
    // Includes ENOMEM retry internally; buffer is right-sized and truncated.
    let buf = sysctl_vec(&mut mib)?;

    let mut out = Vec::<RawRoute>::new();
    let mut off = 0usize;

    while off + mem::size_of::<rt_msghdr>() <= buf.len() {
        let hdr = unsafe { &*(buf[off..].as_ptr() as *const rt_msghdr) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen == 0 || off + msglen > buf.len() {
            break;
        }
        // Version mismatch: skip the record but keep reading.
        if hdr.rtm_version != RTM_VERSION {
            off += msglen;
            continue;
        }
        // Skip cloned/old-style cache entries; keep only real routes.
        if (hdr.rtm_flags & RTF_WASCLONED) != 0 {
            off += msglen;
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(code_to_error(hdr.rtm_errno));
        }

        let addr_block = &buf[off + mem::size_of::<rt_msghdr>()..off + msglen];
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
        if r.prefix != 0 || (r.flags & RTF_GATEWAY) == 0 {
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
