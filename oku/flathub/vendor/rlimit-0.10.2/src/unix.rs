use crate::bindings as C;
use crate::resource::Resource;

use std::{io, mem};

/// A value indicating no limit.
#[allow(clippy::unnecessary_cast)]
pub const INFINITY: u64 = C::RLIM_INFINITY as u64;

fn check_supported(resource: Resource) -> io::Result<()> {
    let raw_resource = resource.as_raw();
    if raw_resource == u8::MAX {
        return Err(io::Error::new(io::ErrorKind::Other, "unsupported resource"));
    }
    Ok(())
}

/// Set resource limits.
/// # Errors
/// \[Linux\] See <https://man7.org/linux/man-pages/man2/setrlimit.2.html>
#[allow(clippy::unnecessary_min_or_max)]
#[inline]
pub fn setrlimit(resource: Resource, soft: u64, hard: u64) -> io::Result<()> {
    check_supported(resource)?;
    let rlim = C::rlimit {
        rlim_cur: soft.min(INFINITY) as _,
        rlim_max: hard.min(INFINITY) as _,
    };
    #[allow(clippy::cast_lossless)]
    let ret = unsafe { C::setrlimit(resource.as_raw() as _, &rlim) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Get resource limits.
/// # Errors
/// \[Linux\] See <https://man7.org/linux/man-pages/man2/getrlimit.2.html>
#[allow(clippy::unnecessary_min_or_max)]
#[inline]
pub fn getrlimit(resource: Resource) -> io::Result<(u64, u64)> {
    check_supported(resource)?;
    let mut rlim = unsafe { mem::zeroed() };
    #[allow(clippy::cast_lossless)]
    let ret = unsafe { C::getrlimit(resource.as_raw() as _, &mut rlim) };

    #[allow(clippy::unnecessary_cast)]
    if ret == 0 {
        let soft = (rlim.rlim_cur as u64).min(INFINITY);
        let hard = (rlim.rlim_max as u64).min(INFINITY);
        Ok((soft, hard))
    } else {
        Err(io::Error::last_os_error())
    }
}

/// The type of a process ID
#[allow(non_camel_case_types)]
#[cfg_attr(docsrs, doc(cfg(any(target_os = "linux", target_os = "android"))))]
#[cfg(any(doc, target_os = "linux", target_os = "android"))]
pub type pid_t = i32;

/// Set and get the resource limits of an arbitrary process.
/// # Errors
/// See <https://man7.org/linux/man-pages/man2/prlimit.2.html>
#[allow(clippy::unnecessary_min_or_max)]
#[inline]
#[cfg_attr(docsrs, doc(cfg(any(target_os = "linux", target_os = "android"))))]
#[cfg(any(doc, rlimit__has_prlimit64))]
pub fn prlimit(
    pid: pid_t,
    resource: Resource,
    new_limit: Option<(u64, u64)>,
    old_limit: Option<(&mut u64, &mut u64)>,
) -> io::Result<()> {
    check_supported(resource)?;

    let new_rlim: Option<C::rlimit> = new_limit.map(|(soft, hard)| C::rlimit {
        rlim_cur: soft.min(INFINITY) as _,
        rlim_max: hard.min(INFINITY) as _,
    });

    let new_rlimit_ptr: *const C::rlimit = match new_rlim {
        Some(ref rlim) => rlim,
        None => std::ptr::null(),
    };

    let mut old_rlim: C::rlimit = unsafe { mem::zeroed() };

    let old_rlimit_ptr: *mut C::rlimit = if old_limit.is_some() {
        &mut old_rlim
    } else {
        std::ptr::null_mut()
    };

    #[allow(clippy::cast_lossless)]
    let ret = unsafe { C::prlimit(pid, resource.as_raw() as _, new_rlimit_ptr, old_rlimit_ptr) };

    if ret == 0 {
        #[allow(clippy::unnecessary_cast)]
        if let Some((soft, hard)) = old_limit {
            *soft = (old_rlim.rlim_cur as u64).min(INFINITY);
            *hard = (old_rlim.rlim_max as u64).min(INFINITY);
        }

        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
