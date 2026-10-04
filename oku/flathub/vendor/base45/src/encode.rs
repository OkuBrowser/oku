use crate::alphabet::{self, SIZE, SIZE_SIZE};

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

fn divmod<const N: u32>(x: u32) -> (u32, u32) {
    (x / N, x % N)
}

#[inline(always)]
fn ae(b: u8) -> u8 {
    match alphabet::encode(b) {
        Some(ch) => ch,
        #[cfg(feature = "unsafe")]
        // SAFETY: encode for this is highly unlikely to ever reach this point.
        None => unsafe { core::hint::unreachable_unchecked() },
        #[cfg(not(feature = "unsafe"))]
        None => unreachable!(),
    }
}

fn encode_buffer(input: &[u8]) -> String {
    // setup
    let (input, remainder) = input.as_chunks::<2>();

    let mut s = Vec::with_capacity(input.len() + input.len().div_ceil(2));

    // Core function
    #[inline(always)]
    fn core_fn([_first, _second]: [u8; 2], s: &mut Vec<u8>) {
        let v = (_first as u32 * 256) + _second as u32;
        let (e, rest) = divmod::<SIZE_SIZE>(v);
        let (d, c) = divmod::<SIZE>(rest);

        s.extend_from_slice(&[ae(c as _), ae(d as _), ae(e as _)]);
    }
    // take remainder AoT
    for c in input {
        core_fn(*c, &mut s);
    }
    // Final byte
    if let &[_0] = remainder {
        let (d, c) = divmod::<SIZE>(_0 as u32);

        s.extend_from_slice(&[ae(c as _), ae(d as _)]);
    }
    #[cfg(feature = "unsafe")]
    // SAFETY: we control all bytes that enter this vector.
    unsafe {
        String::from_utf8_unchecked(s)
    }
    #[cfg(not(feature = "unsafe"))]
    String::from_utf8(s).expect("All bytes encoded must be ascii")
}

/// Encode a string to base45
///
/// The function takes a string containing only characters in
/// in the range U+0000 to U+00FF.
///
/// ```rust,no_run
/// # fn main() {
/// let encoded = base45::encode("Hello!!");
/// assert_eq!(encoded, "%69 VD92EX0");
/// # }
/// ```
pub fn encode(input: impl AsRef<[u8]>) -> String {
    encode_buffer(input.as_ref())
}

/// Encode a buffer to base45
///
/// The function takes an arbitrary buffer and encodes it to base45.
///
/// ## Updates
///
/// As of v3.1.0, this function is exactly eqivalent to encode.
///
/// ```rust,no_run
/// # fn main() {
/// let encoded = base45::encode_from_buffer(vec![72,101,108,108,111,33,33]);
/// assert_eq!(encoded, "%69 VD92EX0");
/// # }
/// ```
#[deprecated(
    since = "3.1.0",
    note = "Equivalent to `encode`. Use `encode` instead."
)]
pub fn encode_from_buffer(input: impl AsRef<[u8]>) -> String {
    encode_buffer(input.as_ref())
}
