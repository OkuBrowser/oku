use std::convert::TryInto;

const C1: u32 = 0xcc9e_2d51;
const C2: u32 = 0x1b87_3593;
const D: u32 = 0xe654_6b64;
const SEED: u32 = 3_242_157_231u32;
const FMIX1: u32 = 0x85eb_ca6b;
const FMIX2: u32 = 0xc2b2_ae35;

#[inline(always)]
fn fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(FMIX1);
    h ^= h >> 13;
    h = h.wrapping_mul(FMIX2);
    h ^= h >> 16;
    h
}

#[inline]
pub fn murmurhash3(key: &[u8]) -> u32 {
    let mut h: u32 = SEED;

    let mut four_bytes_chunks = key.chunks_exact(4);

    while let Some(chunk) = four_bytes_chunks.next() {
        let mut k: u32 = u32::from_le_bytes(chunk.try_into().unwrap());
        k = k.wrapping_mul(C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(C2);
        h ^= k;
        h = h.rotate_left(13);
        h = (h.wrapping_mul(5)).wrapping_add(D);
    }

    let remainder = four_bytes_chunks.remainder();
    match remainder.len() {
        3 => {
            let mut k = u32::from(remainder[2]) << 16;
            k ^= u32::from(remainder[1]) << 8;
            k ^= u32::from(remainder[0]);
            k = k.wrapping_mul(C1);
            k = k.rotate_left(15);
            k = k.wrapping_mul(C2);
            h ^= k;
        }
        2 => {
            let mut k = u32::from(remainder[1]) << 8;
            k ^= u32::from(remainder[0]);
            k = k.wrapping_mul(C1);
            k = k.rotate_left(15);
            k = k.wrapping_mul(C2);
            h ^= k;
        }
        1 => {
            let mut k = u32::from(remainder[0]);
            k = k.wrapping_mul(C1);
            k = k.rotate_left(15);
            k = k.wrapping_mul(C2);
            h ^= k;
        }
        _ => {}
    }
    fmix32(h ^ key.len() as u32)
}

#[cfg(test)]
mod test {

    use super::murmurhash3;

    #[test]
    fn test_murmur3() {
        assert_eq!(murmurhash3(b""), 36_859_204);
        assert_eq!(murmurhash3(b"a"), 3_144_985_375);
        assert_eq!(murmurhash3(b"ab"), 3_262_304_301);
        assert_eq!(murmurhash3(b"abc"), 476_091_040);
        assert_eq!(murmurhash3(b"abcd"), 412_992_581);
        assert_eq!(murmurhash3(b"abcde"), 2_747_833_956);
        assert_eq!(murmurhash3(b"abcdefghijklmnop"), 2_078_305_053);
    }
}
