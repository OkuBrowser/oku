//! Cross-platform library for enumerating network interfaces with metadata.
//!
//! `netdev` provides a unified API for discovering local network interfaces
//! and retrieving commonly used metadata across platforms.
//!
//! Main entry points:
//! - [`get_interfaces`] returns a snapshot of all visible interfaces.
//! - [`Interface`] represents one interface and its collected metadata.
//! - [`get_default_interface`] and [`get_default_gateway`] are available with the `gateway` feature (default).
//!
pub mod interface;
pub mod net;
mod os;
pub mod prelude;
#[cfg(feature = "gateway")]
pub mod route;
pub mod stats;

pub use ipnet;

pub use interface::get_interfaces;
pub use interface::interface::Interface;
pub use net::device::NetworkDevice;
pub use net::mac::MacAddr;

#[cfg(feature = "gateway")]
pub use interface::get_default_interface;
#[cfg(feature = "gateway")]
pub use route::get_default_gateway;
