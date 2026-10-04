use std::slice;

#[derive(Clone)]
pub struct FullCharacteristicVector(Vec<u32>);

impl FullCharacteristicVector {
    pub fn shift_and_mask(&self, offset: usize, mask: u32) -> u32 {
        let bucket_id = offset / 32;
        let align = offset - bucket_id * 32;
        if align == 0 {
            self.0[bucket_id] & mask
        } else {
            let left = (self.0[bucket_id] >> align) as u32;
            let right = self.0[bucket_id + 1] << (32 - align) as u32;
            (left | right) & mask
        }
    }
}

pub struct Alphabet {
    charset: Vec<(char, FullCharacteristicVector)>,
}

impl Alphabet {
    pub fn iter(&self) -> slice::Iter<(char, FullCharacteristicVector)> {
        self.charset.iter()
    }

    pub fn for_query_chars(query_chars: &[char]) -> Alphabet {
        let mut charset = Vec::from(query_chars);
        charset.sort();
        charset.dedup();
        let charset = charset
            .into_iter()
            .map(|c| {
                let mut bits: Vec<u32> = query_chars
                    .chunks(32)
                    .map(|chunk| {
                        let mut chunk_bits = 0u32;
                        let mut bit = 1u32;
                        for &chr in chunk {
                            if chr == c {
                                chunk_bits |= bit;
                            }
                            bit <<= 1;
                        }
                        chunk_bits
                    })
                    .collect();
                bits.push(0u32);
                (c, FullCharacteristicVector(bits))
            })
            .collect();
        Alphabet { charset: charset }
    }
}

#[cfg(test)]
mod tests {
    use super::{Alphabet, FullCharacteristicVector};

    #[test]
    fn test_alphabet() {
        let chars: Vec<char> = "happy".chars().collect();
        let alphabet = Alphabet::for_query_chars(&chars);
        let mut it = alphabet.iter();

        {
            let &(ref c, ref chi) = it.next().unwrap();
            assert_eq!(*c, 'a');
            assert_eq!(chi.0[0], 2u32);
        }
        {
            let &(ref c, ref chi) = it.next().unwrap();
            assert_eq!(*c, 'h');
            assert_eq!(chi.0[0], 1u32);
        }
        {
            let &(ref c, ref chi) = it.next().unwrap();
            assert_eq!(*c, 'p');
            assert_eq!(chi.0[0], 4u32 + 8u32);
        }
        {
            let &(ref c, ref chi) = it.next().unwrap();
            assert_eq!(*c, 'y');
            assert_eq!(chi.0[0], 16u32);
        }
    }

    #[test]
    fn test_full_characteristic() {
        assert_eq!(
            FullCharacteristicVector(vec![2u32, 0u32]).shift_and_mask(1, 1u32),
            1
        );
        assert_eq!(
            FullCharacteristicVector(vec![(1u32 << 5) + (1u32 << 10), 0u32])
                .shift_and_mask(3, 63u32),
            4
        );
    }

    #[test]
    fn test_long_characteristic() {
        let query_chars: Vec<char> = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaabcabewa".chars().collect();
        let alphabet = Alphabet::for_query_chars(&query_chars[..]);
        let mut alphabet_it = alphabet.iter();
        {
            let &(ref c, ref chi) = alphabet_it.next().unwrap();
            assert_eq!(*c, 'a');
            assert_eq!(chi.shift_and_mask(0, 7), 7);
            assert_eq!(chi.shift_and_mask(28, 7), 3);
            assert_eq!(chi.shift_and_mask(28, 127), 1 + 2 + 16);
            assert_eq!(chi.shift_and_mask(28, 4095), 1 + 2 + 16 + 256);
        }
        {
            let &(ref c, ref chi) = alphabet_it.next().unwrap();
            assert_eq!(*c, 'b');
            assert_eq!(chi.shift_and_mask(0, 7), 0);
            assert_eq!(chi.shift_and_mask(28, 15), 4);
            assert_eq!(chi.shift_and_mask(28, 63), 4 + 32);
        }
    }
}
