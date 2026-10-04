use crate::net::mac::MacAddr;
use core::mem;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

pub(crate) enum SockaddrRef<'a> {
    V4(&'a libc::sockaddr_in),
    V6(&'a libc::sockaddr_in6),
}

impl<'a> SockaddrRef<'a> {
    /// # Safety
    /// - `sa` must point to a valid, readable memory region of at least `len` bytes.
    /// - `sa` must point to the beginning of a `sockaddr`-compatible structure
    pub(crate) unsafe fn from_raw(sa: *const libc::sockaddr, len: libc::socklen_t) -> Option<Self> {
        if sa.is_null() || len < mem::size_of::<libc::sa_family_t>() as libc::socklen_t {
            return None;
        }
        let family = unsafe { (*sa).sa_family as libc::c_int };

        match family {
            libc::AF_INET => {
                if len as usize >= mem::size_of::<libc::sockaddr_in>() {
                    let sa: &libc::sockaddr_in = unsafe { &*(sa as *const libc::sockaddr_in) };
                    Some(SockaddrRef::V4(sa))
                } else {
                    None
                }
            }
            libc::AF_INET6 => {
                if len as usize >= mem::size_of::<libc::sockaddr_in6>() {
                    let sa: &libc::sockaddr_in6 = unsafe { &*(sa as *const libc::sockaddr_in6) };
                    Some(SockaddrRef::V6(sa))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn family(&self) -> libc::c_int {
        match self {
            SockaddrRef::V4(_) => libc::AF_INET,
            SockaddrRef::V6(_) => libc::AF_INET6,
        }
    }

    #[inline]
    pub(crate) fn to_ip(&self) -> IpAddr {
        match self {
            SockaddrRef::V4(sin) => {
                let addr = u32::from_be((*sin).sin_addr.s_addr as u32);
                IpAddr::V4(Ipv4Addr::from(addr))
            }
            SockaddrRef::V6(sin6) => {
                let bytes = (*sin6).sin6_addr.s6_addr;
                IpAddr::V6(Ipv6Addr::from(bytes))
            }
        }
    }

    #[inline]
    pub(crate) fn to_ipv6_scope(&self) -> Option<u32> {
        match self {
            SockaddrRef::V4(_) => None,
            SockaddrRef::V6(sin6) => {
                let scope = (*sin6).sin6_scope_id as u32;
                Some(scope)
            }
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn port(&self) -> u16 {
        match self {
            SockaddrRef::V4(sin) => u16::from_be((*sin).sin_port),
            SockaddrRef::V6(sin6) => u16::from_be((*sin6).sin6_port),
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn to_socket_addr(&self) -> SocketAddr {
        let ip = self.to_ip();
        let port = self.port();
        match self {
            SockaddrRef::V4(_) => SocketAddr::V4(SocketAddrV4::new(
                match ip {
                    IpAddr::V4(v4) => v4,
                    _ => unreachable!(),
                },
                port,
            )),
            SockaddrRef::V6(_) => {
                let scope = self.to_ipv6_scope().unwrap_or(0);
                let port = self.port();
                SocketAddr::V6(SocketAddrV6::new(
                    match ip {
                        IpAddr::V6(v6) => v6,
                        _ => unreachable!(),
                    },
                    port,
                    0,
                    scope,
                ))
            }
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn netmask_ip(&self) -> IpAddr {
        match self {
            SockaddrRef::V4(sin) => {
                let m = u32::from_be((*sin).sin_addr.s_addr as u32);
                IpAddr::V4(Ipv4Addr::from(m))
            }
            SockaddrRef::V6(sin6) => {
                let bytes = (*sin6).sin6_addr.s6_addr;
                IpAddr::V6(Ipv6Addr::from(bytes))
            }
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn netmask_prefix(&self) -> Option<u8> {
        match self {
            SockaddrRef::V4(sin) => {
                let m = u32::from_be((*sin).sin_addr.s_addr as u32);
                mask_to_prefix_u32(m)
            }
            SockaddrRef::V6(sin6) => {
                let bytes = (*sin6).sin6_addr.s6_addr;
                mask_to_prefix_128(&bytes)
            }
        }
    }
}

#[inline]
fn mask_to_prefix_u32(m: u32) -> Option<u8> {
    if m == 0 {
        return Some(0);
    }
    let leading = m.leading_ones();
    if (m << leading) == 0 {
        Some(leading as u8)
    } else {
        None
    }
}

#[inline]
fn mask_to_prefix_128(b: &[u8; 16]) -> Option<u8> {
    let mut count = 0u8;
    let mut flipped = false;
    for byte in b {
        if *byte == 0xFF {
            if flipped {
                return None;
            }
            count += 8;
        } else if *byte == 0 {
            flipped = true;
        } else {
            if flipped {
                return None;
            }
            let ones = byte.leading_ones() as u8;
            if (*byte << ones) != 0 {
                return None;
            }
            count += ones;
            flipped = true;
        }
    }
    Some(count)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) unsafe fn try_mac_from_raw(
    sa: *const libc::sockaddr,
    len: libc::socklen_t,
) -> Option<MacAddr> {
    if sa.is_null() || (len as usize) < core::mem::size_of::<libc::sa_family_t>() {
        return None;
    }

    let family = unsafe { (*sa).sa_family as libc::c_int };
    if family != libc::AF_PACKET {
        return None;
    }

    if (len as usize) < core::mem::size_of::<libc::sockaddr_ll>() {
        return None;
    }

    let sll = unsafe { &*(sa as *const libc::sockaddr_ll) };

    let halen = sll.sll_halen as usize;
    if halen < 6 {
        return None;
    }

    Some(MacAddr::from_octets([
        sll.sll_addr[0],
        sll.sll_addr[1],
        sll.sll_addr[2],
        sll.sll_addr[3],
        sll.sll_addr[4],
        sll.sll_addr[5],
    ]))
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) unsafe fn try_mac_from_raw(
    sa: *const libc::sockaddr,
    len: libc::socklen_t,
) -> Option<MacAddr> {
    // Validate pointer and minimal length (must contain sa_family)
    if sa.is_null() || (len as usize) < core::mem::size_of::<libc::sa_family_t>() {
        return None;
    }

    let sa_dr = unsafe { *sa };
    if sa_dr.sa_family as libc::c_int != libc::AF_LINK {
        return None;
    }

    // Length sanity check
    let sa_len = sa_dr.sa_len as usize;
    if sa_len == 0 {
        return None;
    }
    let eff_len = core::cmp::min(sa_len, len as usize);
    if eff_len < 8 {
        return None;
    }

    // Access sa_data region (starts after sa_len + sa_family)
    let base = sa as *const u8;
    let sa_data = unsafe { base.add(2) };
    // Offsets inside sockaddr_dl
    let nlen = unsafe { *sa_data.add(3) } as usize;
    let alen = unsafe { *sa_data.add(4) } as usize;

    if alen < 6 {
        return None;
    }
    // LLADDR is located at an offset of 6 + nlen bytes from the beginning of sa_data
    // (6 bytes correspond to index(2) + type(1) + nlen(1) + alen(1) + slen(1)).
    let lladdr_off_in_data = 6usize + nlen;
    let mac_end_in_data = lladdr_off_in_data + 6;

    // The usable length of sa_data is (eff_len - 2), excluding sa_len and sa_family.
    if mac_end_in_data > eff_len.saturating_sub(2) {
        return None;
    }

    let mac_ptr = unsafe { sa_data.add(lladdr_off_in_data) };
    unsafe {
        Some(MacAddr(
            *mac_ptr.add(0),
            *mac_ptr.add(1),
            *mac_ptr.add(2),
            *mac_ptr.add(3),
            *mac_ptr.add(4),
            *mac_ptr.add(5),
        ))
    }
}

/// Computes the effective length of a `sockaddr` structure
#[inline]
pub(crate) unsafe fn compute_sockaddr_len(
    sa: *const libc::sockaddr,
    provided_len: Option<libc::socklen_t>,
    capacity_hint: Option<libc::socklen_t>,
) -> Option<libc::socklen_t> {
    if sa.is_null() {
        return None;
    }

    // 1) Use caller-provided explicit length if available.
    if let Some(mut len) = provided_len {
        if let Some(cap) = capacity_hint {
            if len > cap {
                len = cap;
            }
        }
        // Must be at least large enough to include sa_family_t.
        if len >= core::mem::size_of::<libc::sa_family_t>() as libc::socklen_t {
            return Some(len);
        }
    }

    let sa_dr = unsafe { *sa };

    // 2) BSD / Darwin platforms store sa_len as the first byte of sockaddr.
    #[cfg(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let len = sa_dr.sa_len as libc::socklen_t;
        if len >= core::mem::size_of::<libc::sa_family_t>() as libc::socklen_t {
            let len = if let Some(cap) = capacity_hint {
                len.min(cap)
            } else {
                len
            };
            return Some(len);
        }
    }

    // 3) Fallback: Guess length from address family (for Linux / Windows).
    let fam = sa_dr.sa_family as libc::c_int;
    let guessed = guess_len_from_family(fam)?;
    let guessed = if let Some(cap) = capacity_hint {
        guessed.min(cap)
    } else {
        guessed
    };

    Some(guessed)
}

/// Estimates the appropriate size of a `sockaddr` structure based on its address family.
#[inline]
fn guess_len_from_family(family: libc::c_int) -> Option<libc::socklen_t> {
    // IPv4 / IPv6 are universally consistent across all OSes.
    if family == libc::AF_INET {
        return Some(core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t);
    }
    if family == libc::AF_INET6 {
        return Some(core::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t);
    }

    // Linux / Android: Layer 2 (AF_PACKET)
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if family == libc::AF_PACKET {
        return Some(core::mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t);
    }

    // Darwin / BSD: Layer 2 (AF_LINK)
    #[cfg(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    if family == libc::AF_LINK {
        // sockaddr_dl is variable-length.
        // Return the minimum structure size; the true length should be read from sa_len.
        return Some(core::mem::size_of::<libc::sockaddr_dl>() as libc::socklen_t);
    }

    // Unknown or unsupported family
    None
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[inline]
pub(crate) unsafe fn netmask_ip_autolen(sa: *const libc::sockaddr) -> Option<IpAddr> {
    if sa.is_null() {
        return None;
    }

    match unsafe { (*sa).sa_family as libc::c_int } {
        libc::AF_INET => {
            let sin = unsafe { &*(sa as *const libc::sockaddr_in) };
            let n = u32::from_be(sin.sin_addr.s_addr as u32);
            Some(IpAddr::V4(Ipv4Addr::from(n)))
        }
        libc::AF_INET6 => {
            let sin6 = unsafe { &*(sa as *const libc::sockaddr_in6) };
            let bytes = sin6.sin6_addr.s6_addr; // [u8; 16]
            Some(IpAddr::V6(Ipv6Addr::from(bytes)))
        }
        _ => None,
    }
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
/// Reads a netmask `IpAddr` from a BSD/Darwin `sockaddr` whose actual length is encoded in `sa_len`.
#[inline]
pub(crate) unsafe fn netmask_ip_autolen(sa: *const libc::sockaddr) -> Option<IpAddr> {
    if sa.is_null() {
        return None;
    }

    // BSD/Darwin carry the effective length in the first byte (sa_len).
    let sa_dr = unsafe { *sa };
    let sa_len = sa_dr.sa_len as usize;
    if sa_len == 0 {
        // Zero-length sockaddr (e.g., default route). Treat as no mask.
        return None;
    }

    let fam = sa_dr.sa_family as libc::c_int;
    let base = sa as *const u8;

    match fam {
        libc::AF_INET => {
            // Layout on BSD/Darwin:
            //   offset 0: sa_len (1)
            //   offset 1: sa_family (1)
            //   offset 2..3: sin_port (2)
            //   offset 4..7: sin_addr (4)
            const OFF_SIN_ADDR: usize = 4;
            if sa_len <= OFF_SIN_ADDR {
                return Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
            }
            // Copy up to 4 bytes; tolerate short/truncated structures.
            let n = (sa_len - OFF_SIN_ADDR).min(4);

            let mut bytes = [0u8; 4];
            unsafe {
                core::ptr::copy_nonoverlapping(base.add(OFF_SIN_ADDR), bytes.as_mut_ptr(), n)
            };
            Some(IpAddr::V4(Ipv4Addr::from(u32::from_be_bytes(bytes))))
        }
        libc::AF_INET6 => {
            // Layout on BSD/Darwin:
            //   offset 0:  sa_len (1)
            //   offset 1:  sa_family (1)
            //   offset 2..3:  sin6_port (2)
            //   offset 4..7:  sin6_flowinfo (4)
            //   offset 8..23: sin6_addr (16)
            const OFF_SIN6_ADDR: usize = 8;
            if sa_len <= OFF_SIN6_ADDR {
                return Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED));
            }
            // Copy up to 16 bytes; tolerate short/truncated structures.
            let n = (sa_len - OFF_SIN6_ADDR).min(16);

            let mut bytes = [0u8; 16];
            unsafe {
                core::ptr::copy_nonoverlapping(base.add(OFF_SIN6_ADDR), bytes.as_mut_ptr(), n)
            };
            Some(IpAddr::V6(Ipv6Addr::from(bytes)))
        }
        _ => None,
    }
}

#[allow(dead_code)]
#[cfg(any(target_os = "linux", target_os = "android"))]
#[inline]
pub(crate) unsafe fn netmask_prefix_autolen(sa: *const libc::sockaddr) -> Option<u8> {
    if sa.is_null() {
        return None;
    }

    match unsafe { (*sa).sa_family as libc::c_int } {
        libc::AF_INET => {
            let sin = unsafe { &*(sa as *const libc::sockaddr_in) };
            let m = u32::from_be(sin.sin_addr.s_addr as u32);
            Some(m.leading_ones() as u8)
        }
        libc::AF_INET6 => {
            let sin6 = unsafe { &*(sa as *const libc::sockaddr_in6) };
            // [u8; 16]
            let bytes = sin6.sin6_addr.s6_addr;
            Some(u128::from_be_bytes(bytes).leading_ones() as u8)
        }
        _ => None,
    }
}

#[allow(dead_code)]
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
#[inline]
pub(crate) unsafe fn netmask_prefix_autolen(sa: *const libc::sockaddr) -> Option<u8> {
    if sa.is_null() {
        return None;
    }
    let sa_dr = unsafe { *sa };
    let sa_len = sa_dr.sa_len as usize;
    if sa_len == 0 {
        return Some(0);
    }

    let fam = sa_dr.sa_family as libc::c_int;
    let base = sa as *const u8;

    match fam {
        libc::AF_INET => {
            const OFF_SIN_ADDR: usize = 4;
            if sa_len <= OFF_SIN_ADDR {
                return Some(0);
            }
            let n = (sa_len - OFF_SIN_ADDR).min(4);

            let mut bytes = [0u8; 4];
            unsafe {
                core::ptr::copy_nonoverlapping(base.add(OFF_SIN_ADDR), bytes.as_mut_ptr(), n);
            }
            Some(u32::from_be_bytes(bytes).leading_ones() as u8)
        }
        libc::AF_INET6 => {
            const OFF_SIN6_ADDR: usize = 8;
            if sa_len <= OFF_SIN6_ADDR {
                return Some(0);
            }
            let n = (sa_len - OFF_SIN6_ADDR).min(16);

            let mut bytes = [0u8; 16];
            unsafe {
                core::ptr::copy_nonoverlapping(base.add(OFF_SIN6_ADDR), bytes.as_mut_ptr(), n);
            }
            Some(u128::from_be_bytes(bytes).leading_ones() as u8)
        }
        _ => None,
    }
}

#[inline]
pub(crate) fn sockaddr_storage_cap() -> libc::socklen_t {
    mem::size_of::<libc::sockaddr_storage>().try_into().unwrap()
}
