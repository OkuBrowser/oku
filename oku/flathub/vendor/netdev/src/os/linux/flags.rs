use crate::interface::interface::Interface;
pub use libc::IFF_LOWER_UP;

pub fn is_physical_interface(interface: &Interface) -> bool {
    !interface.is_loopback() && !super::sysfs::is_virtual_interface(&interface.name)
}
