// SPDX-License-Identifier: MIT

use bitflags::bitflags;
use netlink_packet_core::{
    emit_u16, emit_u32, parse_u16, parse_u32, DecodeError, DefaultNla,
    ErrorContext, Nla, NlaBuffer, Parseable,
};

const IFLA_CAN_BITTIMING: u16 = 1;
const IFLA_CAN_BITTIMING_CONST: u16 = 2;
const IFLA_CAN_CLOCK: u16 = 3;
const IFLA_CAN_STATE: u16 = 4;
const IFLA_CAN_CTRLMODE: u16 = 5;
const IFLA_CAN_RESTART_MS: u16 = 6;
const IFLA_CAN_RESTART: u16 = 7;
const IFLA_CAN_BERR_COUNTER: u16 = 8;
const IFLA_CAN_DATA_BITTIMING: u16 = 9;
const IFLA_CAN_DATA_BITTIMING_CONST: u16 = 10;
const IFLA_CAN_TERMINATION: u16 = 12;
const IFLA_CAN_BITTIMING_MAX: u16 = 14;
const IFLA_CAN_CC_LEN: u16 = 15;
const IFLA_CAN_DATA_BITTIMING_MAX: u16 = 16;
const IFLA_CAN_TDC: u16 = 17;
const IFLA_CAN_CTRLMODE_EXT: u16 = 18;

/// CAN bit timing parameters, corresponding to `struct can_bittiming` in the
/// kernel.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub struct CanBitTiming {
    pub bitrate: u32,
    pub sample_point: u32,
    pub tq: u32,
    pub prop_seg: u32,
    pub phase_seg1: u32,
    pub phase_seg2: u32,
    pub sjw: u32,
    pub brp: u32,
}

const CAN_BITTIMING_LEN: usize = 32;

impl CanBitTiming {
    /// Create a new `CanBitTiming` with the given bitrate.
    pub fn new(bitrate: u32) -> Self {
        Self {
            bitrate,
            ..Default::default()
        }
    }

    fn value_len() -> usize {
        CAN_BITTIMING_LEN
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        let mut offset = 0;
        emit_u32(&mut buffer[offset..], self.bitrate).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.sample_point).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tq).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.prop_seg).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.phase_seg1).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.phase_seg2).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.sjw).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.brp).unwrap();
    }

    fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        if payload.len() < Self::value_len() {
            return Err("invalid IFLA_CAN_BITTIMING: expected 32 bytes".into());
        }
        Ok(Self {
            bitrate: parse_u32(&payload[0..4])
                .context("invalid IFLA_CAN_BITTIMING bitrate")?,
            sample_point: parse_u32(&payload[4..8])
                .context("invalid IFLA_CAN_BITTIMING sample_point")?,
            tq: parse_u32(&payload[8..12])
                .context("invalid IFLA_CAN_BITTIMING tq")?,
            prop_seg: parse_u32(&payload[12..16])
                .context("invalid IFLA_CAN_BITTIMING prop_seg")?,
            phase_seg1: parse_u32(&payload[16..20])
                .context("invalid IFLA_CAN_BITTIMING phase_seg1")?,
            phase_seg2: parse_u32(&payload[20..24])
                .context("invalid IFLA_CAN_BITTIMING phase_seg2")?,
            sjw: parse_u32(&payload[24..28])
                .context("invalid IFLA_CAN_BITTIMING sjw")?,
            brp: parse_u32(&payload[28..32])
                .context("invalid IFLA_CAN_BITTIMING brp")?,
        })
    }
}

const CAN_CTRLMODE_LOOPBACK: u32 = 0x001;
const CAN_CTRLMODE_LISTENONLY: u32 = 0x002;
const CAN_CTRLMODE_3_SAMPLES: u32 = 0x004;
const CAN_CTRLMODE_ONE_SHOT: u32 = 0x008;
const CAN_CTRLMODE_BERR_REPORTING: u32 = 0x010;
const CAN_CTRLMODE_FD: u32 = 0x020;
const CAN_CTRLMODE_PRESUME_ACK: u32 = 0x040;
const CAN_CTRLMODE_FD_NON_ISO: u32 = 0x080;
const CAN_CTRLMODE_CC_LEN8_DLC: u32 = 0x100;
const CAN_CTRLMODE_TDC_AUTO: u32 = 0x200;
const CAN_CTRLMODE_TDC_MANUAL: u32 = 0x400;
const CAN_CTRLMODE_RESTRICTED: u32 = 0x800;
const CAN_CTRLMODE_XL: u32 = 0x1000;
const CAN_CTRLMODE_XL_TDC_AUTO: u32 = 0x2000;
const CAN_CTRLMODE_XL_TDC_MANUAL: u32 = 0x4000;
const CAN_CTRLMODE_XL_TMS: u32 = 0x8000;

bitflags! {
    #[non_exhaustive]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CanCtrlModeFlags: u32 {
        const Loopback = CAN_CTRLMODE_LOOPBACK;
        const ListenOnly = CAN_CTRLMODE_LISTENONLY;
        const TripleSampling = CAN_CTRLMODE_3_SAMPLES;
        const OneShot = CAN_CTRLMODE_ONE_SHOT;
        const BerrReporting = CAN_CTRLMODE_BERR_REPORTING;
        const Fd = CAN_CTRLMODE_FD;
        const PresumeAck = CAN_CTRLMODE_PRESUME_ACK;
        const FdNonIso = CAN_CTRLMODE_FD_NON_ISO;
        const CcLen8Dlc = CAN_CTRLMODE_CC_LEN8_DLC;
        const TdcAuto = CAN_CTRLMODE_TDC_AUTO;
        const TdcManual = CAN_CTRLMODE_TDC_MANUAL;
        const Restricted = CAN_CTRLMODE_RESTRICTED;
        const Xl = CAN_CTRLMODE_XL;
        const XlTdcAuto = CAN_CTRLMODE_XL_TDC_AUTO;
        const XlTdcManual = CAN_CTRLMODE_XL_TDC_MANUAL;
        const XlTms = CAN_CTRLMODE_XL_TMS;
        const _ = !0;
    }
}

/// CAN controller mode, corresponding to `struct can_ctrlmode` in the kernel.
///
/// Contains `mask` and `flags` fields, each a bitmask of
/// [`CanCtrlModeFlags`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CanCtrlMode {
    pub mask: CanCtrlModeFlags,
    pub flags: CanCtrlModeFlags,
}

const CAN_CTRLMODE_LEN: usize = 8;

impl CanCtrlMode {
    fn value_len() -> usize {
        CAN_CTRLMODE_LEN
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        emit_u32(&mut buffer[0..4], self.mask.bits()).unwrap();
        emit_u32(&mut buffer[4..8], self.flags.bits()).unwrap();
    }

    fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        if payload.len() < Self::value_len() {
            return Err("invalid IFLA_CAN_CTRLMODE: expected 8 bytes".into());
        }
        Ok(Self {
            mask: CanCtrlModeFlags::from_bits_retain(
                parse_u32(&payload[0..4])
                    .context("invalid IFLA_CAN_CTRLMODE mask")?,
            ),
            flags: CanCtrlModeFlags::from_bits_retain(
                parse_u32(&payload[4..8])
                    .context("invalid IFLA_CAN_CTRLMODE flags")?,
            ),
        })
    }
}

/// CAN clock parameters, corresponding to `struct can_clock` in the kernel.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct CanClock {
    pub freq: u32,
}

const CAN_CLOCK_LEN: usize = 4;

impl CanClock {
    pub fn new(freq: u32) -> Self {
        Self { freq }
    }

    fn value_len() -> usize {
        CAN_CLOCK_LEN
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        emit_u32(buffer, self.freq).unwrap();
    }

    fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        if payload.len() < Self::value_len() {
            return Err("invalid IFLA_CAN_CLOCK: expected 4 bytes".into());
        }
        Ok(Self {
            freq: parse_u32(payload).context("invalid IFLA_CAN_CLOCK value")?,
        })
    }
}

/// CAN bus error counter, corresponding to `struct can_berr_counter` in the
/// kernel.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CanBerrCounter {
    pub txerr: u16,
    pub rxerr: u16,
}

const CAN_BERR_COUNTER_LEN: usize = 4;

impl CanBerrCounter {
    fn value_len() -> usize {
        CAN_BERR_COUNTER_LEN
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        emit_u16(&mut buffer[0..2], self.txerr).unwrap();
        emit_u16(&mut buffer[2..4], self.rxerr).unwrap();
    }

    fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        if payload.len() < Self::value_len() {
            return Err(
                "invalid IFLA_CAN_BERR_COUNTER: expected 4 bytes".into()
            );
        }
        Ok(Self {
            txerr: parse_u16(&payload[0..2])
                .context("invalid IFLA_CAN_BERR_COUNTER txerr")?,
            rxerr: parse_u16(&payload[2..4])
                .context("invalid IFLA_CAN_BERR_COUNTER rxerr")?,
        })
    }
}

/// CAN Transmitter Delay Compensation (TDC) parameters, corresponding to
/// `struct can_tdc` in the kernel.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CanTdc {
    pub tdcv_min: u32,
    pub tdcv_max: u32,
    pub tdcv: u32,
    pub tdco_min: u32,
    pub tdco_max: u32,
    pub tdco: u32,
    pub tdcf: u32,
}

const CAN_TDC_LEN: usize = 28;

impl CanTdc {
    fn value_len() -> usize {
        CAN_TDC_LEN
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        let mut offset = 0;
        emit_u32(&mut buffer[offset..], self.tdcv_min).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tdcv_max).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tdcv).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tdco_min).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tdco_max).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tdco).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tdcf).unwrap();
    }

    fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        if payload.len() < Self::value_len() {
            return Err("invalid IFLA_CAN_TDC: expected 28 bytes".into());
        }
        Ok(Self {
            tdcv_min: parse_u32(&payload[0..4])
                .context("invalid IFLA_CAN_TDC tdcv_min")?,
            tdcv_max: parse_u32(&payload[4..8])
                .context("invalid IFLA_CAN_TDC tdcv_max")?,
            tdcv: parse_u32(&payload[8..12])
                .context("invalid IFLA_CAN_TDC tdcv")?,
            tdco_min: parse_u32(&payload[12..16])
                .context("invalid IFLA_CAN_TDC tdco_min")?,
            tdco_max: parse_u32(&payload[16..20])
                .context("invalid IFLA_CAN_TDC tdco_max")?,
            tdco: parse_u32(&payload[20..24])
                .context("invalid IFLA_CAN_TDC tdco")?,
            tdcf: parse_u32(&payload[24..28])
                .context("invalid IFLA_CAN_TDC tdcf")?,
        })
    }
}

/// CAN bittiming const (read-only), corresponding to
/// `struct can_bittiming_const` in the kernel.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub struct CanBitTimingConst {
    pub name: String,
    pub tseg1_min: u32,
    pub tseg1_max: u32,
    pub tseg2_min: u32,
    pub tseg2_max: u32,
    pub sjw_max: u32,
    pub brp_min: u32,
    pub brp_max: u32,
    pub brp_inc: u32,
}

const CAN_BITTIMING_CONST_LEN: usize = 48;

impl CanBitTimingConst {
    /// Create a new `CanBitTimingConst` with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    fn value_len() -> usize {
        CAN_BITTIMING_CONST_LEN
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        let name_bytes = self.name.as_bytes();
        let name_len = name_bytes.len().min(15);
        buffer[..name_len].copy_from_slice(&name_bytes[..name_len]);
        buffer[name_len] = 0;
        let mut offset = 16usize;
        emit_u32(&mut buffer[offset..], self.tseg1_min).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tseg1_max).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tseg2_min).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.tseg2_max).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.sjw_max).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.brp_min).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.brp_max).unwrap();
        offset += 4;
        emit_u32(&mut buffer[offset..], self.brp_inc).unwrap();
    }

    fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        if payload.len() < Self::value_len() {
            return Err(
                "invalid IFLA_CAN_BITTIMING_CONST: expected 48 bytes".into()
            );
        }
        let name_end = payload[..16].iter().position(|&b| b == 0).unwrap_or(16);
        let name = String::from_utf8_lossy(&payload[..name_end]).to_string();
        Ok(Self {
            name,
            tseg1_min: parse_u32(&payload[16..20])
                .context("invalid IFLA_CAN_BITTIMING_CONST tseg1_min")?,
            tseg1_max: parse_u32(&payload[20..24])
                .context("invalid IFLA_CAN_BITTIMING_CONST tseg1_max")?,
            tseg2_min: parse_u32(&payload[24..28])
                .context("invalid IFLA_CAN_BITTIMING_CONST tseg2_min")?,
            tseg2_max: parse_u32(&payload[28..32])
                .context("invalid IFLA_CAN_BITTIMING_CONST tseg2_max")?,
            sjw_max: parse_u32(&payload[32..36])
                .context("invalid IFLA_CAN_BITTIMING_CONST sjw_max")?,
            brp_min: parse_u32(&payload[36..40])
                .context("invalid IFLA_CAN_BITTIMING_CONST brp_min")?,
            brp_max: parse_u32(&payload[40..44])
                .context("invalid IFLA_CAN_BITTIMING_CONST brp_max")?,
            brp_inc: parse_u32(&payload[44..48])
                .context("invalid IFLA_CAN_BITTIMING_CONST brp_inc")?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoCan {
    BitTiming(CanBitTiming),
    BitTimingConst(CanBitTimingConst),
    Clock(CanClock),
    State(u32),
    CtrlMode(CanCtrlMode),
    RestartMs(u32),
    Restart(u32),
    BerrCounter(CanBerrCounter),
    DataBitTiming(CanBitTiming),
    DataBitTimingConst(CanBitTimingConst),
    Termination(u16),
    BitTimingMax(CanBitTiming),
    CcLen(u32),
    DataBitTimingMax(CanBitTiming),
    Tdc(CanTdc),
    CtrlModeExt(CanCtrlMode),
    Other(DefaultNla),
}

impl Nla for InfoCan {
    fn value_len(&self) -> usize {
        match self {
            Self::BitTiming(_) => CanBitTiming::value_len(),
            Self::BitTimingConst(_) => CanBitTimingConst::value_len(),
            Self::Clock(_) => CanClock::value_len(),
            Self::State(_) => 4,
            Self::CtrlMode(_) => CanCtrlMode::value_len(),
            Self::RestartMs(_) => 4,
            Self::Restart(_) => 4,
            Self::BerrCounter(_) => CanBerrCounter::value_len(),
            Self::DataBitTiming(_) => CanBitTiming::value_len(),
            Self::DataBitTimingConst(_) => CanBitTimingConst::value_len(),
            Self::Termination(_) => 2,
            Self::BitTimingMax(_) => CanBitTiming::value_len(),
            Self::CcLen(_) => 4,
            Self::DataBitTimingMax(_) => CanBitTiming::value_len(),
            Self::Tdc(_) => CanTdc::value_len(),
            Self::CtrlModeExt(_) => CanCtrlMode::value_len(),
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::BitTiming(v) => v.emit_value(buffer),
            Self::BitTimingConst(v) => v.emit_value(buffer),
            Self::Clock(v) => v.emit_value(buffer),
            Self::State(v) => emit_u32(buffer, *v).unwrap(),
            Self::CtrlMode(v) => v.emit_value(buffer),
            Self::RestartMs(v) => emit_u32(buffer, *v).unwrap(),
            Self::Restart(v) => emit_u32(buffer, *v).unwrap(),
            Self::BerrCounter(v) => v.emit_value(buffer),
            Self::DataBitTiming(v) => v.emit_value(buffer),
            Self::DataBitTimingConst(v) => v.emit_value(buffer),
            Self::Termination(v) => emit_u16(buffer, *v).unwrap(),
            Self::BitTimingMax(v) => v.emit_value(buffer),
            Self::CcLen(v) => emit_u32(buffer, *v).unwrap(),
            Self::DataBitTimingMax(v) => v.emit_value(buffer),
            Self::Tdc(v) => v.emit_value(buffer),
            Self::CtrlModeExt(v) => v.emit_value(buffer),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::BitTiming(_) => IFLA_CAN_BITTIMING,
            Self::BitTimingConst(_) => IFLA_CAN_BITTIMING_CONST,
            Self::Clock(_) => IFLA_CAN_CLOCK,
            Self::State(_) => IFLA_CAN_STATE,
            Self::CtrlMode(_) => IFLA_CAN_CTRLMODE,
            Self::RestartMs(_) => IFLA_CAN_RESTART_MS,
            Self::Restart(_) => IFLA_CAN_RESTART,
            Self::BerrCounter(_) => IFLA_CAN_BERR_COUNTER,
            Self::DataBitTiming(_) => IFLA_CAN_DATA_BITTIMING,
            Self::DataBitTimingConst(_) => IFLA_CAN_DATA_BITTIMING_CONST,
            Self::Termination(_) => IFLA_CAN_TERMINATION,
            Self::BitTimingMax(_) => IFLA_CAN_BITTIMING_MAX,
            Self::CcLen(_) => IFLA_CAN_CC_LEN,
            Self::DataBitTimingMax(_) => IFLA_CAN_DATA_BITTIMING_MAX,
            Self::Tdc(_) => IFLA_CAN_TDC,
            Self::CtrlModeExt(_) => IFLA_CAN_CTRLMODE_EXT,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoCan {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_CAN_BITTIMING => Self::BitTiming(
                CanBitTiming::parse(payload)
                    .context("invalid IFLA_CAN_BITTIMING")?,
            ),
            IFLA_CAN_BITTIMING_CONST => Self::BitTimingConst(
                CanBitTimingConst::parse(payload)
                    .context("invalid IFLA_CAN_BITTIMING_CONST")?,
            ),
            IFLA_CAN_CLOCK => Self::Clock(
                CanClock::parse(payload).context("invalid IFLA_CAN_CLOCK")?,
            ),
            IFLA_CAN_STATE => Self::State(
                parse_u32(payload).context("invalid IFLA_CAN_STATE value")?,
            ),
            IFLA_CAN_CTRLMODE => Self::CtrlMode(
                CanCtrlMode::parse(payload)
                    .context("invalid IFLA_CAN_CTRLMODE")?,
            ),
            IFLA_CAN_RESTART_MS => Self::RestartMs(
                parse_u32(payload)
                    .context("invalid IFLA_CAN_RESTART_MS value")?,
            ),
            IFLA_CAN_RESTART => Self::Restart(
                parse_u32(payload).context("invalid IFLA_CAN_RESTART value")?,
            ),
            IFLA_CAN_BERR_COUNTER => Self::BerrCounter(
                CanBerrCounter::parse(payload)
                    .context("invalid IFLA_CAN_BERR_COUNTER")?,
            ),
            IFLA_CAN_DATA_BITTIMING => Self::DataBitTiming(
                CanBitTiming::parse(payload)
                    .context("invalid IFLA_CAN_DATA_BITTIMING")?,
            ),
            IFLA_CAN_DATA_BITTIMING_CONST => Self::DataBitTimingConst(
                CanBitTimingConst::parse(payload)
                    .context("invalid IFLA_CAN_DATA_BITTIMING_CONST")?,
            ),
            IFLA_CAN_TERMINATION => Self::Termination(
                parse_u16(payload)
                    .context("invalid IFLA_CAN_TERMINATION value")?,
            ),
            IFLA_CAN_BITTIMING_MAX => Self::BitTimingMax(
                CanBitTiming::parse(payload)
                    .context("invalid IFLA_CAN_BITTIMING_MAX")?,
            ),
            IFLA_CAN_CC_LEN => Self::CcLen(
                parse_u32(payload).context("invalid IFLA_CAN_CC_LEN value")?,
            ),
            IFLA_CAN_DATA_BITTIMING_MAX => Self::DataBitTimingMax(
                CanBitTiming::parse(payload)
                    .context("invalid IFLA_CAN_DATA_BITTIMING_MAX")?,
            ),
            IFLA_CAN_TDC => Self::Tdc(
                CanTdc::parse(payload).context("invalid IFLA_CAN_TDC")?,
            ),
            IFLA_CAN_CTRLMODE_EXT => Self::CtrlModeExt(
                CanCtrlMode::parse(payload)
                    .context("invalid IFLA_CAN_CTRLMODE_EXT")?,
            ),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(can)"
            ))?),
        })
    }
}
