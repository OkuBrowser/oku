use crate::alphabet::{self, SIZE, SIZE_SIZE};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use core::fmt::{Display, Formatter};
/// The error type returned when the input is not a valid base45 string
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct DecodeError;

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str("Invalid base45 string")
    }
}

impl core::error::Error for DecodeError {}

/// Decode a string from base45
///
/// Takes a base45 encoded string and returns a UTF-8 string on success
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn core::error::Error>> {
/// let decoded = String::from_utf8(base45::decode("%69 VD92EX0")?)?;
/// assert_eq!(decoded, "Hello!!");
/// # Ok(())
/// # }
/// ```
pub fn decode(input: impl AsRef<[u8]>) -> Result<Vec<u8>, DecodeError> {
    decode_intl(input.as_ref())
}
/// Internal function to decode a string from base45, to reduce code bloat.
fn decode_intl(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut output = Vec::with_capacity(match input.len() & 1 {
        0 => input.len() * 2 / 3,
        1 => 1 + input.len() * 2 / 3,
        #[cfg(feature = "unsafe")]
        // SAFETY: it's one of 0 or 1. There are no other options.
        _ => unsafe { core::hint::unreachable_unchecked() },
        #[cfg(not(feature = "unsafe"))]
        _ => unreachable!(),
    });

    #[inline(always)]
    fn core_fn([_first, _second, _third]: [u8; 3]) -> Result<[u8; 2], DecodeError> {
        let v = (_first as u32) + (_second as u32 * SIZE) + (_third as u32) * SIZE_SIZE;
        if v <= (u16::MAX as _) {
            let x = (v >> 8) & 0xFF;
            let y = v & 0xFF;
            Ok([x as u8, y as u8])
        } else {
            Err(DecodeError)
        }
    }
    //short
    #[inline(always)]
    fn preproc([a, b, c]: [u8; 3]) -> Result<[u8; 3], DecodeError> {
        Ok([d(a)?, d(b)?, d(c)?])
    }
    //short
    #[inline(always)]
    fn d(v: u8) -> Result<u8, DecodeError> {
        alphabet::decode(v).ok_or(DecodeError)
    }

    let ((chunks, remainder), pre) = (input.as_chunks::<3>(), |&b| preproc(b));

    for c in chunks.iter().map(pre) {
        let xy = core_fn(c?)?;
        output.extend_from_slice(&xy);
    }
    match remainder {
        [_first, _second] => output.push((d(*_first)? as u32 + (d(*_second)? as u32 * SIZE)) as u8),
        [] => {}
        _ => return Err(DecodeError),
    }

    Ok(output)
}
