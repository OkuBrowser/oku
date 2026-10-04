use std::convert::TryInto;

const SEED: u32 = 3_242_157_231u32;
const M: u32 = 0x5bd1_e995;

#[inline]
pub fn murmurhash2(key: &[u8]) -> u32 {
    let mut h: u32 = SEED ^ (key.len() as u32);

    let mut four_bytes_chunks = key.chunks_exact(4);

    while let Some(chunk) = four_bytes_chunks.next() {
        let mut k: u32 = u32::from_le_bytes(chunk.try_into().unwrap());
        k = k.wrapping_mul(M);
        k ^= k >> 24;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M);
        h ^= k;
    }
    let remainder = four_bytes_chunks.remainder();

    // Handle the last few bytes of the input array
    match remainder.len() {
        3 => {
            h ^= u32::from(remainder[2]) << 16;
            h ^= u32::from(remainder[1]) << 8;
            h ^= u32::from(remainder[0]);
            h = h.wrapping_mul(M);
        }
        2 => {
            h ^= u32::from(remainder[1]) << 8;
            h ^= u32::from(remainder[0]);
            h = h.wrapping_mul(M);
        }
        1 => {
            h ^= u32::from(remainder[0]);
            h = h.wrapping_mul(M);
        }
        _ => {}
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^ (h >> 15)
}

#[cfg(test)]
mod test {

    use super::murmurhash2;
    use std::collections::HashSet;

    #[test]
    fn test_murmur2() {
        let s1 = "abcdef";
        let s2 = "abcdeg";
        for i in 0..5 {
            assert_eq!(
                murmurhash2(&s1[i..5].as_bytes()),
                murmurhash2(&s2[i..5].as_bytes())
            );
        }
    }

    #[test]
    fn test_murmur_against_reference_impl() {
        assert_eq!(murmurhash2("".as_bytes()), 3_632_506_080);
        assert_eq!(murmurhash2("a".as_bytes()), 455_683_869);
        assert_eq!(murmurhash2("ab".as_bytes()), 2_448_092_234);
        assert_eq!(murmurhash2("abc".as_bytes()), 2_066_295_634);
        assert_eq!(murmurhash2("abcd".as_bytes()), 2_588_571_162);
        assert_eq!(murmurhash2("abcde".as_bytes()), 29_886_969_42);
        assert_eq!(murmurhash2("abcdefghijklmnop".as_bytes()), 2_350_868_870);
    }

    #[test]
    fn test_murmur_collisions() {
        let mut set: HashSet<u32> = HashSet::default();
        for i in 0..10_000 {
            let s = format!("hash{}", i);
            let hash = murmurhash2(s.as_bytes());
            set.insert(hash);
        }
        assert_eq!(set.len(), 10_000);
    }
}
