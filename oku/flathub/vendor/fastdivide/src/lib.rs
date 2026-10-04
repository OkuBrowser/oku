/*!
# Yes, but what is it really ?

Division is a very costly operation for your CPU (probably between 10
and 40 cycles).

You may have noticed that when the divisor is known at compile time,
your compiler transforms the operations into a cryptic combination of
a multiplication and bitshift.

Fastdivide is about doing the same trick your compiler uses but
when the divisor is unknown at compile time.
Of course, it requires preprocessing a datastructure that is
specific to your divisor, and using it only makes sense if
this preprocessing is amortized by a high number of division (with the
same divisor).

# When is it useful ?

You should probably use `fastdivide`, if you do a lot (> 10) of division with the same divisor ;
and these divisions are a bottleneck in your program.

This is for instance useful to compute histograms.



# Example

```rust
use fastdivide::DividerU64;

fn histogram(vals: &[u64], min: u64, interval: u64, output: &mut [usize]) {

    // Preprocessing a datastructure dedicated
    // to dividing `u64` by `interval`.
    //
    // This preprocessing is not cheap.
    let divide = DividerU64::divide_by(interval);

    // We reuse the same `Divider` for all of the
    // values in vals.
    for &val in vals {
        if val < min {
            continue;
        }
        let bucket_id = divide.divide(val - min) as usize;
        if bucket_id < output.len() {
            output[bucket_id as usize] += 1;
        }
    }
}

# let mut output = vec![0; 3];
# histogram(&[0u64, 1u64, 4u64, 36u64, 2u64, 1u64], 1u64, 3u64, &mut output[..]);
# assert_eq!(output[0], 3);
# assert_eq!(output[1], 1);
# assert_eq!(output[2], 0);

```

*/
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(test)]
#[macro_use]
extern crate std;

// ported from  libdivide.h by ridiculous_fish
//
//  This file is not the original library, it is an attempt to port part
//  of it to rust.

// This algorithm is described in https://ridiculousfish.com/blog/posts/labor-of-division-episode-i.html

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerU64 {
    Fast { magic: u64, shift: u8 },
    BitShift(u8),
    General { magic_low: u64, shift: u8 },
}

#[inline(always)]
fn libdivide_mullhi_u64(x: u64, y: u64) -> u64 {
    let xl = x as u128;
    let yl = y as u128;
    ((xl * yl) >> 64) as u64
}


#[inline(always)]
fn floor_log2(n: u64) -> u8 {
    assert_ne!(n, 0);
    63u8 - (n.leading_zeros() as u8)
}

impl DividerU64 {
    fn power_of_2_division(divisor: u64) -> Option<DividerU64> {
        if divisor == 0 {
            return None;
        }
        if !divisor.is_power_of_two() {
            return None;
        }
        // Divisor is a power of 2. We can just do a bit shift.
        Some(DividerU64::BitShift(floor_log2(divisor)))
    }

    fn fast_path(divisor: u64) -> Option<DividerU64> {
        if divisor.is_power_of_two() {
            return None;
        }
        let floor_log_2_d: u8 = floor_log2(divisor);
        let u = 1u128 << (floor_log_2_d + 64);
        let proposed_magic_number: u128 = u / divisor as u128;
        let reminder: u64 = (u - proposed_magic_number * (divisor as u128)) as u64;
        assert!(reminder > 0 && reminder < divisor);
        let e: u64 = divisor - reminder;
        // This is a sufficient condition for our 64-bits magic number
        // condition to work as described in
        // See https://ridiculousfish.com/blog/posts/labor-of-division-episode-i.html
        if e >= (1u64 << floor_log_2_d) {
            return None;
        }
        Some(DividerU64::Fast {
            magic: (proposed_magic_number as u64) + 1u64,
            shift: floor_log_2_d,
        })
    }

    fn general_path(divisor: u64) -> DividerU64 {
        // p=⌈log2d⌉
        let p: u8 = 64u8 - (divisor.leading_zeros() as u8);
        // m=⌈2^{64+p} / d⌉. This is a 33 bit number, so keep only the low 32 bits.
        // we do a little dance to avoid the overflow if p = 64.
        let e = 1u128 << (63 + p);
        let m = 2 + (e + (e - divisor as u128)) / divisor as u128;
        DividerU64::General {
            magic_low: m as u64,
            shift: p - 1,
        }
    }

    pub fn divide_by(divisor: u64) -> DividerU64 {
        assert!(divisor > 0u64);
        Self::power_of_2_division(divisor)
            .or_else(|| DividerU64::fast_path(divisor))
            .unwrap_or_else(|| DividerU64::general_path(divisor))
    }

    #[inline(always)]
    pub fn divide(&self, n: u64) -> u64 {
        match *self {
            DividerU64::Fast { magic, shift } => {
                // The divisor has a magic number that is lower than 32 bits.
                // We get away with a multiplication and a bit-shift.
                libdivide_mullhi_u64(magic, n) >> shift
            }
            DividerU64::BitShift(d) => n >> d,
            DividerU64::General { magic_low, shift } => {
                // magic only contains the low 64 bits of our actual magic number which actually has a 65 bits.
                // The following dance computes n * (magic + 2^64) >> shift
                let q = libdivide_mullhi_u64(magic_low, n);
                let t = ((n - q) >> 1).wrapping_add(q);
                t >> shift
            }
        }
    }
}

impl core::ops::Div<DividerU64> for u64 {
    type Output = u64;

    #[inline(always)]
    fn div(self, denom: DividerU64) -> Self::Output {
        denom.divide(self)
    }
}

#[cfg(test)]
mod tests {
    use super::DividerU64;
    use proptest::prelude::*;

    #[test]
    fn test_divide_op() {
        let divider = DividerU64::divide_by(2);
        let res = 4u64 / divider;
        assert_eq!(res, 2);
        let res = 8u64 / divider;
        assert_eq!(res, 4);
    }

    #[test]
    fn test_divide_by_4() {
        let divider = DividerU64::divide_by(4);
        assert!(matches!(divider, DividerU64::BitShift(2)));
    }

    #[test]
    fn test_divide_by_7() {
        let divider = DividerU64::divide_by(7);
        assert!(matches!(divider, DividerU64::General { .. }));
    }

    #[test]
    fn test_divide_by_11() {
        let divider = DividerU64::divide_by(11);
        assert_eq!(
            divider,
            DividerU64::Fast {
                magic: 13415813871788764812,
                shift: 3
            }
        );
    }


    #[test]
    fn test_floor_log2() {
        for i in [1, 2, 3, 4, 10, 15, 16, 31, 32, 33, u64::MAX] {
            let log_i = super::floor_log2(i);
            let lower_bound = 1 << log_i;
            let upper_bound = lower_bound - 1 + lower_bound;
            assert!(lower_bound <= i);
            assert!(upper_bound >= i);
        }
    }


    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn test_proptest(n in 0..u64::MAX, d in 1..u64::MAX) {
            let divider = DividerU64::divide_by(d);
            let quotient = divider.divide(n);
            assert_eq!(quotient, n / d);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn test_proptest_divide_by_7(n in 0..u64::MAX) {
            let divider = DividerU64::divide_by(7);
            let quotient = divider.divide(n);
            assert_eq!(quotient, n / 7);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]
        #[test]
        fn test_proptest_divide_by_11(n in 0..u64::MAX) {
            let divider = DividerU64::divide_by(11);
            let quotient = divider.divide(n);
            assert_eq!(quotient, n / 11);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]
        #[test]
        fn test_proptest_divide_by_any(d in 1..u64::MAX) {
            DividerU64::divide_by(d);
        }
    }

    #[test]
    fn test_libdivide() {
        for d in (1u64..100u64)
            .chain(vec![2048, 234234131223u64].into_iter())
            .chain((5..63).map(|i| 1 << i))
        {
            let divider = DividerU64::divide_by(d);
            for i in (0u64..10_000).chain(vec![2048, 234234131223u64, 1 << 43, 1 << 43 + 1]) {
                assert_eq!(divider.divide(i), i / d);
            }
        }
    }
}
