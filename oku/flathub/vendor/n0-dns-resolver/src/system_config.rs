//! Reading the host system's DNS configuration.
//!
//! The source is platform-specific: `/etc/resolv.conf` on Unix, the
//! SystemConfiguration framework on Apple platforms, the network adapters on
//! Windows, and a Java Native Interface (JNI) call on Android. The per-platform
//! readers live in the submodules; this module dispatches to them via
//! [`read_system`]. The [`Config`] they produce lives in [`crate::config`].

use tracing::warn;

use super::{DnsProtocol, Nameserver, config::Config};

#[cfg(any(target_os = "android", doc))]
mod android;
#[cfg(target_vendor = "apple")]
mod apple;
mod hosts;
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(any(target_os = "android", doc))]
pub use android::install_android_jni_context;
#[cfg(target_os = "android")]
use android::read_system_dns;
#[cfg(target_vendor = "apple")]
use apple::read_system_dns;
pub(crate) use hosts::Hosts;
#[cfg(all(unix, not(any(target_os = "android", target_vendor = "apple"))))]
use unix::read_system_dns;
#[cfg(windows)]
use windows::read_system_dns;

/// Reads the host system's DNS configuration using the platform-specific reader.
///
/// A reader failure (a missing or unreadable `/etc/resolv.conf`, an
/// uninitialized JNI context on Android) is logged and yields an otherwise-empty
/// configuration, so the resolver falls back to public resolvers. The hosts file
/// is still read in that case, since a missing resolv.conf does not imply a
/// missing hosts file.
pub(crate) fn read_system() -> Config {
    match read_system_dns() {
        Ok(config) => config,
        Err(err) => {
            warn!(%err, "failed to read system DNS configuration, using fallback");
            Config {
                hosts: Hosts::from_system(),
                ..Default::default()
            }
        }
    }
}
