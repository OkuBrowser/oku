use crate::interface::interface::Interface;

const SIOCGIFFLAGS: libc::c_ulong = 0xc0206911;

pub fn is_physical_interface(interface: &Interface) -> bool {
    interface.is_up() && interface.is_running() && !interface.is_tun() && !interface.is_loopback()
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub fn get_interface_flags(if_name: &str) -> std::io::Result<u32> {
    use libc::{AF_INET, SOCK_DGRAM, c_char, ioctl, socket};
    use std::mem;
    use std::os::unix::io::RawFd;
    use std::ptr;

    #[cfg(target_os = "netbsd")]
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct IfReq {
        ifr_name: [c_char; libc::IFNAMSIZ],
        ifru_flags: [libc::c_short; 2],
    }

    #[cfg(not(target_os = "netbsd"))]
    use libc::ifreq as IfReq;

    let sock: RawFd = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
    if sock < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let mut ifr: IfReq = unsafe { mem::zeroed() };

    let ifname_c = std::ffi::CString::new(if_name).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    let bytes = ifname_c.as_bytes_with_nul();

    if bytes.len() > ifr.ifr_name.len() {
        unsafe { libc::close(sock) };
        return Err(std::io::ErrorKind::InvalidInput.into());
    }

    unsafe {
        ptr::copy_nonoverlapping(
            bytes.as_ptr() as *const c_char,
            ifr.ifr_name.as_mut_ptr(),
            bytes.len(),
        );
    }

    let res = unsafe { ioctl(sock, SIOCGIFFLAGS, &mut ifr) };
    unsafe { libc::close(sock) };

    if res < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        #[cfg(any(target_vendor = "apple", target_os = "openbsd"))]
        {
            Ok(unsafe { ifr.ifr_ifru.ifru_flags as u32 })
        }

        #[cfg(target_os = "netbsd")]
        {
            Ok(ifr.ifru_flags[0] as u32)
        }

        #[cfg(not(any(target_vendor = "apple", target_os = "netbsd", target_os = "openbsd")))]
        {
            Ok(unsafe { ifr.ifr_ifru.ifru_flags[0] as u32 })
        }
    }
}
