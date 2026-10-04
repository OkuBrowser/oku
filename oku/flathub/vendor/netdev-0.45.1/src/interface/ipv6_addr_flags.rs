//! Per-address IPv6 state flags, normalized across platforms.

/// State flags for a single IPv6 address.
///
/// All fields default to `false` when the platform does not provide the
/// corresponding information.
///
/// Flags are collected from platform-specific sources:
///
/// - **Linux/Android**: netlink `IFA_FLAGS` attribute (`IFA_F_*` from [`<linux/if_addr.h>`])
/// - **macOS/iOS**: `SIOCGIFAFLAG_IN6` ioctl (`IN6_IFF_*` from [`<netinet6/in6_var.h>`][xnu])
/// - **FreeBSD/OpenBSD/NetBSD**: `SIOCGIFAFLAG_IN6` ioctl (`IN6_IFF_*` from [`<netinet6/in6_var.h>`][freebsd])
/// - **Windows**: [`NL_DAD_STATE`] and [`NL_SUFFIX_ORIGIN`] from `IP_ADAPTER_UNICAST_ADDRESS`
///
/// [`<linux/if_addr.h>`]: https://github.com/torvalds/linux/blob/master/include/uapi/linux/if_addr.h
/// [xnu]: https://github.com/apple-oss-distributions/xnu/blob/main/bsd/netinet6/in6_var.h
/// [freebsd]: https://github.com/freebsd/freebsd-src/blob/main/sys/netinet6/in6_var.h
/// [`NL_DAD_STATE`]: https://learn.microsoft.com/en-us/windows/win32/api/nldef/ne-nldef-nl_dad_state
/// [`NL_SUFFIX_ORIGIN`]: https://learn.microsoft.com/en-us/windows/win32/api/nldef/ne-nldef-nl_suffix_origin
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ipv6AddrFlags {
    /// Preferred lifetime expired; should not be used for new connections.
    ///
    /// Sourced from `IFA_F_DEPRECATED` (Linux), `IN6_IFF_DEPRECATED` (BSD),
    /// or `IpDadStateDeprecated` (Windows).
    pub deprecated: bool,
    /// Privacy address ([RFC 4941](https://datatracker.ietf.org/doc/html/rfc4941)).
    ///
    /// Sourced from `IFA_F_TEMPORARY` (Linux), `IN6_IFF_TEMPORARY` (BSD),
    /// or `IpSuffixOriginRandom` (Windows).
    pub temporary: bool,
    /// Undergoing duplicate address detection.
    ///
    /// Sourced from `IFA_F_TENTATIVE` (Linux), `IN6_IFF_TENTATIVE` (BSD),
    /// or `IpDadStateTentative` (Windows).
    pub tentative: bool,
    /// Duplicate address detection failed.
    ///
    /// Sourced from `IFA_F_DADFAILED` (Linux), `IN6_IFF_DUPLICATED` (BSD),
    /// or `IpDadStateDuplicate` (Windows).
    pub duplicated: bool,
    /// Manually configured, not from SLAAC.
    ///
    /// Sourced from `IFA_F_PERMANENT` (Linux). Not available on BSD or Windows.
    pub permanent: bool,
}

// Platform dispatch for `get_ipv6_addr_flags`, called from `unix_interfaces()`.

#[cfg(target_vendor = "apple")]
pub(crate) use crate::os::darwin::ipv6_addr_flags::*;

#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub(crate) use crate::os::bsd::ipv6_addr_flags::*;

// On Linux/Android flags come from netlink; this is only reached via the
// `unix_interfaces()` fallback when netlink is unavailable.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn get_ipv6_addr_flags(_ifname: &str, _addr: &std::net::Ipv6Addr) -> Ipv6AddrFlags {
    Ipv6AddrFlags::default()
}
