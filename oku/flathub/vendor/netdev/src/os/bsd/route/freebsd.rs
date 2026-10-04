use libc::{c_int, c_ulong, c_ushort, pid_t};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(in crate::os::bsd) struct rt_metrics {
    pub(in crate::os::bsd) rmx_locks: c_ulong,
    pub(in crate::os::bsd) rmx_mtu: c_ulong,
    pub(in crate::os::bsd) rmx_hopcount: c_ulong,
    pub(in crate::os::bsd) rmx_expire: c_ulong,
    pub(in crate::os::bsd) rmx_recvpipe: c_ulong,
    pub(in crate::os::bsd) rmx_sendpipe: c_ulong,
    pub(in crate::os::bsd) rmx_ssthresh: c_ulong,
    pub(in crate::os::bsd) rmx_rtt: c_ulong,
    pub(in crate::os::bsd) rmx_rttvar: c_ulong,
    pub(in crate::os::bsd) rmx_pksent: c_ulong,
    pub(in crate::os::bsd) rmx_weight: c_ulong,
    pub(in crate::os::bsd) rmx_nhidx: c_ulong,
    pub(in crate::os::bsd) rmx_filler: [c_ulong; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(in crate::os::bsd) struct rt_msghdr {
    pub(in crate::os::bsd) rtm_msglen: c_ushort,
    pub(in crate::os::bsd) rtm_version: u8,
    pub(in crate::os::bsd) rtm_type: u8,
    pub(in crate::os::bsd) rtm_index: c_ushort,
    pub(in crate::os::bsd) _rtm_spare1: c_ushort,
    pub(in crate::os::bsd) rtm_flags: c_int,
    pub(in crate::os::bsd) rtm_addrs: c_int,
    pub(in crate::os::bsd) rtm_pid: pid_t,
    pub(in crate::os::bsd) rtm_seq: c_int,
    pub(in crate::os::bsd) rtm_errno: c_int,
    pub(in crate::os::bsd) rtm_fmask: c_int,
    pub(in crate::os::bsd) rtm_inits: c_ulong,
    pub(in crate::os::bsd) rtm_rmx: rt_metrics,
}

pub(in crate::os::bsd) const SOCKADDR_ALIGN: usize = core::mem::size_of::<libc::c_long>();

#[inline]
pub(in crate::os::bsd) fn message_header_len(_: &rt_msghdr) -> usize {
    core::mem::size_of::<rt_msghdr>()
}

#[cfg(all(
    target_pointer_width = "64",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const _: [(); 112] = [(); core::mem::size_of::<rt_metrics>()];

#[cfg(all(
    target_pointer_width = "64",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const _: [(); 152] = [(); core::mem::size_of::<rt_msghdr>()];
