// SPDX-License-Identifier: MIT

const BUFF_LEN: usize = 8;

use netlink_packet_core::{
    emit_u32, parse_u32, DecodeError, Emitable, Parseable,
};

// Kernel struct `br_boolopt_multi`
/// Change multiple bridge boolean options
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct BridgeBooleanOptions {
    pub value: BridgeBooleanOptionFlags,
    pub mask: BridgeBooleanOptionFlags,
}

impl<T: AsRef<[u8]> + ?Sized> Parseable<T> for BridgeBooleanOptions {
    fn parse(buf: &T) -> Result<Self, DecodeError> {
        let buf = buf.as_ref();
        if buf.len() < BUFF_LEN {
            return Err(DecodeError::from(format!(
                "Invalid length for IFLA_BR_MULTI_BOOLOPT data, expecting {}, \
                 but got {}",
                BUFF_LEN,
                buf.len()
            )));
        }

        Ok(Self {
            value: BridgeBooleanOptionFlags::from_bits_retain(parse_u32(
                &buf[..4],
            )?),
            mask: BridgeBooleanOptionFlags::from_bits_retain(parse_u32(
                &buf[4..],
            )?),
        })
    }
}

impl Emitable for BridgeBooleanOptions {
    fn buffer_len(&self) -> usize {
        BUFF_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        emit_u32(&mut buffer[..4], self.value.bits()).unwrap();
        emit_u32(&mut buffer[4..], self.mask.bits()).unwrap();
    }
}

const BR_BOOLOPT_NO_LL_LEARN: u32 = 1 << 0;
const BR_BOOLOPT_MCAST_VLAN_SNOOPING: u32 = 1 << 1;
const BR_BOOLOPT_MST_ENABLE: u32 = 1 << 2;
const BR_BOOLOPT_MDB_OFFLOAD_FAIL_NOTIFICATION: u32 = 1 << 3;
const BR_BOOLOPT_FDB_LOCAL_VLAN_0: u32 = 1 << 4;

bitflags! {
    #[derive(Clone, Eq, PartialEq, Debug, Copy, Default)]
    #[non_exhaustive]
    pub struct BridgeBooleanOptionFlags: u32 {
        const NoLinkLocalLearn= BR_BOOLOPT_NO_LL_LEARN;
        const VlanMulticastSnooping = BR_BOOLOPT_MCAST_VLAN_SNOOPING;
        const MstEnable = BR_BOOLOPT_MST_ENABLE;
        const MdbOffloadFailNotif = BR_BOOLOPT_MDB_OFFLOAD_FAIL_NOTIFICATION;
        const FdbLocalVlan0 = BR_BOOLOPT_FDB_LOCAL_VLAN_0;
        const _ = !0;
    }
}
