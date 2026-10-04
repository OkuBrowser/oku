use libc::{AF_INET, SIOCGIFMTU, SOCK_DGRAM, c_char, c_int, close, ifreq, ioctl, socket};
use std::ffi::CString;
use std::mem;
use std::os::unix::io::RawFd;
use std::ptr;

pub(crate) fn get_mtu(name: &str) -> Option<u32> {
    // Create a socket for ioctl operations
    let sock: RawFd = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
    if sock < 0 {
        eprintln!(
            "Failed to create socket: {:?}",
            std::io::Error::last_os_error()
        );
        return None;
    }

    let mut ifr: ifreq = unsafe { mem::zeroed() };

    // Set the interface name (must not exceed `IFNAMSIZ`)
    let c_interface = CString::new(name).ok()?;
    // Ensure null termination
    let bytes = c_interface.to_bytes_with_nul();
    if bytes.len() > ifr.ifr_name.len() {
        eprintln!("Interface name too long: {}", name);
        unsafe { close(sock) };
        return None;
    }

    unsafe {
        ptr::copy_nonoverlapping(
            bytes.as_ptr() as *const c_char,
            ifr.ifr_name.as_mut_ptr(),
            bytes.len(),
        );
    }

    // Retrieve the MTU using ioctl
    let ret: c_int = unsafe { ioctl(sock, SIOCGIFMTU as _, &mut ifr) };
    if ret < 0 {
        eprintln!(
            "ioctl(SIOCGIFMTU) failed for {}: {:?}",
            name,
            std::io::Error::last_os_error()
        );
        unsafe { close(sock) };
        return None;
    }

    let mtu = unsafe { ifr.ifr_ifru.ifru_mtu } as u32;

    // Close the socket
    unsafe { close(sock) };

    Some(mtu)
}
