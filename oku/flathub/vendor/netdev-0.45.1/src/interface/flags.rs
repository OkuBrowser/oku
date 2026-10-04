#[cfg(target_family = "unix")]
pub use crate::os::unix::flags::*;

#[cfg(target_os = "linux")]
pub use crate::os::linux::flags::*;

#[cfg(target_os = "android")]
pub use crate::os::android::flags::*;

#[cfg(target_vendor = "apple")]
pub use crate::os::darwin::flags::*;

#[cfg(target_os = "windows")]
pub use crate::os::windows::flags::*;

#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub use crate::os::bsd::flags::*;
