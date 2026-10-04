// SPDX-License-Identifier: MIT

// TODO: If you have hardware to simulate DPLL pins, please add unit tests for
// this module.

use netlink_packet_core::{
    emit_u32, parse_u32, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    Parseable,
};

const DPLL_A_PIN_ID: u16 = 1;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum DpllPin {
    /// The unique pin identifier within the DPLL subsystem.
    PinId(u32),
    Other(DefaultNla),
}

impl Nla for DpllPin {
    fn value_len(&self) -> usize {
        match self {
            Self::PinId(_) => 4,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::PinId(v) => emit_u32(buffer, *v).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::PinId(_) => DPLL_A_PIN_ID,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for DpllPin {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            DPLL_A_PIN_ID => Self::PinId(
                parse_u32(payload).context("invalid DPLL_A_PIN_ID value")?,
            ),
            _ => {
                Self::Other(DefaultNla::parse(buf).context(format!(
                    "unknown DPLL_A_PIN type {}",
                    buf.kind()
                ))?)
            }
        })
    }
}
