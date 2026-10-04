use libc::{c_int, pid_t};

#[repr(C, align(8))]
#[derive(Debug, Copy, Clone)]
pub(in crate::os::bsd) struct rt_metrics {
    pub(in crate::os::bsd) rmx_locks: u64,
    pub(in crate::os::bsd) rmx_mtu: u64,
    pub(in crate::os::bsd) rmx_hopcount: u64,
    pub(in crate::os::bsd) rmx_recvpipe: u64,
    pub(in crate::os::bsd) rmx_sendpipe: u64,
    pub(in crate::os::bsd) rmx_ssthresh: u64,
    pub(in crate::os::bsd) rmx_rtt: u64,
    pub(in crate::os::bsd) rmx_rttvar: u64,
    pub(in crate::os::bsd) rmx_expire: i64,
    pub(in crate::os::bsd) rmx_pksent: i64,
}

#[repr(C, align(8))]
#[derive(Debug, Copy, Clone)]
pub(in crate::os::bsd) struct rt_msghdr {
    pub(in crate::os::bsd) rtm_msglen: u16,
    pub(in crate::os::bsd) rtm_version: u8,
    pub(in crate::os::bsd) rtm_type: u8,
    pub(in crate::os::bsd) rtm_index: u16,
    pub(in crate::os::bsd) rtm_flags: c_int,
    pub(in crate::os::bsd) rtm_addrs: c_int,
    pub(in crate::os::bsd) rtm_pid: pid_t,
    pub(in crate::os::bsd) rtm_seq: c_int,
    pub(in crate::os::bsd) rtm_errno: c_int,
    pub(in crate::os::bsd) rtm_use: c_int,
    pub(in crate::os::bsd) rtm_inits: c_int,
    pub(in crate::os::bsd) rtm_rmx: rt_metrics,
}

pub(in crate::os::bsd) const SOCKADDR_ALIGN: usize = core::mem::size_of::<u64>();

#[inline]
pub(in crate::os::bsd) fn message_header_len(_: &rt_msghdr) -> usize {
    core::mem::size_of::<rt_msghdr>()
}

const _: [(); 80] = [(); core::mem::size_of::<rt_metrics>()];
const _: [(); 120] = [(); core::mem::size_of::<rt_msghdr>()];
