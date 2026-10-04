use libc::{c_int, pid_t};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(in crate::os::bsd) struct rt_metrics {
    pub(in crate::os::bsd) rmx_pksent: u64,
    pub(in crate::os::bsd) rmx_expire: i64,
    pub(in crate::os::bsd) rmx_locks: u32,
    pub(in crate::os::bsd) rmx_mtu: u32,
    pub(in crate::os::bsd) rmx_refcnt: u32,
    pub(in crate::os::bsd) rmx_hopcount: u32,
    pub(in crate::os::bsd) rmx_recvpipe: u32,
    pub(in crate::os::bsd) rmx_sendpipe: u32,
    pub(in crate::os::bsd) rmx_ssthresh: u32,
    pub(in crate::os::bsd) rmx_rtt: u32,
    pub(in crate::os::bsd) rmx_rttvar: u32,
    pub(in crate::os::bsd) rmx_pad: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(in crate::os::bsd) struct rt_msghdr {
    pub(in crate::os::bsd) rtm_msglen: u16,
    pub(in crate::os::bsd) rtm_version: u8,
    pub(in crate::os::bsd) rtm_type: u8,
    pub(in crate::os::bsd) rtm_hdrlen: u16,
    pub(in crate::os::bsd) rtm_index: u16,
    pub(in crate::os::bsd) rtm_tableid: u16,
    pub(in crate::os::bsd) rtm_priority: u8,
    pub(in crate::os::bsd) rtm_mpls: u8,
    pub(in crate::os::bsd) rtm_addrs: c_int,
    pub(in crate::os::bsd) rtm_flags: c_int,
    pub(in crate::os::bsd) rtm_fmask: c_int,
    pub(in crate::os::bsd) rtm_pid: pid_t,
    pub(in crate::os::bsd) rtm_seq: c_int,
    pub(in crate::os::bsd) rtm_errno: c_int,
    pub(in crate::os::bsd) rtm_inits: u32,
    pub(in crate::os::bsd) rtm_rmx: rt_metrics,
}

pub(in crate::os::bsd) const SOCKADDR_ALIGN: usize = core::mem::size_of::<libc::c_long>();

#[inline]
pub(in crate::os::bsd) fn message_header_len(hdr: &rt_msghdr) -> usize {
    hdr.rtm_hdrlen as usize
}

#[cfg(target_pointer_width = "64")]
const _: [(); 56] = [(); core::mem::size_of::<rt_metrics>()];

#[cfg(target_pointer_width = "64")]
const _: [(); 96] = [(); core::mem::size_of::<rt_msghdr>()];
