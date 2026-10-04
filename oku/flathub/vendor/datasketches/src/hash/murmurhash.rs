// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::hash::Hasher;

use crate::hash::DEFAULT_UPDATE_SEED;

const C1: u64 = 0x87c37b91114253d5;
const C2: u64 = 0x4cf5ad432745937f;

/// The MurmurHash3 is a fast, non-cryptographic, 128-bit hash function that has
/// excellent avalanche and 2-way bit independence properties.
#[derive(Debug)]
pub struct MurmurHash3X64128 {
    h1: u64,
    h2: u64,
    total: u64,
    buf: [u8; 16],
    buf_len: usize,
}

impl MurmurHash3X64128 {
    pub fn with_seed(seed: u64) -> Self {
        MurmurHash3X64128 {
            h1: seed,
            h2: seed,
            total: 0,
            buf: [0; 16],
            buf_len: 0,
        }
    }

    pub fn finish128(&self) -> (u64, u64) {
        let mut h1 = self.h1;
        let mut h2 = self.h2;

        let total = self.total + self.buf_len as u64;
        let rem = self.buf_len;

        // tail
        if rem > 0 {
            if rem > 8 {
                // read k2 little endian
                let mut k2 = super::read_u64_le(&self.buf[8..rem]);
                // mix k2
                k2 = k2.wrapping_mul(C2);
                k2 = k2.rotate_left(33);
                k2 = k2.wrapping_mul(C1);
                h2 ^= k2;
            }

            // read k1 little endian
            let k1_len = rem.min(8);
            let mut k1 = super::read_u64_le(&self.buf[..k1_len]);
            // mix k1
            k1 = k1.wrapping_mul(C1);
            k1 = k1.rotate_left(31);
            k1 = k1.wrapping_mul(C2);
            h1 ^= k1;
        }

        h1 ^= total;
        h2 ^= total;
        h1 = h1.wrapping_add(h2);
        h2 = h2.wrapping_add(h1);
        h1 = fmix64(h1);
        h2 = fmix64(h2);
        h1 = h1.wrapping_add(h2);
        h2 = h2.wrapping_add(h1);
        (h1, h2)
    }

    #[inline]
    fn update(&mut self, mut k1: u64, mut k2: u64) {
        // k1 *= c1; k1 = MURMUR3_ROTL64(k1, 31); k1 *= c2; out.h1 ^= k1;
        k1 = k1.wrapping_mul(C1);
        k1 = k1.rotate_left(31);
        k1 = k1.wrapping_mul(C2);
        self.h1 ^= k1;

        // out.h1 = MURMUR3_ROTL64(out.h1, 27); out.h1 += out.h2; out.h1 = out.h1*5+0x52dce729;
        self.h1 = self.h1.rotate_left(27);
        self.h1 = self.h1.wrapping_add(self.h2);
        self.h1 = self.h1.wrapping_mul(5).wrapping_add(0x52dce729);

        // k2 *= c2; k2 = MURMUR3_ROTL64(k2, 33); k2 *= c1; out.h2 ^= k2;
        k2 = k2.wrapping_mul(C2);
        k2 = k2.rotate_left(33);
        k2 = k2.wrapping_mul(C1);
        self.h2 ^= k2;

        // out.h2 = MURMUR3_ROTL64(out.h2,31); out.h2 += out.h1; out.h2 = out.h2*5+c4;
        self.h2 = self.h2.rotate_left(31);
        self.h2 = self.h2.wrapping_add(self.h1);
        self.h2 = self.h2.wrapping_mul(5).wrapping_add(0x38495ab5);

        // accumulate total length
        self.total += 16;
    }
}

impl Default for MurmurHash3X64128 {
    fn default() -> Self {
        Self::with_seed(DEFAULT_UPDATE_SEED)
    }
}

impl Hasher for MurmurHash3X64128 {
    fn finish(&self) -> u64 {
        self.finish128().0
    }

    fn write(&mut self, mut bytes: &[u8]) {
        if self.buf_len + bytes.len() < 16 {
            self.buf[self.buf_len..self.buf_len + bytes.len()].copy_from_slice(bytes);
            self.buf_len += bytes.len();
            return;
        }

        if self.buf_len != 0 {
            let wanted = 16 - self.buf_len;
            self.buf[self.buf_len..].copy_from_slice(&bytes[..wanted]);

            let k1 = super::read_u64_le(&self.buf[0..8]);
            let k2 = super::read_u64_le(&self.buf[8..16]);
            self.update(k1, k2);

            bytes = &bytes[wanted..];
            self.buf_len = 0;
        }

        // Number of full 128-bit blocks of 16 bytes.
        // Possible exclusion of a remainder of up to 15 bytes.
        let blocks = bytes.len() >> 4; // bytes / 16

        // Process the 128-bit blocks (the body) into the hash
        for i in 0..blocks {
            let lo = i << 4;
            let mi = lo + 8;
            let hi = mi + 8;
            let k1 = super::read_u64_le(&bytes[lo..mi]);
            let k2 = super::read_u64_le(&bytes[mi..hi]);
            self.update(k1, k2);
        }

        // remain bytes
        let len = bytes.len() % 16;
        if len > 0 {
            self.buf[0..len].copy_from_slice(&bytes[blocks << 4..]);
            self.buf_len = len;
        }
    }
}

/// Finalization mix: force all bits of a hash block to avalanche.
#[inline]
fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51afd7ed558ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
    k ^ (k >> 33)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn murmurhash3_x64_128(key: &[u8], seed: u64) -> (u64, u64) {
        let mut hasher = MurmurHash3X64128::with_seed(seed);
        hasher.write(key);
        hasher.finish128()
    }

    #[test]
    fn test_remainder() {
        // remainder > 8
        let key = "The quick brown fox jumps over the lazy dog";
        let (h1, h2) = murmurhash3_x64_128(key.as_bytes(), 0);
        assert_eq!(h1, 0xe34bbc7bbc071b6c);
        assert_eq!(h2, 0x7a433ca9c49a9347);

        // change one bit
        let key = "The quick brown fox jumps over the lazy eog";
        let (h1, h2) = murmurhash3_x64_128(key.as_bytes(), 0);
        assert_eq!(h1, 0x362108102c62d1c9);
        assert_eq!(h2, 0x3285cd100292b305);

        // test a remainder < 8
        let key = "The quick brown fox jumps over the lazy dogdogdog";
        let (h1, h2) = murmurhash3_x64_128(key.as_bytes(), 0);
        assert_eq!(h1, 0x9c8205300e612fc4);
        assert_eq!(h2, 0xcbc0af6136aa3df9);

        // test a remainder = 8
        let key = "The quick brown fox jumps over the lazy1";
        let (h1, h2) = murmurhash3_x64_128(key.as_bytes(), 0);
        assert_eq!(h1, 0xe3301a827e5cdfe3);
        assert_eq!(h2, 0xbdbf05f8da0f0392);

        // test a remainder = 0
        let key = "The quick brown fox jumps over t";
        let (h1, h2) = murmurhash3_x64_128(key.as_bytes(), 0);
        assert_eq!(h1, 0xdf6af91bb29bdacf);
        assert_eq!(h2, 0x91a341c58df1f3a6);

        // test a ones byte and a zeros byte
        let key = [
            0x54, 0x68, 0x65, 0x20, 0x71, 0x75, 0x69, 0x63, 0x6b, 0x20, 0x62, 0x72, 0x6f, 0x77,
            0x6e, 0x20, 0x66, 0x6f, 0x78, 0x20, 0x6a, 0x75, 0x6d, 0x70, 0x73, 0x20, 0x6f, 0x76,
            0x65, 0x72, 0x20, 0x74, 0x68, 0x65, 0x20, 0x6c, 0x61, 0x7a, 0x79, 0x20, 0x64, 0x6f,
            0x67, 0xff, 0x64, 0x6f, 0x67, 0x00,
        ];
        let (h1, h2) = murmurhash3_x64_128(&key, 0);
        assert_eq!(h1, 0xe88abda785929c9e);
        assert_eq!(h2, 0x96b98587cacc83d6);
    }
}
