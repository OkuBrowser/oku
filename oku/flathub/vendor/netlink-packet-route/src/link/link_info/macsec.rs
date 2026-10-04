// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u16, emit_u32, emit_u64, parse_u16, parse_u32, parse_u64, parse_u8,
    DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer, Parseable,
};

const IFLA_MACSEC_SCI: u16 = 1;
const IFLA_MACSEC_PORT: u16 = 2;
const IFLA_MACSEC_ICV_LEN: u16 = 3;
const IFLA_MACSEC_CIPHER_SUITE: u16 = 4;
const IFLA_MACSEC_WINDOW: u16 = 5;
const IFLA_MACSEC_ENCODING_SA: u16 = 6;
const IFLA_MACSEC_ENCRYPT: u16 = 7;
const IFLA_MACSEC_PROTECT: u16 = 8;
const IFLA_MACSEC_INC_SCI: u16 = 9;
const IFLA_MACSEC_ES: u16 = 10;
const IFLA_MACSEC_SCB: u16 = 11;
const IFLA_MACSEC_REPLAY_PROTECT: u16 = 12;
const IFLA_MACSEC_VALIDATION: u16 = 13;
// const IFLA_MACSEC_PAD: u16 = 14;
const IFLA_MACSEC_OFFLOAD: u16 = 15;
const MACSEC_VALIDATE_DISABLED: u8 = 0;
const MACSEC_VALIDATE_CHECK: u8 = 1;
const MACSEC_VALIDATE_STRICT: u8 = 2;
const MACSEC_OFFLOAD_OFF: u8 = 0;
const MACSEC_OFFLOAD_PHY: u8 = 1;
const MACSEC_OFFLOAD_MAC: u8 = 2;
const MACSEC_CIPHER_ID_GCM_AES_128: u64 = 0x0080C20001000001;
const MACSEC_CIPHER_ID_GCM_AES_256: u64 = 0x0080C20001000002;
const MACSEC_CIPHER_ID_GCM_AES_XPN_128: u64 = 0x0080C20001000003;
const MACSEC_CIPHER_ID_GCM_AES_XPN_256: u64 = 0x0080C20001000004;
const MACSEC_DEFAULT_CIPHER_ID: u64 = 0x0080020001000001;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum MacSecCipherId {
    #[deprecated]
    DefaultGcmAes128,
    GcmAes128,
    GcmAes256,
    GcmAesXpn128,
    GcmAesXpn256,
    Other(u64),
}

impl From<u64> for MacSecCipherId {
    fn from(d: u64) -> Self {
        match d {
            #[allow(deprecated)]
            MACSEC_DEFAULT_CIPHER_ID => Self::DefaultGcmAes128,
            MACSEC_CIPHER_ID_GCM_AES_128 => Self::GcmAes128,
            MACSEC_CIPHER_ID_GCM_AES_256 => Self::GcmAes256,
            MACSEC_CIPHER_ID_GCM_AES_XPN_128 => Self::GcmAesXpn128,
            MACSEC_CIPHER_ID_GCM_AES_XPN_256 => Self::GcmAesXpn256,
            _ => Self::Other(d),
        }
    }
}

impl From<MacSecCipherId> for u64 {
    fn from(d: MacSecCipherId) -> Self {
        match d {
            #[allow(deprecated)]
            MacSecCipherId::DefaultGcmAes128 => MACSEC_DEFAULT_CIPHER_ID,
            MacSecCipherId::GcmAes128 => MACSEC_CIPHER_ID_GCM_AES_128,
            MacSecCipherId::GcmAes256 => MACSEC_CIPHER_ID_GCM_AES_256,
            MacSecCipherId::GcmAesXpn128 => MACSEC_CIPHER_ID_GCM_AES_XPN_128,
            MacSecCipherId::GcmAesXpn256 => MACSEC_CIPHER_ID_GCM_AES_XPN_256,
            MacSecCipherId::Other(value) => value,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum MacSecValidate {
    Disabled,
    Check,
    Strict,
    Other(u8),
}

impl From<u8> for MacSecValidate {
    fn from(d: u8) -> Self {
        match d {
            MACSEC_VALIDATE_DISABLED => Self::Disabled,
            MACSEC_VALIDATE_CHECK => Self::Check,
            MACSEC_VALIDATE_STRICT => Self::Strict,
            _ => Self::Other(d),
        }
    }
}

impl From<MacSecValidate> for u8 {
    fn from(d: MacSecValidate) -> Self {
        match d {
            MacSecValidate::Disabled => MACSEC_VALIDATE_DISABLED,
            MacSecValidate::Check => MACSEC_VALIDATE_CHECK,
            MacSecValidate::Strict => MACSEC_VALIDATE_STRICT,
            MacSecValidate::Other(value) => value,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum MacSecOffload {
    Off,
    Phy,
    Mac,
    Other(u8),
}

impl From<u8> for MacSecOffload {
    fn from(d: u8) -> Self {
        match d {
            MACSEC_OFFLOAD_OFF => Self::Off,
            MACSEC_OFFLOAD_PHY => Self::Phy,
            MACSEC_OFFLOAD_MAC => Self::Mac,
            _ => Self::Other(d),
        }
    }
}

impl From<MacSecOffload> for u8 {
    fn from(d: MacSecOffload) -> Self {
        match d {
            MacSecOffload::Off => MACSEC_OFFLOAD_OFF,
            MacSecOffload::Phy => MACSEC_OFFLOAD_PHY,
            MacSecOffload::Mac => MACSEC_OFFLOAD_MAC,
            MacSecOffload::Other(value) => value,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoMacSec {
    Sci(u64),
    Port(u16),
    IcvLen(u8),
    CipherSuite(MacSecCipherId),
    Window(u32),
    EncodingSa(u8),
    Encrypt(u8),
    Protect(u8),
    IncSci(u8),
    Es(u8),
    Scb(u8),
    ReplayProtect(u8),
    Validation(MacSecValidate),
    Offload(MacSecOffload),
    Other(DefaultNla),
}

impl Nla for InfoMacSec {
    fn value_len(&self) -> usize {
        match self {
            Self::Sci(_) | Self::CipherSuite(_) => 8,
            Self::Window(_) => 4,
            Self::Port(_) => 2,
            Self::IcvLen(_)
            | Self::EncodingSa(_)
            | Self::Encrypt(_)
            | Self::Protect(_)
            | Self::IncSci(_)
            | Self::Es(_)
            | Self::Scb(_)
            | Self::ReplayProtect(_)
            | Self::Validation(_)
            | Self::Offload(_) => 1,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Sci(value) => emit_u64(buffer, *value).unwrap(),
            Self::CipherSuite(value) => {
                emit_u64(buffer, (*value).into()).unwrap()
            }
            Self::Window(value) => emit_u32(buffer, *value).unwrap(),
            Self::Port(value) => emit_u16(buffer, *value).unwrap(),
            Self::IcvLen(value)
            | Self::EncodingSa(value)
            | Self::Encrypt(value)
            | Self::Protect(value)
            | Self::IncSci(value)
            | Self::Es(value)
            | Self::Scb(value)
            | Self::ReplayProtect(value) => buffer[0] = *value,
            Self::Offload(value) => buffer[0] = (*value).into(),
            Self::Validation(value) => buffer[0] = (*value).into(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Sci(_) => IFLA_MACSEC_SCI,
            Self::Port(_) => IFLA_MACSEC_PORT,
            Self::IcvLen(_) => IFLA_MACSEC_ICV_LEN,
            Self::CipherSuite(_) => IFLA_MACSEC_CIPHER_SUITE,
            Self::Window(_) => IFLA_MACSEC_WINDOW,
            Self::EncodingSa(_) => IFLA_MACSEC_ENCODING_SA,
            Self::Encrypt(_) => IFLA_MACSEC_ENCRYPT,
            Self::Protect(_) => IFLA_MACSEC_PROTECT,
            Self::IncSci(_) => IFLA_MACSEC_INC_SCI,
            Self::Es(_) => IFLA_MACSEC_ES,
            Self::Scb(_) => IFLA_MACSEC_SCB,
            Self::ReplayProtect(_) => IFLA_MACSEC_REPLAY_PROTECT,
            Self::Validation(_) => IFLA_MACSEC_VALIDATION,
            Self::Offload(_) => IFLA_MACSEC_OFFLOAD,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoMacSec {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_MACSEC_SCI => Self::Sci(
                parse_u64(payload).context("invalid IFLA_MACSEC_SCI value")?,
            ),
            IFLA_MACSEC_PORT => Self::Port(
                parse_u16(payload).context("invalid IFLA_MACSEC_PORT value")?,
            ),
            IFLA_MACSEC_ICV_LEN => Self::IcvLen(
                parse_u8(payload)
                    .context("invalid IFLA_MACSEC_ICV_LEN value")?,
            ),
            IFLA_MACSEC_CIPHER_SUITE => Self::CipherSuite(
                parse_u64(payload)
                    .context("invalid IFLA_MACSEC_CIPHER_SUITE value")?
                    .into(),
            ),
            IFLA_MACSEC_WINDOW => Self::Window(
                parse_u32(payload)
                    .context("invalid IFLA_MACSEC_WINDOW value")?,
            ),
            IFLA_MACSEC_ENCODING_SA => Self::EncodingSa(
                parse_u8(payload)
                    .context("invalid IFLA_MACSEC_ENCODING_SA value")?,
            ),
            IFLA_MACSEC_ENCRYPT => Self::Encrypt(
                parse_u8(payload)
                    .context("invalid IFLA_MACSEC_ENCRYPT value")?,
            ),
            IFLA_MACSEC_PROTECT => Self::Protect(
                parse_u8(payload)
                    .context("invalid IFLA_MACSEC_PROTECT value")?,
            ),
            IFLA_MACSEC_INC_SCI => Self::IncSci(
                parse_u8(payload)
                    .context("invalid IFLA_MACSEC_INC_SCI value")?,
            ),
            IFLA_MACSEC_ES => Self::Es(
                parse_u8(payload).context("invalid IFLA_MACSEC_ES value")?,
            ),
            IFLA_MACSEC_SCB => Self::Scb(
                parse_u8(payload).context("invalid IFLA_MACSEC_SCB value")?,
            ),
            IFLA_MACSEC_REPLAY_PROTECT => Self::ReplayProtect(
                parse_u8(payload)
                    .context("invalid IFLA_MACSEC_REPLAY_PROTECT value")?,
            ),
            IFLA_MACSEC_VALIDATION => Self::Validation(
                parse_u8(payload)
                    .context("invalid IFLA_MACSEC_VALIDATION value")?
                    .into(),
            ),
            IFLA_MACSEC_OFFLOAD => Self::Offload(
                parse_u8(payload)
                    .context("invalid IFLA_MACSEC_OFFLOAD value")?
                    .into(),
            ),
            kind => Self::Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind}"))?,
            ),
        })
    }
}
