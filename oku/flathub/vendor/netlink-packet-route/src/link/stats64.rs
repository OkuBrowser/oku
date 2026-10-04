// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
    Unaligned,
)]
#[repr(C, packed)]
pub struct Stats64Buffer {
    rx_packets: u64,
    tx_packets: u64,
    rx_bytes: u64,
    tx_bytes: u64,
    rx_errors: u64,
    tx_errors: u64,
    rx_dropped: u64,
    tx_dropped: u64,
    multicast: u64,
    collisions: u64,
    rx_length_errors: u64,
    rx_over_errors: u64,
    rx_crc_errors: u64,
    rx_frame_errors: u64,
    rx_fifo_errors: u64,
    rx_missed_errors: u64,
    tx_aborted_errors: u64,
    tx_carrier_errors: u64,
    tx_fifo_errors: u64,
    tx_heartbeat_errors: u64,
    tx_window_errors: u64,
    rx_compressed: u64,
    tx_compressed: u64,
    rx_nohandler: u64,
    rx_otherhost_dropped: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct Stats64 {
    /// total packets received
    pub rx_packets: u64,
    /// total packets transmitted
    pub tx_packets: u64,
    /// total bytes received
    pub rx_bytes: u64,
    /// total bytes transmitted
    pub tx_bytes: u64,
    /// bad packets received
    pub rx_errors: u64,
    /// packet transmit problems
    pub tx_errors: u64,
    /// no space in linux buffers
    pub rx_dropped: u64,
    /// no space available in linux
    pub tx_dropped: u64,
    /// multicast packets received
    pub multicast: u64,
    pub collisions: u64,

    // detailed rx_errors
    pub rx_length_errors: u64,
    /// receiver ring buff overflow
    pub rx_over_errors: u64,
    /// received packets with crc error
    pub rx_crc_errors: u64,
    /// received frame alignment errors
    pub rx_frame_errors: u64,
    /// recv'r fifo overrun
    pub rx_fifo_errors: u64,
    /// receiver missed packet
    pub rx_missed_errors: u64,

    // detailed tx_errors
    pub tx_aborted_errors: u64,
    pub tx_carrier_errors: u64,
    pub tx_fifo_errors: u64,
    pub tx_heartbeat_errors: u64,
    pub tx_window_errors: u64,

    // for cslip etc
    pub rx_compressed: u64,
    pub tx_compressed: u64,

    /// dropped, no handler found
    pub rx_nohandler: u64,

    pub rx_otherhost_dropped: u64,
}

impl Stats64 {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            Stats64Buffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<Stats64Buffer>(),
                )
            })?;
        Ok(Self {
            rx_packets: raw.rx_packets,
            tx_packets: raw.tx_packets,
            rx_bytes: raw.rx_bytes,
            tx_bytes: raw.tx_bytes,
            rx_errors: raw.rx_errors,
            tx_errors: raw.tx_errors,
            rx_dropped: raw.rx_dropped,
            tx_dropped: raw.tx_dropped,
            multicast: raw.multicast,
            collisions: raw.collisions,
            rx_length_errors: raw.rx_length_errors,
            rx_over_errors: raw.rx_over_errors,
            rx_crc_errors: raw.rx_crc_errors,
            rx_frame_errors: raw.rx_frame_errors,
            rx_fifo_errors: raw.rx_fifo_errors,
            rx_missed_errors: raw.rx_missed_errors,
            tx_aborted_errors: raw.tx_aborted_errors,
            tx_carrier_errors: raw.tx_carrier_errors,
            tx_fifo_errors: raw.tx_fifo_errors,
            tx_heartbeat_errors: raw.tx_heartbeat_errors,
            tx_window_errors: raw.tx_window_errors,
            rx_compressed: raw.rx_compressed,
            tx_compressed: raw.tx_compressed,
            rx_nohandler: raw.rx_nohandler,
            rx_otherhost_dropped: raw.rx_otherhost_dropped,
        })
    }
}

impl From<&Stats64> for Stats64Buffer {
    fn from(value: &Stats64) -> Self {
        Self {
            rx_packets: value.rx_packets,
            tx_packets: value.tx_packets,
            rx_bytes: value.rx_bytes,
            tx_bytes: value.tx_bytes,
            rx_errors: value.rx_errors,
            tx_errors: value.tx_errors,
            rx_dropped: value.rx_dropped,
            tx_dropped: value.tx_dropped,
            multicast: value.multicast,
            collisions: value.collisions,
            rx_length_errors: value.rx_length_errors,
            rx_over_errors: value.rx_over_errors,
            rx_crc_errors: value.rx_crc_errors,
            rx_frame_errors: value.rx_frame_errors,
            rx_fifo_errors: value.rx_fifo_errors,
            rx_missed_errors: value.rx_missed_errors,
            tx_aborted_errors: value.tx_aborted_errors,
            tx_carrier_errors: value.tx_carrier_errors,
            tx_fifo_errors: value.tx_fifo_errors,
            tx_heartbeat_errors: value.tx_heartbeat_errors,
            tx_window_errors: value.tx_window_errors,
            rx_compressed: value.rx_compressed,
            tx_compressed: value.tx_compressed,
            rx_nohandler: value.rx_nohandler,
            rx_otherhost_dropped: value.rx_otherhost_dropped,
        }
    }
}

impl Emitable for Stats64 {
    fn buffer_len(&self) -> usize {
        size_of::<Stats64Buffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = Stats64Buffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
