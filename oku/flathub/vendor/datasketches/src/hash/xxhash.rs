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

const DEFAULT_SEED: u64 = 0;

// Unsigned 64-bit primes from xxhash64.
const P1: u64 = 0x9E3779B185EBCA87;
const P2: u64 = 0xC2B2AE3D27D4EB4F;
const P3: u64 = 0x165667B19E3779F9;
const P4: u64 = 0x85EBCA77C2B2AE63;
const P5: u64 = 0x27D4EB2F165667C5;

/// The XxHash64 is a fast, non-cryptographic, 64-bit hash function that has
/// excellent avalanche and 2-way bit independence properties.
#[derive(Debug)]
pub struct XxHash64 {
    seed: u64,
    total_len: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    v4: u64,
    buffer: [u8; 32],
    buffer_len: usize,
}

impl XxHash64 {
    pub fn with_seed(seed: u64) -> Self {
        XxHash64 {
            seed,
            total_len: 0,
            v1: seed.wrapping_add(P1).wrapping_add(P2),
            v2: seed.wrapping_add(P2),
            v3: seed,
            v4: seed.wrapping_sub(P1),
            buffer: [0; 32],
            buffer_len: 0,
        }
    }

    pub fn finish64(&self) -> u64 {
        let mut hash = if self.total_len >= 32 {
            let mut acc = self
                .v1
                .rotate_left(1)
                .wrapping_add(self.v2.rotate_left(7))
                .wrapping_add(self.v3.rotate_left(12))
                .wrapping_add(self.v4.rotate_left(18));
            acc = merge_round(acc, self.v1);
            acc = merge_round(acc, self.v2);
            acc = merge_round(acc, self.v3);
            acc = merge_round(acc, self.v4);
            acc
        } else {
            self.seed.wrapping_add(P5)
        };

        hash = hash.wrapping_add(self.total_len);

        let mut idx = 0;
        let buf = &self.buffer[..self.buffer_len];
        while idx + 8 <= buf.len() {
            let mut k1 = super::read_u64_le(&buf[idx..idx + 8]);
            k1 = k1.wrapping_mul(P2);
            k1 = k1.rotate_left(31);
            k1 = k1.wrapping_mul(P1);
            hash ^= k1;
            hash = hash.rotate_left(27).wrapping_mul(P1).wrapping_add(P4);
            idx += 8;
        }

        if idx + 4 <= buf.len() {
            let k1 = super::read_u64_le(&buf[idx..idx + 4]);
            hash ^= k1.wrapping_mul(P1);
            hash = hash.rotate_left(23).wrapping_mul(P2).wrapping_add(P3);
            idx += 4;
        }

        while idx < buf.len() {
            let k1 = buf[idx] as u64;
            hash ^= k1.wrapping_mul(P5);
            hash = hash.rotate_left(11).wrapping_mul(P1);
            idx += 1;
        }

        finalize(hash)
    }

    #[allow(dead_code)]
    pub fn hash_u64(input: u64, seed: u64) -> u64 {
        let mut hash = seed.wrapping_add(P5).wrapping_add(8);
        let mut k1 = input;
        k1 = k1.wrapping_mul(P2);
        k1 = k1.rotate_left(31);
        k1 = k1.wrapping_mul(P1);
        hash ^= k1;
        hash = hash.rotate_left(27).wrapping_mul(P1).wrapping_add(P4);
        finalize(hash)
    }

    #[inline]
    fn update(&mut self, chunk: &[u8]) {
        self.v1 = round(self.v1, super::read_u64_le(&chunk[0..8]));
        self.v2 = round(self.v2, super::read_u64_le(&chunk[8..16]));
        self.v3 = round(self.v3, super::read_u64_le(&chunk[16..24]));
        self.v4 = round(self.v4, super::read_u64_le(&chunk[24..32]));
    }
}

impl Default for XxHash64 {
    fn default() -> Self {
        Self::with_seed(DEFAULT_SEED)
    }
}

impl Hasher for XxHash64 {
    fn finish(&self) -> u64 {
        self.finish64()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.total_len = self.total_len.wrapping_add(bytes.len() as u64);

        if self.buffer_len + bytes.len() < 32 {
            self.buffer[self.buffer_len..self.buffer_len + bytes.len()].copy_from_slice(bytes);
            self.buffer_len += bytes.len();
            return;
        }

        let mut bytes = bytes;

        if self.buffer_len != 0 {
            let needed = 32 - self.buffer_len;
            self.buffer[self.buffer_len..].copy_from_slice(&bytes[..needed]);
            let chunk = self.buffer;
            self.update(&chunk);
            self.buffer_len = 0;
            bytes = &bytes[needed..];
        }

        let mut chunks = bytes.chunks_exact(32);
        for chunk in &mut chunks {
            self.update(chunk);
        }

        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            self.buffer[..remainder.len()].copy_from_slice(remainder);
            self.buffer_len = remainder.len();
        }
    }
}

#[inline]
fn round(mut acc: u64, input: u64) -> u64 {
    acc = acc.wrapping_add(input.wrapping_mul(P2));
    acc = acc.rotate_left(31);
    acc.wrapping_mul(P1)
}

#[inline]
fn merge_round(mut acc: u64, val: u64) -> u64 {
    let mut v = val;
    v = v.wrapping_mul(P2);
    v = v.rotate_left(31);
    v = v.wrapping_mul(P1);
    acc ^= v;
    acc.wrapping_mul(P1).wrapping_add(P4)
}

#[inline]
fn finalize(mut hash: u64) -> u64 {
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(P2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(P3);
    hash ^ (hash >> 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIME32: u64 = 0x9E3779B1;
    const PRIME64: u64 = 0x9E3779B185EBCA8D;

    fn fill_test_buffer(len: usize) -> Vec<u8> {
        let mut buffer = vec![0u8; len];
        let mut byte_gen = PRIME32;
        for byte in &mut buffer {
            *byte = (byte_gen >> 56) as u8;
            byte_gen = byte_gen.wrapping_mul(PRIME64);
        }
        buffer
    }

    fn xxhash64(data: &[u8], seed: u64) -> u64 {
        let mut hasher = XxHash64::with_seed(seed);
        hasher.write(data);
        hasher.finish64()
    }

    #[test]
    fn test_vectors_seed_zero() {
        let buf = fill_test_buffer(101);
        assert_eq!(xxhash64(&buf[..0], 0), 0xEF46DB3751D8E999);
        assert_eq!(xxhash64(&buf[..1], 0), 0xE934A84ADB052768);
        assert_eq!(xxhash64(&buf[..32], 0), 0x18B216492BB44B70);
        assert_eq!(xxhash64(&buf[..33], 0), 0x55C8DC3E578F5B59);
        assert_eq!(xxhash64(&buf[..100], 0), 0x4BFE019CD91D9EA4);
    }

    #[test]
    fn test_vectors_seed_prime32() {
        let buf = fill_test_buffer(101);
        assert_eq!(xxhash64(&buf[..0], PRIME32), 0xAC75FDA2929B17EF);
        assert_eq!(xxhash64(&buf[..1], PRIME32), 0x5014607643A9B4C3);
        assert_eq!(xxhash64(&buf[..32], PRIME32), 0xB3F33BDF93ADE409);
        assert_eq!(xxhash64(&buf[..100], PRIME32), 0x4853706DC9625CAE);
    }

    #[test]
    fn test_long_check() {
        let seed = 0;
        let hash1 = XxHash64::hash_u64(123, seed);
        let mut hasher = XxHash64::with_seed(seed);
        hasher.write(&123u64.to_le_bytes());
        let hash2 = hasher.finish64();
        assert_eq!(hash2, hash1);
    }
}
