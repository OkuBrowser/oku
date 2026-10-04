mod murmurhash2;
mod murmurhash3;

pub use self::murmurhash2::murmurhash2;
pub use self::murmurhash3::murmurhash3;

#[cfg(test)]
mod test {

    use super::{murmurhash2, murmurhash3};
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
    fn test_murmur3() {
        assert_eq!(murmurhash3(b""), 36_859_204);
        assert_eq!(murmurhash3(b"a"), 3_144_985_375);
        assert_eq!(murmurhash3(b"ab"), 3_262_304_301);
        assert_eq!(murmurhash3(b"abc"), 476_091_040);
        assert_eq!(murmurhash3(b"abcd"), 412_992_581);
        assert_eq!(murmurhash3(b"abcde"), 2_747_833_956);
        assert_eq!(murmurhash3(b"abcdefghijklmnop"), 2_078_305_053);
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
