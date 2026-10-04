use crate::stats::counters::InterfaceStats;

pub(crate) fn get_stats_from_index(index: u32) -> Option<InterfaceStats> {
    use std::mem::zeroed;
    use std::time::SystemTime;
    use windows_sys::Win32::NetworkManagement::IpHelper::{GetIfEntry2, MIB_IF_ROW2};

    let mut row: MIB_IF_ROW2 = unsafe { zeroed() };
    row.InterfaceIndex = index;

    unsafe {
        if GetIfEntry2(&mut row) == 0 {
            Some(InterfaceStats {
                rx_bytes: row.InOctets as u64,
                tx_bytes: row.OutOctets as u64,
                timestamp: Some(SystemTime::now()),
            })
        } else {
            None
        }
    }
}
