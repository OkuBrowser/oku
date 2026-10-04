use crate::interface::ipv6_addr_flags::Ipv6AddrFlags;

/// Decode a raw `IFA_F_*` bitmask into [`Ipv6AddrFlags`].
pub(crate) fn from_netlink_flags(raw: u32) -> Ipv6AddrFlags {
    // <linux/if_addr.h>
    const IFA_F_TEMPORARY: u32 = 0x01;
    const IFA_F_DADFAILED: u32 = 0x08;
    const IFA_F_DEPRECATED: u32 = 0x20;
    const IFA_F_TENTATIVE: u32 = 0x40;
    const IFA_F_PERMANENT: u32 = 0x80;

    Ipv6AddrFlags {
        deprecated: raw & IFA_F_DEPRECATED != 0,
        temporary: raw & IFA_F_TEMPORARY != 0,
        tentative: raw & IFA_F_TENTATIVE != 0,
        duplicated: raw & IFA_F_DADFAILED != 0,
        permanent: raw & IFA_F_PERMANENT != 0,
    }
}
