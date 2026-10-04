pub mod arp;
#[cfg(not(target_os = "android"))]
mod dhcp;
#[cfg(not(target_os = "android"))]
pub mod flags;
#[cfg(not(target_os = "android"))]
pub mod interface;
pub mod ipv6_addr_flags;
pub mod mtu;
#[cfg(not(target_os = "android"))]
pub mod netlink;
#[cfg(not(target_os = "android"))]
#[cfg(feature = "gateway")]
pub mod procfs;
#[cfg(not(target_os = "android"))]
pub mod state;
#[cfg(not(target_os = "android"))]
pub mod sysfs;
