#[cfg(target_family = "unix")]
pub mod unix;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod linux;

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_vendor = "apple")]
pub mod darwin;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
pub mod bsd;
