// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u16, parse_u16, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    Parseable,
};

const IFLA_IPOIB_PKEY: u16 = 1;
const IFLA_IPOIB_MODE: u16 = 2;
const IFLA_IPOIB_UMCAST: u16 = 3;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoIpoib {
    Pkey(u16),
    Mode(IpoibMode),
    UmCast(u16),
    Other(DefaultNla),
}

impl Nla for InfoIpoib {
    fn value_len(&self) -> usize {
        match self {
            Self::Pkey(_) | Self::Mode(_) | Self::UmCast(_) => 2,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Pkey(value) => emit_u16(buffer, *value).unwrap(),
            Self::Mode(value) => emit_u16(buffer, (*value).into()).unwrap(),
            Self::UmCast(value) => emit_u16(buffer, *value).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Pkey(_) => IFLA_IPOIB_PKEY,
            Self::Mode(_) => IFLA_IPOIB_MODE,
            Self::UmCast(_) => IFLA_IPOIB_UMCAST,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoIpoib {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_IPOIB_PKEY => Self::Pkey(
                parse_u16(payload).context("invalid IFLA_IPOIB_PKEY value")?,
            ),
            IFLA_IPOIB_MODE => Self::Mode(
                parse_u16(payload)
                    .context("invalid IFLA_IPOIB_MODE value")?
                    .into(),
            ),
            IFLA_IPOIB_UMCAST => Self::UmCast(
                parse_u16(payload)
                    .context("invalid IFLA_IPOIB_UMCAST value")?,
            ),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(ipoib)"
            ))?),
        })
    }
}

const IPOIB_MODE_DATAGRAM: u16 = 0;
const IPOIB_MODE_CONNECTED: u16 = 1;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum IpoibMode {
    Datagram,
    Connected,
    Other(u16),
}

impl From<u16> for IpoibMode {
    fn from(d: u16) -> Self {
        match d {
            IPOIB_MODE_DATAGRAM => Self::Datagram,
            IPOIB_MODE_CONNECTED => Self::Connected,
            _ => Self::Other(d),
        }
    }
}

impl From<IpoibMode> for u16 {
    fn from(v: IpoibMode) -> u16 {
        match v {
            IpoibMode::Datagram => IPOIB_MODE_DATAGRAM,
            IpoibMode::Connected => IPOIB_MODE_CONNECTED,
            IpoibMode::Other(d) => d,
        }
    }
}
