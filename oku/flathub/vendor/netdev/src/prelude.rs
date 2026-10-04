/// Common imports for interface discovery and inspection.
pub use crate::interface::get_interfaces;
pub use crate::interface::interface::Interface;
pub use crate::interface::state::OperState;
pub use crate::interface::types::InterfaceType;
pub use crate::net::device::NetworkDevice;
pub use crate::net::mac::MacAddr;
pub use crate::stats::counters::InterfaceStats;
pub use ipnet::{Ipv4Net, Ipv6Net};

#[cfg(feature = "gateway")]
pub use crate::interface::get_default_interface;
#[cfg(feature = "gateway")]
pub use crate::route::get_default_gateway;
