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
use std::mem::size_of;

use crate::codec::SketchBytes;
use crate::codec::SketchSlice;
use crate::countmin::serialization::COUNTMIN_FAMILY_ID;
use crate::countmin::serialization::FLAGS_IS_EMPTY;
use crate::countmin::serialization::LONG_SIZE_BYTES;
use crate::countmin::serialization::PREAMBLE_LONGS_SHORT;
use crate::countmin::serialization::SERIAL_VERSION;
use crate::countmin::serialization::compute_seed_hash;
use crate::error::Error;
use crate::hash::DEFAULT_UPDATE_SEED;
use crate::hash::MurmurHash3X64128;

const MAX_TABLE_ENTRIES: usize = 1 << 30;

/// Count-Min sketch for estimating item frequencies.
///
/// The sketch provides upper and lower bounds on estimated item frequencies
/// with configurable relative error and confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct CountMinSketch {
    num_hashes: u8,
    num_buckets: u32,
    seed: u64,
    total_weight: i64,
    counts: Vec<i64>,
    hash_seeds: Vec<u64>,
}

impl CountMinSketch {
    /// Creates a new Count-Min sketch with the default seed.
    ///
    /// # Panics
    ///
    /// Panics if `num_hashes` is 0, `num_buckets` is less than 3, or the
    /// total table size exceeds the supported limit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// let sketch = CountMinSketch::new(4, 128);
    /// assert_eq!(sketch.num_buckets(), 128);
    /// ```
    pub fn new(num_hashes: u8, num_buckets: u32) -> Self {
        Self::with_seed(num_hashes, num_buckets, DEFAULT_UPDATE_SEED)
    }

    /// Creates a new Count-Min sketch with the provided seed.
    ///
    /// # Panics
    ///
    /// Panics if `num_hashes` is 0, `num_buckets` is less than 3, or the
    /// total table size exceeds the supported limit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// let sketch = CountMinSketch::with_seed(4, 64, 42);
    /// assert_eq!(sketch.seed(), 42);
    /// ```
    pub fn with_seed(num_hashes: u8, num_buckets: u32, seed: u64) -> Self {
        let entries = entries_for_config(num_hashes, num_buckets);
        Self::make(num_hashes, num_buckets, seed, entries)
    }

    /// Returns the number of hash functions used by the sketch.
    pub fn num_hashes(&self) -> u8 {
        self.num_hashes
    }

    /// Returns the number of buckets per hash function.
    pub fn num_buckets(&self) -> u32 {
        self.num_buckets
    }

    /// Returns the seed used by the sketch.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Returns the total weight inserted into the sketch.
    pub fn total_weight(&self) -> i64 {
        self.total_weight
    }

    /// Returns the relative error (epsilon) implied by the number of buckets.
    pub fn relative_error(&self) -> f64 {
        std::f64::consts::E / self.num_buckets as f64
    }

    /// Returns true if the sketch has not seen any updates.
    pub fn is_empty(&self) -> bool {
        self.total_weight == 0
    }

    /// Suggests the number of buckets to achieve the given relative error.
    ///
    /// # Panics
    ///
    /// Panics if `relative_error` is negative.
    pub fn suggest_num_buckets(relative_error: f64) -> u32 {
        assert!(relative_error >= 0.0, "relative_error must be at least 0");
        (std::f64::consts::E / relative_error).ceil() as u32
    }

    /// Suggests the number of hashes to achieve the given confidence.
    ///
    /// # Panics
    ///
    /// Panics if `confidence` is not in (0, 1].
    pub fn suggest_num_hashes(confidence: f64) -> u8 {
        assert!(
            (0.0..=1.0).contains(&confidence),
            "confidence must be between 0 and 1.0 (inclusive)"
        );
        if confidence == 1.0 {
            return 127;
        }
        let hashes = (1.0 / (1.0 - confidence)).ln().ceil();
        hashes.min(127.0) as u8
    }

    /// Updates the sketch with a single occurrence of the item.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// let mut sketch = CountMinSketch::new(4, 128);
    /// sketch.update("apple");
    /// assert!(sketch.estimate("apple") >= 1);
    /// ```
    pub fn update<T: Hash>(&mut self, item: T) {
        self.update_with_weight(item, 1);
    }

    /// Updates the sketch with the given item and weight.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// let mut sketch = CountMinSketch::new(4, 128);
    /// sketch.update_with_weight("banana", 3);
    /// assert!(sketch.estimate("banana") >= 3);
    /// ```
    pub fn update_with_weight<T: Hash>(&mut self, item: T, weight: i64) {
        if weight == 0 {
            return;
        }
        let abs_weight = abs_i64(weight);
        self.total_weight = self.total_weight.wrapping_add(abs_weight);
        let num_buckets = self.num_buckets as usize;
        for (row, seed) in self.hash_seeds.iter().enumerate() {
            let bucket = self.bucket_index(&item, *seed);
            let index = row * num_buckets + bucket;
            self.counts[index] = self.counts[index].wrapping_add(weight);
        }
    }

    /// Returns the estimated frequency of the given item.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// let mut sketch = CountMinSketch::new(4, 128);
    /// sketch.update_with_weight("pear", 2);
    /// assert!(sketch.estimate("pear") >= 2);
    /// ```
    pub fn estimate<T: Hash>(&self, item: T) -> i64 {
        let num_buckets = self.num_buckets as usize;
        let mut min = i64::MAX;
        for (row, seed) in self.hash_seeds.iter().enumerate() {
            let bucket = self.bucket_index(&item, *seed);
            let index = row * num_buckets + bucket;
            let value = self.counts[index];
            if value < min {
                min = value;
            }
        }
        min
    }

    /// Returns the lower bound on the true frequency of the given item.
    pub fn lower_bound<T: Hash>(&self, item: T) -> i64 {
        self.estimate(item)
    }

    /// Returns the upper bound on the true frequency of the given item.
    pub fn upper_bound<T: Hash>(&self, item: T) -> i64 {
        let estimate = self.estimate(item);
        let error = (self.relative_error() * self.total_weight as f64) as i64;
        estimate.wrapping_add(error)
    }

    /// Merges another sketch into this one.
    ///
    /// # Panics
    ///
    /// Panics if the sketches have incompatible configurations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// let mut left = CountMinSketch::new(4, 128);
    /// let mut right = CountMinSketch::new(4, 128);
    ///
    /// left.update("apple");
    /// right.update_with_weight("banana", 2);
    ///
    /// left.merge(&right);
    /// assert!(left.estimate("banana") >= 2);
    /// ```
    pub fn merge(&mut self, other: &CountMinSketch) {
        if std::ptr::eq(self, other) {
            panic!("Cannot merge a sketch with itself.");
        }
        if self.num_hashes != other.num_hashes
            || self.num_buckets != other.num_buckets
            || self.seed != other.seed
        {
            panic!("Incompatible sketch configuration.");
        }
        for (dst, src) in self.counts.iter_mut().zip(other.counts.iter()) {
            *dst = dst.wrapping_add(*src);
        }
        self.total_weight = self.total_weight.wrapping_add(other.total_weight);
    }

    /// Serializes this sketch into the DataSketches Count-Min format.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// # let mut sketch = CountMinSketch::new(4, 128);
    /// # sketch.update("apple");
    /// let bytes = sketch.serialize();
    /// let decoded = CountMinSketch::deserialize(&bytes).unwrap();
    /// assert!(decoded.estimate("apple") >= 1);
    /// ```
    pub fn serialize(&self) -> Vec<u8> {
        let header_size = PREAMBLE_LONGS_SHORT as usize * LONG_SIZE_BYTES;
        let payload_size = if self.is_empty() {
            0
        } else {
            LONG_SIZE_BYTES + (self.counts.len() * size_of::<i64>())
        };
        let mut bytes = SketchBytes::with_capacity(header_size + payload_size);

        bytes.write_u8(PREAMBLE_LONGS_SHORT);
        bytes.write_u8(SERIAL_VERSION);
        bytes.write_u8(COUNTMIN_FAMILY_ID);
        bytes.write_u8(if self.is_empty() { FLAGS_IS_EMPTY } else { 0 });
        bytes.write_u32_le(0); // unused

        bytes.write_u32_le(self.num_buckets);
        bytes.write_u8(self.num_hashes);
        bytes.write_u16_le(compute_seed_hash(self.seed));
        bytes.write_u8(0);

        if self.is_empty() {
            return bytes.into_bytes();
        }

        bytes.write_i64_le(self.total_weight);
        for count in self.counts.iter().copied() {
            bytes.write_i64_le(count);
        }
        bytes.into_bytes()
    }

    /// Deserializes a sketch from bytes using the default seed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// # let mut sketch = CountMinSketch::new(4, 64);
    /// # sketch.update("apple");
    /// # let bytes = sketch.serialize();
    /// let decoded = CountMinSketch::deserialize(&bytes).unwrap();
    /// assert!(decoded.estimate("apple") >= 1);
    /// ```
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        Self::deserialize_with_seed(bytes, DEFAULT_UPDATE_SEED)
    }

    /// Deserializes a sketch from bytes using the provided seed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use datasketches::countmin::CountMinSketch;
    /// # let mut sketch = CountMinSketch::with_seed(4, 64, 7);
    /// # sketch.update("apple");
    /// # let bytes = sketch.serialize();
    /// let decoded = CountMinSketch::deserialize_with_seed(&bytes, 7).unwrap();
    /// assert!(decoded.estimate("apple") >= 1);
    /// ```
    pub fn deserialize_with_seed(bytes: &[u8], seed: u64) -> Result<Self, Error> {
        fn make_error(tag: &'static str) -> impl FnOnce(std::io::Error) -> Error {
            move |_| Error::insufficient_data(tag)
        }

        let mut cursor = SketchSlice::new(bytes);
        let preamble_longs = cursor.read_u8().map_err(make_error("preamble_longs"))?;
        let serial_version = cursor.read_u8().map_err(make_error("serial_version"))?;
        let family_id = cursor.read_u8().map_err(make_error("family_id"))?;
        let flags = cursor.read_u8().map_err(make_error("flags"))?;
        cursor.read_u32_le().map_err(make_error("<unused>"))?;

        if family_id != COUNTMIN_FAMILY_ID {
            return Err(Error::invalid_family(
                COUNTMIN_FAMILY_ID,
                family_id,
                "CountMinSketch",
            ));
        }
        if serial_version != SERIAL_VERSION {
            return Err(Error::unsupported_serial_version(
                SERIAL_VERSION,
                serial_version,
            ));
        }
        if preamble_longs != PREAMBLE_LONGS_SHORT {
            return Err(Error::invalid_preamble_longs(
                PREAMBLE_LONGS_SHORT,
                preamble_longs,
            ));
        }

        let num_buckets = cursor.read_u32_le().map_err(make_error("num_buckets"))?;
        let num_hashes = cursor.read_u8().map_err(make_error("num_hashes"))?;
        let seed_hash = cursor.read_u16_le().map_err(make_error("seed_hash"))?;
        cursor.read_u8().map_err(make_error("unused8"))?;

        let expected_seed_hash = compute_seed_hash(seed);
        if seed_hash != expected_seed_hash {
            return Err(Error::deserial(format!(
                "incompatible seed hash: expected {expected_seed_hash}, got {seed_hash}",
            )));
        }

        let entries = entries_for_config_checked(num_hashes, num_buckets)?;
        let mut sketch = Self::make(num_hashes, num_buckets, seed, entries);
        if (flags & FLAGS_IS_EMPTY) != 0 {
            return Ok(sketch);
        }

        sketch.total_weight = cursor.read_i64_le().map_err(make_error("total_weight"))?;
        for count in sketch.counts.iter_mut() {
            *count = cursor.read_i64_le().map_err(make_error("counts"))?;
        }
        Ok(sketch)
    }

    fn make(num_hashes: u8, num_buckets: u32, seed: u64, entries: usize) -> Self {
        let counts = vec![0i64; entries];
        let hash_seeds = make_hash_seeds(seed, num_hashes);
        CountMinSketch {
            num_hashes,
            num_buckets,
            seed,
            total_weight: 0,
            counts,
            hash_seeds,
        }
    }

    fn bucket_index<T: Hash>(&self, item: &T, seed: u64) -> usize {
        let mut hasher = MurmurHash3X64128::with_seed(seed);
        item.hash(&mut hasher);
        let (h1, _) = hasher.finish128();
        (h1 % self.num_buckets as u64) as usize
    }
}

fn entries_for_config(num_hashes: u8, num_buckets: u32) -> usize {
    assert!(num_hashes > 0, "num_hashes must be at least 1");
    assert!(num_buckets >= 3, "num_buckets must be at least 3");
    let entries = (num_hashes as usize)
        .checked_mul(num_buckets as usize)
        .expect("num_hashes * num_buckets overflows usize");
    assert!(
        entries < MAX_TABLE_ENTRIES,
        "num_hashes * num_buckets must be < {}",
        MAX_TABLE_ENTRIES
    );
    entries
}

fn entries_for_config_checked(num_hashes: u8, num_buckets: u32) -> Result<usize, Error> {
    if num_hashes == 0 {
        return Err(Error::deserial("num_hashes must be at least 1"));
    }
    if num_buckets < 3 {
        return Err(Error::deserial("num_buckets must be at least 3"));
    }
    let entries = (num_hashes as usize)
        .checked_mul(num_buckets as usize)
        .ok_or_else(|| Error::deserial("num_hashes * num_buckets overflows usize"))?;
    if entries >= MAX_TABLE_ENTRIES {
        return Err(Error::deserial(format!(
            "num_hashes * num_buckets must be < {MAX_TABLE_ENTRIES}",
        )));
    }
    Ok(entries)
}

fn make_hash_seeds(seed: u64, num_hashes: u8) -> Vec<u64> {
    let mut seeds = Vec::with_capacity(num_hashes as usize);
    for i in 0..num_hashes {
        // Derive per-row hash seeds deterministically from the sketch seed.
        let mut hasher = MurmurHash3X64128::with_seed(seed);
        hasher.write(&u64::from(i).to_le_bytes());
        let (h1, _) = hasher.finish128();
        seeds.push(h1);
    }
    seeds
}

fn abs_i64(value: i64) -> i64 {
    if value >= 0 {
        value
    } else {
        value.wrapping_neg()
    }
}
