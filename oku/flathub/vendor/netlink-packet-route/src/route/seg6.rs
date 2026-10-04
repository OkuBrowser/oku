// SPDX-License-Identifier: MIT

use std::net::{IpAddr, Ipv6Addr};

use netlink_packet_core::{
    DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer, Parseable,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::ip::{emit_ip_addr, parse_ipv6_addr};

const SEG6_IPTUN_MODE_INLINE: u32 = 0;
const SEG6_IPTUN_MODE_ENCAP: u32 = 1;
//const SEG6_IPTUN_MODE_L2ENCAP: u32 = 2;
//const SEG6_IPTUN_MODE_ENCAP_RED: u32 = 3;
//const SEG6_IPTUN_MODE_L2ENCAP_RED: u32 = 4;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[non_exhaustive]
pub enum Seg6Mode {
    // Inline mode for Seg6
    #[default]
    Inline,
    // Encapsulation mode for Seg6
    Encap,
    // L2ENCAP = 2,
    // ENCAP_RED = 3,
    // L2ENCAP_RED = 4
    Other(u32),
}

impl From<Seg6Mode> for u32 {
    fn from(value: Seg6Mode) -> Self {
        match value {
            Seg6Mode::Inline => SEG6_IPTUN_MODE_INLINE,
            Seg6Mode::Encap => SEG6_IPTUN_MODE_ENCAP,
            Seg6Mode::Other(i) => i,
        }
    }
}

impl From<u32> for Seg6Mode {
    fn from(value: u32) -> Self {
        match value {
            SEG6_IPTUN_MODE_INLINE => Seg6Mode::Inline,
            SEG6_IPTUN_MODE_ENCAP => Seg6Mode::Encap,
            v => Seg6Mode::Other(v),
        }
    }
}

const SEG6_IPTUNNEL_SRH: u16 = 1;

/// Netlink attributes for `RTA_ENCAP` with `RTA_ENCAP_TYPE` set to
/// `LWTUNNEL_ENCAP_SEG6`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum RouteSeg6IpTunnel {
    // Use an IPv6 segment routing header
    Seg6(Seg6Header),
    Other(DefaultNla),
}

impl Nla for RouteSeg6IpTunnel {
    fn value_len(&self) -> usize {
        match self {
            RouteSeg6IpTunnel::Seg6(v) => v.value_len(),
            RouteSeg6IpTunnel::Other(v) => v.value_len(),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            RouteSeg6IpTunnel::Seg6(v) => v.kind(),
            RouteSeg6IpTunnel::Other(v) => v.kind(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            RouteSeg6IpTunnel::Seg6(v) => v.emit_value(buffer),
            RouteSeg6IpTunnel::Other(v) => v.emit_value(buffer),
        }
    }
}

const SEG6_HEADER_LEN: usize = 12;

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
pub struct Seg6MessageBuffer {
    mode: u32,
    nexthdr: u8,
    hdrlen: u8,
    seg_type: u8,
    segments_left: u8,
    first_segment: u8,
    flags: u8,
    tag: u16,
}

const SEG6_SEGMENT_LEN: usize = 16;

/// Netlink attributes for `RTA_ENCAP` with `RTA_ENCAP_TYPE` set to
/// `LWTUNNEL_ENCAP_SEG6`.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct Seg6Header {
    // Operation mode
    pub mode: Seg6Mode,
    // List of segments
    pub segments: Vec<Ipv6Addr>,
}

impl Seg6Header {
    fn push_segments(buf: &mut [u8], mut segments: Vec<Ipv6Addr>) {
        if let Some(segment) = segments.pop() {
            emit_ip_addr(&IpAddr::V6(segment), &mut buf[..SEG6_SEGMENT_LEN]);
            Self::push_segments(&mut buf[SEG6_SEGMENT_LEN..], segments);
        }
    }

    fn get_segments(
        buf: &[u8],
        segments: &mut Vec<Ipv6Addr>,
    ) -> Result<(), DecodeError> {
        // are there any remaining segments ?
        if buf.len() >= SEG6_SEGMENT_LEN {
            let segment = parse_ipv6_addr(&buf[..SEG6_SEGMENT_LEN])?;
            segments.push(segment);
            Self::get_segments(&buf[SEG6_SEGMENT_LEN..], segments)?;
        }
        Ok(())
    }
}

impl Nla for Seg6Header {
    fn value_len(&self) -> usize {
        let segments = match self.mode {
            // in inline mode, seg6 add an additional segment (::) at the
            // end of the segment list, thus must have one additional
            // segment slot in the payload
            Seg6Mode::Inline => self.segments.len() + 1,
            Seg6Mode::Encap => self.segments.len(),
            Seg6Mode::Other(_) => self.segments.len(),
        };
        12 + 16 * segments
    }

    fn kind(&self) -> u16 {
        SEG6_IPTUNNEL_SRH
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        // Some sources for understanding the format of Seg6 in Netlink
        //
        // torvalds/linux:
        //      include/uapi/linux/seg6.h
        //      include/uapi/linux/seg6_iptunnel.h
        //
        // iproute2/iproute2
        //      ip/iproute_lwtunnel.c parse_encap_seg6()

        let mut number_segments = self.segments.len();
        if matches!(self.mode, Seg6Mode::Inline) {
            number_segments += 1 // last segment (::) added
        }

        let srhlen = 8 + 16 * number_segments;

        let raw = Seg6MessageBuffer {
            mode: self.mode.into(),
            nexthdr: 0,
            hdrlen: ((srhlen >> 3) - 1) as u8,
            seg_type: 4,
            segments_left: (number_segments - 1) as u8,
            first_segment: (number_segments - 1) as u8,
            flags: 0,
            tag: 0,
        };
        buffer[..SEG6_HEADER_LEN].copy_from_slice(raw.as_bytes());

        let mut segments = self.segments.clone();

        // Add the last segment (::) if working in inline mode
        if matches!(self.mode, Seg6Mode::Inline) {
            segments.push("::".parse().expect("Impossible error"))
        }

        Seg6Header::push_segments(&mut buffer[SEG6_HEADER_LEN..], segments);
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for RouteSeg6IpTunnel
{
    fn parse(
        buf: &NlaBuffer<&'a T>,
    ) -> Result<Self, netlink_packet_core::DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            SEG6_IPTUNNEL_SRH => {
                let (raw, segments_buf) = Seg6MessageBuffer::ref_from_prefix(
                    payload,
                )
                .map_err(|_| {
                    DecodeError::buffer_too_small(
                        payload.len(),
                        SEG6_HEADER_LEN,
                    )
                })?;

                let mut segments: Vec<Ipv6Addr> = vec![];
                Seg6Header::get_segments(segments_buf, &mut segments)?;

                let mut segments: Vec<Ipv6Addr> =
                    segments.into_iter().rev().collect();

                let mode = Seg6Mode::from(raw.mode);
                if matches!(mode, Seg6Mode::Inline) {
                    segments.pop(); // remove last inline segment
                }

                RouteSeg6IpTunnel::Seg6(Seg6Header { mode, segments })
            }
            _ => Self::Other(
                DefaultNla::parse(buf)
                    .context("invalid NLA value (unknown type) value")?,
            ),
        })
    }
}
