#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn get_mtu(_ifa: &libc::ifaddrs, name: &str) -> Option<u32> {
    crate::os::linux::mtu::get_mtu(name)
}

#[cfg(target_vendor = "apple")]
pub(crate) use crate::os::darwin::mtu::*;

#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
pub(crate) use crate::os::bsd::mtu::*;
