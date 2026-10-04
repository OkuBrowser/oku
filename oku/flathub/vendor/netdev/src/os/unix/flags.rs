pub use libc::{IFF_BROADCAST, IFF_LOOPBACK, IFF_MULTICAST, IFF_POINTOPOINT, IFF_RUNNING, IFF_UP};

use crate::interface::interface::Interface;

pub fn is_running(interface: &Interface) -> bool {
    interface.flags & (IFF_RUNNING as u32) != 0
}
