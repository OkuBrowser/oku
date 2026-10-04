#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::interface::types::InterfaceType;

use std::ffi::CStr;
use std::os::raw::c_char;

pub(crate) fn interface_name_from_ptr(c_str: *const c_char) -> String {
    unsafe { CStr::from_ptr(c_str) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn get_interface_type(addr_ref: &libc::ifaddrs) -> InterfaceType {
    let c_str = addr_ref.ifa_name as *const c_char;
    let name = interface_name_from_ptr(c_str);
    #[cfg(target_os = "linux")]
    {
        crate::os::linux::sysfs::get_interface_type(&name)
    }
    #[cfg(target_os = "android")]
    {
        crate::os::android::types::guess_type_by_name(&name).unwrap_or(InterfaceType::Unknown)
    }
}

#[cfg(target_vendor = "apple")]
pub use crate::os::darwin::types::get_interface_type;

#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
pub use crate::os::bsd::types::get_interface_type;
