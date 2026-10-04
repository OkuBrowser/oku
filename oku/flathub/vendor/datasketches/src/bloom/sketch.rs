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

use std::hash::Hash;
use std::hash::Hasher;

use crate::codec::SketchBytes;
use crate::codec::SketchSlice;
use crate::error::Error;
use crate::hash::XxHash64;

// Serialization constants
const PREAMBLE_LONGS_EMPTY: u8 = 3;
const PREAMBLE_LONGS_STANDARD: u8 = 4;
const FAMILY_ID: u8 = 21; // Bloom filter family ID
const SERIAL_VERSION: u8 = 1;
const EMPTY_FLAG_MASK: u8 = 1 << 2;

/// A Bloom filter for probabilistic set membership testing.
///
/// Provides fast membership queries with:
/// - No false negatives (inserted items always return `true`)
/// - Tunable false positive rate
/// - Constant space usage
///
/// Use [`super::BloomFilterBuilder`] to construct instances.
#[derive(Debug, Clone, PartialEq)]
pub struct BloomFilter {
    /// Hash seed for all hash functions
    pub(super) seed: u64,
    /// Number of hash functions to use (k)
    pub(super) num_hashes: u16,
    /// Total number of bits in the filter (m)
    pub(super) capacity_bits: u64,
    /// Count of bits set to 1 (for statistics)
    pub(super) num_bits_set: u64,
    /// Bit array packed into u64 words
    /// Length = ceil(capacity_bits / 64)
    pub(super) bit_array: Vec<u64>,
}

impl BloomFilter {
    /// Tests whether an item is possibly in the set.
    ///
    /// Returns:
    /// - `true`: Item was **possibly** inserted (or false positive)
    /// - `false`: Item was **definitely not** inserted
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::BloomFilterBuilder;
    /// let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
    /// filter.insert("apple");
    ///
    /// assert!(filter.contains(&"apple")); // true - was inserted (probably)
    /// assert!(!filter.contains(&"grape")); // false - never inserted
    /// ```
    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        if self.is_empty() {
            return false;
        }

        let (h0, h1) = self.compute_hash(item);
        self.check_bits(h0, h1)
    }

    /// Tests and inserts an item in a single operation.
    ///
    /// Returns whether the item was possibly already in the set before insertion.
    /// This is more efficient than calling `contains()` then `insert()` separately.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::BloomFilterBuilder;
    /// let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
    ///
    /// let was_present = filter.contains_and_insert(&"apple");
    /// assert!(!was_present); // First insertion
    ///
    /// let was_present = filter.contains_and_insert(&"apple");
    /// assert!(was_present); // Now it's in the set
    /// ```
    pub fn contains_and_insert<T: Hash>(&mut self, item: &T) -> bool {
        let (h0, h1) = self.compute_hash(item);
        let was_present = self.check_bits(h0, h1);
        self.set_bits(h0, h1);
        was_present
    }

    /// Inserts an item into the filter.
    ///
    /// After insertion, `contains(item)` will always return `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::BloomFilterBuilder;
    /// let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
    ///
    /// filter.insert("apple");
    /// filter.insert(42_u64);
    /// filter.insert(&[1, 2, 3]);
    ///
    /// assert!(filter.contains(&"apple"));
    /// ```
    pub fn insert<T: Hash>(&mut self, item: T) {
        let (h0, h1) = self.compute_hash(&item);
        self.set_bits(h0, h1);
    }

    /// Resets the filter to its initial empty state.
    ///
    /// Clears all bits while preserving capacity and configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::BloomFilterBuilder;
    /// let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
    /// filter.insert("apple");
    /// assert!(!filter.is_empty());
    ///
    /// filter.reset();
    /// assert!(filter.is_empty());
    /// assert!(!filter.contains(&"apple"));
    /// ```
    pub fn reset(&mut self) {
        self.bit_array.fill(0);
        self.num_bits_set = 0
    }

    /// Merges another filter into this one via bitwise OR (union).
    ///
    /// After merging, this filter will recognize items from either filter
    /// (plus any false positives from either).
    ///
    /// # Panics
    ///
    /// Panics if the filters are not compatible (different size, hashes, or seed).
    /// Use [`is_compatible()`](Self::is_compatible) to check first.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::BloomFilterBuilder;
    /// let mut f1 = BloomFilterBuilder::with_accuracy(100, 0.01)
    ///     .seed(123)
    ///     .build();
    /// let mut f2 = BloomFilterBuilder::with_accuracy(100, 0.01)
    ///     .seed(123)
    ///     .build();
    ///
    /// f1.insert("a");
    /// f2.insert("b");
    ///
    /// f1.union(&f2);
    /// assert!(f1.contains(&"a"));
    /// assert!(f1.contains(&"b"));
    /// ```
    pub fn union(&mut self, other: &BloomFilter) {
        assert!(
            self.is_compatible(other),
            "Cannot union incompatible Bloom filters"
        );

        // Count bits during union operation (single pass)
        let mut num_bits_set = 0;
        for (word, other_word) in self.bit_array.iter_mut().zip(&other.bit_array) {
            *word |= *other_word;
            num_bits_set += word.count_ones() as u64;
        }
        self.num_bits_set = num_bits_set;
    }

    /// Intersects this filter with another via bitwise AND.
    ///
    /// After intersection, this filter will recognize only items present in both
    /// filters (plus false positives).
    ///
    /// # Panics
    ///
    /// Panics if the filters are not compatible (different size, hashes, or seed).
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::BloomFilterBuilder;
    /// let mut f1 = BloomFilterBuilder::with_accuracy(100, 0.01)
    ///     .seed(123)
    ///     .build();
    /// let mut f2 = BloomFilterBuilder::with_accuracy(100, 0.01)
    ///     .seed(123)
    ///     .build();
    ///
    /// f1.insert("a");
    /// f1.insert("b");
    /// f2.insert("b");
    /// f2.insert("c");
    ///
    /// f1.intersect(&f2);
    /// assert!(f1.contains(&"b")); // In both
    /// // "a" and "c" likely return false now
    /// ```
    pub fn intersect(&mut self, other: &BloomFilter) {
        assert!(
            self.is_compatible(other),
            "Cannot intersect incompatible Bloom filters"
        );

        // Count bits during intersect operation (single pass)
        let mut num_bits_set = 0;
        for (word, other_word) in self.bit_array.iter_mut().zip(&other.bit_array) {
            *word &= *other_word;
            num_bits_set += word.count_ones() as u64;
        }
        self.num_bits_set = num_bits_set;
    }

    /// Inverts all bits in the filter.
    ///
    /// This approximately inverts the notion of set membership, though the false
    /// positive guarantees no longer hold in a well-defined way.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::BloomFilterBuilder;
    /// let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
    /// filter.insert("apple");
    ///
    /// filter.invert();
    /// // Now "apple" probably returns false, and most other items return true
    /// ```
    pub fn invert(&mut self) {
        // Invert bits and count during operation (single pass)
        let mut num_bits_set = 0;
        for word in &mut self.bit_array {
            *word = !*word;
            num_bits_set += word.count_ones() as u64;
        }

        // Mask off excess bits in the last word
        let excess_bits = self.capacity_bits % 64;
        if excess_bits != 0 {
            let last_idx = self.bit_array.len() - 1;
            let old_count = self.bit_array[last_idx].count_ones() as u64;
            let mask = (1u64 << excess_bits) - 1;
            self.bit_array[last_idx] &= mask;
            let new_count = self.bit_array[last_idx].count_ones() as u64;
            // Adjust count for masked-off bits
            num_bits_set = num_bits_set - old_count + new_count;
        }

        self.num_bits_set = num_bits_set;
    }

    /// Returns whether the filter is empty (no items inserted).
    pub fn is_empty(&self) -> bool {
        self.num_bits_set == 0
    }

    /// Returns the number of bits set to 1.
    ///
    /// Useful for monitoring filter saturation.
    pub fn bits_used(&self) -> u64 {
        self.num_bits_set
    }

    /// Returns the total number of bits in the filter (capacity).
    pub fn capacity(&self) -> u64 {
        self.capacity_bits
    }

    /// Returns the number of hash functions used.
    pub fn num_hashes(&self) -> u16 {
        self.num_hashes
    }

    /// Returns the hash seed.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Returns the current load factor (fraction of bits set).
    ///
    /// Values near 0.5 indicate the filter is approaching saturation.
    /// Values above 0.5 indicate degraded false positive rates.
    pub fn load_factor(&self) -> f64 {
        self.num_bits_set as f64 / self.capacity_bits as f64
    }

    /// Estimates the current false positive probability.
    ///
    /// Uses the approximation: `load_factor^k`
    /// where:
    /// - load_factor = fraction of bits set (bits_used / capacity)
    /// - k = num_hashes
    ///
    /// This assumes uniform bit distribution and is more accurate than
    /// trying to estimate insertion count from the load factor.
    pub fn estimated_fpp(&self) -> f64 {
        let k = self.num_hashes as f64;
        let load = self.load_factor();

        // FPP ≈ load^k
        // This is the standard approximation when load factor is known directly
        load.powf(k)
    }

    /// Checks if two filters are compatible for merging.
    ///
    /// Filters are compatible if they have the same:
    /// - Capacity (number of bits)
    /// - Number of hash functions
    /// - Seed
    pub fn is_compatible(&self, other: &BloomFilter) -> bool {
        self.capacity_bits == other.capacity_bits
            && self.num_hashes == other.num_hashes
            && self.seed == other.seed
    }

    /// Serializes the filter to a byte vector.
    ///
    /// The format is compatible with other Apache DataSketches implementations.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::{BloomFilter, BloomFilterBuilder};
    /// let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
    /// filter.insert("test");
    ///
    /// let bytes = filter.serialize();
    /// let restored = BloomFilter::deserialize(&bytes).unwrap();
    /// assert!(restored.contains(&"test"));
    /// ```
    pub fn serialize(&self) -> Vec<u8> {
        let is_empty = self.is_empty();
        let preamble_longs = if is_empty {
            PREAMBLE_LONGS_EMPTY
        } else {
            PREAMBLE_LONGS_STANDARD
        };

        let capacity = 8 * preamble_longs as usize
            + if is_empty {
                0
            } else {
                self.bit_array.len() * 8
            };
        let mut bytes = SketchBytes::with_capacity(capacity);

        // Preamble
        bytes.write_u8(preamble_longs); // Byte 0
        bytes.write_u8(SERIAL_VERSION); // Byte 1
        bytes.write_u8(FAMILY_ID); // Byte 2
        bytes.write_u8(if is_empty { EMPTY_FLAG_MASK } else { 0 }); // Byte 3: flags
        bytes.write_u16_le(self.num_hashes); // Bytes 4-5
        bytes.write_u16_le(0); // Bytes 6-7: unused

        bytes.write_u64_le(self.seed);

        // Bit array capacity stored as number of 64-bit words (int32) + unused padding (uint32)
        let num_longs = (self.capacity_bits / 64) as i32;
        bytes.write_i32_le(num_longs);
        bytes.write_u32_le(0); // unused

        if !is_empty {
            bytes.write_u64_le(self.num_bits_set);

            // Bit array
            for &word in &self.bit_array {
                bytes.write_u64_le(word);
            }
        }

        bytes.into_bytes()
    }

    /// Deserializes a filter from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data is truncated or corrupted
    /// - The family ID doesn't match (not a Bloom filter)
    /// - The serial version is unsupported
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::bloom::{BloomFilter, BloomFilterBuilder};
    /// let original = BloomFilterBuilder::with_accuracy(100, 0.01).build();
    /// let bytes = original.serialize();
    ///
    /// let restored = BloomFilter::deserialize(&bytes).unwrap();
    /// assert_eq!(original, restored);
    /// ```
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        let mut cursor = SketchSlice::new(bytes);

        // Read preamble
        let preamble_longs = cursor
            .read_u8()
            .map_err(|_| Error::insufficient_data("preamble_longs"))?;
        let serial_version = cursor
            .read_u8()
            .map_err(|_| Error::insufficient_data("serial_version"))?;
        let family_id = cursor
            .read_u8()
            .map_err(|_| Error::insufficient_data("family_id"))?;

        // Byte 3: flags byte (directly after family_id)
        let flags = cursor
            .read_u8()
            .map_err(|_| Error::insufficient_data("flags"))?;

        // Validate
        if family_id != FAMILY_ID {
            return Err(Error::invalid_family(FAMILY_ID, family_id, "BloomFilter"));
        }
        if serial_version != SERIAL_VERSION {
            return Err(Error::unsupported_serial_version(
                SERIAL_VERSION,
                serial_version,
            ));
        }
        if preamble_longs != PREAMBLE_LONGS_EMPTY && preamble_longs != PREAMBLE_LONGS_STANDARD {
            return Err(Error::invalid_preamble_longs(
                PREAMBLE_LONGS_STANDARD,
                preamble_longs,
            ));
        }

        let is_empty = (flags & EMPTY_FLAG_MASK) != 0;

        // Bytes 4-5: num_hashes (u16)
        let num_hashes = cursor
            .read_u16_le()
            .map_err(|_| Error::insufficient_data("num_hashes"))?;
        // Bytes 6-7: unused (u16)
        let _unused = cursor
            .read_u16_le()
            .map_err(|_| Error::insufficient_data("unused_header"))?;
        let seed = cursor
            .read_u64_le()
            .map_err(|_| Error::insufficient_data("seed"))?;

        // Bit array capacity stored as number of 64-bit words (int32) + unused padding (uint32)
        let num_longs = cursor
            .read_i32_le()
            .map_err(|_| Error::insufficient_data("num_longs"))? as u64;
        let _unused = cursor
            .read_u32_le()
            .map_err(|_| Error::insufficient_data("unused"))?;

        let capacity_bits = num_longs * 64; // Convert longs to bits
        let num_words = num_longs as usize;
        let mut bit_array = vec![0u64; num_words];
        let num_bits_set;

        if is_empty {
            num_bits_set = 0;
        } else {
            let raw_num_bits_set = cursor
                .read_u64_le()
                .map_err(|_| Error::insufficient_data("num_bits_set"))?;

            for word in &mut bit_array {
                *word = cursor
                    .read_u64_le()
                    .map_err(|_| Error::insufficient_data("bit_array"))?;
            }

            // Handle "dirty" state: 0xFFFFFFFFFFFFFFFF indicates bits need recounting
            const DIRTY_BITS_VALUE: u64 = 0xFFFFFFFFFFFFFFFF;
            if raw_num_bits_set == DIRTY_BITS_VALUE {
                num_bits_set = bit_array.iter().map(|w| w.count_ones() as u64).sum();
            } else {
                num_bits_set = raw_num_bits_set;
            }
        }

        Ok(BloomFilter {
            seed,
            num_hashes,
            capacity_bits,
            num_bits_set,
            bit_array,
        })
    }

    /// Computes the two base hash values using XXHash64.
    ///
    /// Uses a two-hash approach:
    /// - h0 = XXHash64(item, seed)
    /// - h1 = XXHash64(item, h0)
    fn compute_hash<T: Hash>(&self, item: &T) -> (u64, u64) {
        // First hash with the configured seed
        let mut hasher = XxHash64::with_seed(self.seed);
        item.hash(&mut hasher);
        let h0 = hasher.finish();

        // Second hash using h0 as the seed
        let mut hasher = XxHash64::with_seed(h0);
        item.hash(&mut hasher);
        let h1 = hasher.finish();

        (h0, h1)
    }

    /// Checks if all k bits are set for the given hash values.
    fn check_bits(&self, h0: u64, h1: u64) -> bool {
        for i in 1..=self.num_hashes {
            let bit_index = self.compute_bit_index(h0, h1, i);
            if !self.get_bit(bit_index) {
                return false;
            }
        }
        true
    }

    /// Sets all k bits for the given hash values.
    fn set_bits(&mut self, h0: u64, h1: u64) {
        for i in 1..=self.num_hashes {
            let bit_index = self.compute_bit_index(h0, h1, i);
            self.set_bit(bit_index);
        }
    }

    /// Computes a bit index using double hashing (Kirsch-Mitzenmacher).
    ///
    /// Formula:
    /// ```text
    /// hash_index = ((h0 + i * h1) >> 1) % capacity_bits
    /// ```
    ///
    /// The right shift by 1 improves bit distribution. The index `i` is 1-based.
    fn compute_bit_index(&self, h0: u64, h1: u64, i: u16) -> u64 {
        let hash = h0.wrapping_add(u64::from(i).wrapping_mul(h1));
        (hash >> 1) % self.capacity_bits
    }

    /// Gets the value of a single bit.
    fn get_bit(&self, bit_index: u64) -> bool {
        let word_index = (bit_index >> 6) as usize; // Equivalent to bit_index / 64
        let bit_offset = bit_index & 63; // Equivalent to bit_index % 64
        let mask = 1u64 << bit_offset;
        (self.bit_array[word_index] & mask) != 0
    }

    /// Sets a single bit and updates the count if it wasn't already set.
    fn set_bit(&mut self, bit_index: u64) {
        let word_index = (bit_index >> 6) as usize; // Equivalent to bit_index / 64
        let bit_offset = bit_index & 63; // Equivalent to bit_index % 64
        let mask = 1u64 << bit_offset;

        if (self.bit_array[word_index] & mask) == 0 {
            self.bit_array[word_index] |= mask;
            self.num_bits_set += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BloomFilter;
    use crate::bloom::BloomFilterBuilder;

    #[test]
    fn test_builder_with_accuracy() {
        let filter = BloomFilterBuilder::with_accuracy(1000, 0.01).build();
        assert!(filter.capacity() >= 9000);
        assert_eq!(filter.num_hashes(), 7);
        assert!(filter.is_empty());
    }

    #[test]
    fn test_builder_with_size() {
        let filter = BloomFilterBuilder::with_size(1024, 5).build();
        assert_eq!(filter.capacity(), 1024);
        assert_eq!(filter.num_hashes(), 5);
    }

    #[test]
    fn test_insert_and_contains() {
        let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();

        assert!(!filter.contains(&"apple"));
        filter.insert("apple");
        assert!(filter.contains(&"apple"));
        assert!(!filter.is_empty());
    }

    #[test]
    fn test_contains_and_insert() {
        let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();

        let was_present = filter.contains_and_insert(&42_u64);
        assert!(!was_present);

        let was_present = filter.contains_and_insert(&42_u64);
        assert!(was_present);
    }

    #[test]
    fn test_reset() {
        let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
        filter.insert("test");
        assert!(!filter.is_empty());

        filter.reset();
        assert!(filter.is_empty());
        assert!(!filter.contains(&"test"));
    }

    #[test]
    fn test_union() {
        let mut f1 = BloomFilterBuilder::with_accuracy(100, 0.01)
            .seed(123)
            .build();
        let mut f2 = BloomFilterBuilder::with_accuracy(100, 0.01)
            .seed(123)
            .build();

        f1.insert("a");
        f2.insert("b");

        f1.union(&f2);
        assert!(f1.contains(&"a"));
        assert!(f1.contains(&"b"));
    }

    #[test]
    fn test_intersect() {
        let mut f1 = BloomFilterBuilder::with_accuracy(100, 0.01)
            .seed(123)
            .build();
        let mut f2 = BloomFilterBuilder::with_accuracy(100, 0.01)
            .seed(123)
            .build();

        f1.insert("a");
        f1.insert("b");
        f2.insert("b");
        f2.insert("c");

        f1.intersect(&f2);
        assert!(f1.contains(&"b"));
    }

    #[test]
    fn test_serialize_deserialize_empty() {
        let filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
        let bytes = filter.serialize();
        let restored = BloomFilter::deserialize(&bytes).unwrap();

        assert_eq!(filter, restored);
    }

    #[test]
    fn test_serialize_deserialize_with_data() {
        let mut filter = BloomFilterBuilder::with_accuracy(100, 0.01).build();
        filter.insert("test");
        filter.insert(42_u64);

        let bytes = filter.serialize();
        let restored = BloomFilter::deserialize(&bytes).unwrap();

        assert_eq!(filter, restored);
        assert!(restored.contains(&"test"));
        assert!(restored.contains(&42_u64));
    }

    #[test]
    fn test_statistics() {
        let mut filter = BloomFilterBuilder::with_size(1000, 5).build();
        assert_eq!(filter.bits_used(), 0);
        assert_eq!(filter.load_factor(), 0.0);

        filter.insert("test");
        assert!(filter.bits_used() > 0);
        assert!(filter.load_factor() > 0.0);
        assert!(filter.estimated_fpp() > 0.0);
    }

    #[test]
    fn test_is_compatible() {
        let f1 = BloomFilterBuilder::with_accuracy(100, 0.01)
            .seed(123)
            .build();
        let f2 = BloomFilterBuilder::with_accuracy(100, 0.01)
            .seed(123)
            .build();
        let f3 = BloomFilterBuilder::with_accuracy(100, 0.01)
            .seed(456)
            .build();

        assert!(f1.is_compatible(&f2));
        assert!(!f1.is_compatible(&f3));
    }

    #[test]
    #[should_panic(expected = "max_items must be greater than 0")]
    fn test_invalid_max_items() {
        BloomFilterBuilder::with_accuracy(0, 0.01);
    }

    #[test]
    #[should_panic(expected = "fpp must be between")]
    fn test_invalid_fpp() {
        BloomFilterBuilder::with_accuracy(100, 1.5);
    }
}
