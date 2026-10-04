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

//! Frequent items sketch implementations.

use std::hash::Hash;

use crate::codec::SketchBytes;
use crate::codec::SketchSlice;
use crate::error::Error;
use crate::frequencies::reverse_purge_item_hash_map::ReversePurgeItemHashMap;
use crate::frequencies::serialization::*;

type CountSerializeSize<T> = fn(&[T]) -> usize;
type SerializeItems<T> = fn(&mut SketchBytes, &[T]);
type DeserializeItems<T> = fn(SketchSlice<'_>, usize) -> Result<Vec<T>, Error>;

const LG_MIN_MAP_SIZE: u8 = 3;
const SAMPLE_SIZE: usize = 1024;
const EPSILON_FACTOR: f64 = 3.5;
const LOAD_FACTOR_NUMERATOR: usize = 3;
const LOAD_FACTOR_DENOMINATOR: usize = 4;

/// Error guarantees for frequent item queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// Include items if upper bound exceeds threshold (no false negatives).
    NoFalseNegatives,
    /// Include items if lower bound exceeds threshold (no false positives).
    NoFalsePositives,
}

/// Result row for frequent item queries.
///
/// Each row includes an estimate and upper and lower bounds on the true frequency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row<T> {
    item: T,
    estimate: u64,
    upper_bound: u64,
    lower_bound: u64,
}

impl<T> Row<T> {
    /// Returns the item value.
    pub fn item(&self) -> &T {
        &self.item
    }

    /// Returns the estimated frequency.
    pub fn estimate(&self) -> u64 {
        self.estimate
    }

    /// Returns the upper bound for the frequency.
    pub fn upper_bound(&self) -> u64 {
        self.upper_bound
    }

    /// Returns the guaranteed lower bound for the frequency.
    ///
    /// This value is never negative.
    pub fn lower_bound(&self) -> u64 {
        self.lower_bound
    }
}

/// Frequent items sketch for generic item types.
///
/// The sketch tracks approximate item frequencies and can return estimates with
/// guaranteed upper and lower bounds.
///
/// See [`crate::frequencies`] for an overview and error guarantees.
#[derive(Debug, Clone)]
pub struct FrequentItemsSketch<T> {
    lg_max_map_size: u8,
    cur_map_cap: usize,
    offset: u64,
    stream_weight: u64,
    sample_size: usize,
    hash_map: ReversePurgeItemHashMap<T>,
}

impl<T: Eq + Hash> FrequentItemsSketch<T> {
    /// Creates a new sketch with the given maximum map size (power of two).
    ///
    /// The maximum map capacity is `0.75 * max_map_size`, and the internal map grows
    /// from a small starting size up to the maximum as needed.
    ///
    /// # Panics
    ///
    /// Panics if `max_map_size` is not a power of two.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// let mut sketch = FrequentItemsSketch::<i64>::new(64);
    /// sketch.update(1);
    /// sketch.update(2);
    /// assert_eq!(sketch.num_active_items(), 2);
    /// ```
    pub fn new(max_map_size: usize) -> Self {
        let lg_max_map_size = exact_log2(max_map_size);
        Self::with_lg_map_sizes(lg_max_map_size, LG_MIN_MAP_SIZE)
    }

    /// Returns true if the sketch is empty.
    pub fn is_empty(&self) -> bool {
        self.hash_map.num_active() == 0
    }

    /// Returns the number of active items being tracked.
    pub fn num_active_items(&self) -> usize {
        self.hash_map.num_active()
    }

    /// Returns the total weight of the stream.
    ///
    /// This is the sum of all counts passed to `update` and `update_with_count`.
    pub fn total_weight(&self) -> u64 {
        self.stream_weight
    }

    /// Returns the estimated frequency for an item.
    ///
    /// If the item is tracked, this is `item_count + offset`. Otherwise it is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// let mut sketch = FrequentItemsSketch::<i64>::new(64);
    /// sketch.update_with_count(10, 2);
    /// assert!(sketch.estimate(&10) >= 2);
    /// ```
    pub fn estimate(&self, item: &T) -> u64 {
        let value = self.hash_map.get(item);
        if value > 0 { value + self.offset } else { 0 }
    }

    /// Returns the guaranteed lower bound frequency for an item.
    ///
    /// This value is never negative and is guaranteed to be no larger than the true frequency.
    /// If the item is not tracked, the lower bound is zero.
    pub fn lower_bound(&self, item: &T) -> u64 {
        self.hash_map.get(item)
    }

    /// Returns the guaranteed upper bound frequency for an item.
    ///
    /// This value is guaranteed to be no smaller than the true frequency.
    /// If the item is tracked, this is `item_count + offset`.
    pub fn upper_bound(&self, item: &T) -> u64 {
        self.hash_map.get(item) + self.offset
    }

    /// Returns an upper bound on the maximum error of [`FrequentItemsSketch::estimate`]
    /// for any item.
    ///
    /// This is equivalent to the maximum distance between the upper bound and the lower bound
    /// for any item.
    pub fn maximum_error(&self) -> u64 {
        self.offset
    }

    /// Returns epsilon for this sketch.
    pub fn epsilon(&self) -> f64 {
        Self::epsilon_for_lg(self.lg_max_map_size)
    }

    /// Returns epsilon for a sketch configured with `lg_max_map_size`.
    pub fn epsilon_for_lg(lg_max_map_size: u8) -> f64 {
        EPSILON_FACTOR / (1u64 << lg_max_map_size) as f64
    }

    /// Returns the a priori error estimate.
    pub fn apriori_error(lg_max_map_size: u8, estimated_total_weight: i64) -> f64 {
        Self::epsilon_for_lg(lg_max_map_size) * estimated_total_weight as f64
    }

    /// Returns the maximum map capacity for this sketch.
    ///
    /// This is `0.75 * max_map_size`.
    pub fn maximum_map_capacity(&self) -> usize {
        (1usize << self.lg_max_map_size) * LOAD_FACTOR_NUMERATOR / LOAD_FACTOR_DENOMINATOR
    }

    /// Returns the current map capacity.
    ///
    /// This is the number of counters supported before resizing or purging.
    pub fn current_map_capacity(&self) -> usize {
        self.cur_map_cap
    }

    /// Returns the configured log2 maximum map size.
    pub fn lg_max_map_size(&self) -> u8 {
        self.lg_max_map_size
    }

    /// Returns the current map size in log2.
    pub fn lg_cur_map_size(&self) -> u8 {
        self.hash_map.lg_length()
    }

    /// Updates the sketch with a count of one.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// let mut sketch = FrequentItemsSketch::<i64>::new(64);
    /// sketch.update(42);
    /// assert!(sketch.estimate(&42) >= 1);
    /// ```
    pub fn update(&mut self, item: T) {
        self.update_with_count(item, 1);
    }

    /// Updates the sketch with an item and count.
    ///
    /// A count of zero is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// let mut sketch = FrequentItemsSketch::<i64>::new(64);
    /// sketch.update_with_count(10, 3);
    /// assert!(sketch.estimate(&10) >= 3);
    /// ```
    pub fn update_with_count(&mut self, item: T, count: u64) {
        if count == 0 {
            return;
        }
        assert!(count > 0, "count may not be negative");
        self.stream_weight += count;
        self.hash_map.adjust_or_put_value(item, count);
        self.maybe_resize_or_purge();
    }

    /// Merges another sketch into this one.
    ///
    /// The other sketch may have a different map size. The merged sketch respects the
    /// larger error tolerance of the inputs.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// let mut left = FrequentItemsSketch::<i64>::new(64);
    /// let mut right = FrequentItemsSketch::<i64>::new(64);
    /// left.update(1);
    /// right.update_with_count(2, 2);
    /// left.merge(&right);
    /// assert!(left.estimate(&2) >= 2);
    /// ```
    pub fn merge(&mut self, other: &Self)
    where
        T: Clone,
    {
        if other.is_empty() {
            return;
        }
        let merged_total = self.stream_weight + other.stream_weight;
        for (item, count) in other.hash_map.iter() {
            self.update_with_count(item.clone(), count);
        }
        self.offset += other.offset;
        self.stream_weight = merged_total;
    }

    /// Resets the sketch to an empty state.
    pub fn reset(&mut self) {
        *self = Self::with_lg_map_sizes(self.lg_max_map_size, LG_MIN_MAP_SIZE);
    }

    /// Returns frequent items using the sketch maximum error as threshold.
    ///
    /// This is equivalent to `frequent_items_with_threshold(self.maximum_error(), error_type)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::ErrorType;
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// let mut sketch = FrequentItemsSketch::<i64>::new(64);
    /// sketch.update_with_count(1, 5);
    /// sketch.update(2);
    /// let rows = sketch.frequent_items(ErrorType::NoFalseNegatives);
    /// assert!(rows.iter().any(|row| *row.item() == 1));
    /// ```
    pub fn frequent_items(&self, error_type: ErrorType) -> Vec<Row<T>>
    where
        T: Clone,
    {
        self.frequent_items_with_threshold(error_type, self.offset)
    }

    /// Returns frequent items using a custom threshold.
    ///
    /// If `threshold` is less than `maximum_error`, `maximum_error` is used instead.
    ///
    /// For [`ErrorType::NoFalseNegatives`], items are included when `upper_bound > threshold`.
    /// For [`ErrorType::NoFalsePositives`], items are included when `lower_bound > threshold`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::ErrorType;
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// let mut sketch = FrequentItemsSketch::<i64>::new(64);
    /// sketch.update_with_count(1, 5);
    /// sketch.update(2);
    /// let rows = sketch.frequent_items_with_threshold(ErrorType::NoFalsePositives, 3);
    /// assert!(rows.iter().any(|row| *row.item() == 1));
    /// ```
    pub fn frequent_items_with_threshold(
        &self,
        error_type: ErrorType,
        threshold: u64,
    ) -> Vec<Row<T>>
    where
        T: Clone,
    {
        let threshold = threshold.max(self.offset);
        let mut rows = Vec::new();
        for (item, count) in self.hash_map.iter() {
            let lower = count;
            let upper = count + self.offset;
            let include = match error_type {
                ErrorType::NoFalseNegatives => upper > threshold,
                ErrorType::NoFalsePositives => lower > threshold,
            };
            if include {
                rows.push(Row {
                    item: item.clone(),
                    estimate: upper,
                    upper_bound: upper,
                    lower_bound: lower,
                });
            }
        }
        rows.sort_by(|a, b| b.estimate.cmp(&a.estimate));
        rows
    }

    fn maybe_resize_or_purge(&mut self) {
        if self.hash_map.num_active() > self.cur_map_cap {
            if self.hash_map.lg_length() < self.lg_max_map_size {
                self.hash_map.resize(self.hash_map.len() * 2);
                self.cur_map_cap = self.hash_map.capacity();
            } else {
                let delta = self.hash_map.purge(self.sample_size);
                self.offset += delta;
                if self.hash_map.num_active() > self.maximum_map_capacity() {
                    panic!("purge did not reduce number of active items");
                }
            }
        }
    }

    fn with_lg_map_sizes(lg_max_map_size: u8, lg_cur_map_size: u8) -> Self {
        let lg_max = lg_max_map_size.max(LG_MIN_MAP_SIZE);
        let lg_cur = lg_cur_map_size.max(LG_MIN_MAP_SIZE);
        assert!(
            lg_cur <= lg_max,
            "lg_cur_map_size must not exceed lg_max_map_size"
        );
        let map = ReversePurgeItemHashMap::new(1usize << lg_cur);
        let cur_map_cap = map.capacity();
        let max_map_cap = (1usize << lg_max) * LOAD_FACTOR_NUMERATOR / LOAD_FACTOR_DENOMINATOR;
        let sample_size = SAMPLE_SIZE.min(max_map_cap);
        Self {
            lg_max_map_size: lg_max,
            cur_map_cap,
            offset: 0,
            stream_weight: 0,
            sample_size,
            hash_map: map,
        }
    }

    fn serialize_inner(
        &self,
        count_serialize_size: CountSerializeSize<T>,
        serialize_items: SerializeItems<T>,
    ) -> Vec<u8>
    where
        T: Clone, // for self.hash_map.active_keys()
    {
        if self.is_empty() {
            let mut bytes = SketchBytes::with_capacity(8);
            bytes.write_u8(PREAMBLE_LONGS_EMPTY);
            bytes.write_u8(SERIAL_VERSION);
            bytes.write_u8(FAMILY_ID);
            bytes.write_u8(self.lg_max_map_size);
            bytes.write_u8(self.hash_map.lg_length());
            bytes.write_u8(EMPTY_FLAG_MASK);
            return bytes.into_bytes();
        }

        let active_items = self.num_active_items();
        let values = self.hash_map.active_values();
        let keys = self.hash_map.active_keys();
        let total_bytes =
            PREAMBLE_LONGS_NONEMPTY as usize * 8 + (active_items * 8) + count_serialize_size(&keys);

        let mut bytes = SketchBytes::with_capacity(total_bytes);
        bytes.write_u8(PREAMBLE_LONGS_NONEMPTY);
        bytes.write_u8(SERIAL_VERSION);
        bytes.write_u8(FAMILY_ID);
        bytes.write_u8(self.lg_max_map_size);
        bytes.write_u8(self.hash_map.lg_length());
        bytes.write_u8(0); // flags
        bytes.write_u16_le(0); // unused

        bytes.write_u32_le(active_items as u32);
        bytes.write_u32_le(0); // unused
        bytes.write_u64_le(self.stream_weight);
        bytes.write_u64_le(self.offset);

        for value in values {
            bytes.write_u64_le(value);
        }
        serialize_items(&mut bytes, &keys);

        bytes.into_bytes()
    }

    fn deserialize_inner(
        bytes: &[u8],
        deserialize_items: DeserializeItems<T>,
    ) -> Result<Self, Error> {
        fn make_error(tag: &'static str) -> impl FnOnce(std::io::Error) -> Error {
            move |_| Error::insufficient_data(tag)
        }

        let mut cursor = SketchSlice::new(bytes);
        let pre_longs = cursor.read_u8().map_err(make_error("pre_longs"))?;
        let pre_longs = pre_longs & 0x3F;
        let serial_version = cursor.read_u8().map_err(make_error("serial_version"))?;
        let family = cursor.read_u8().map_err(make_error("family"))?;
        let lg_max = cursor.read_u8().map_err(make_error("lg_max_map_size"))?;
        let lg_cur = cursor.read_u8().map_err(make_error("lg_cur_map_size"))?;
        let flags = cursor.read_u8().map_err(make_error("flags"))?;
        cursor.read_u16_le().map_err(make_error("<unused>"))?;

        if serial_version != SERIAL_VERSION {
            return Err(Error::unsupported_serial_version(
                SERIAL_VERSION,
                serial_version,
            ));
        }

        if family != FAMILY_ID {
            return Err(Error::invalid_family(
                FAMILY_ID,
                family,
                "FrequentItemsSketch",
            ));
        }

        if lg_cur > lg_max {
            return Err(Error::deserial("lg_cur_map_size exceeds lg_max_map_size"));
        }

        let is_empty = (flags & EMPTY_FLAG_MASK) != 0;
        if is_empty {
            return if pre_longs != PREAMBLE_LONGS_EMPTY {
                Err(Error::invalid_preamble_longs(
                    PREAMBLE_LONGS_EMPTY,
                    pre_longs,
                ))
            } else {
                Ok(Self::with_lg_map_sizes(lg_max, lg_cur))
            };
        }

        if pre_longs != PREAMBLE_LONGS_NONEMPTY {
            return Err(Error::invalid_preamble_longs(
                PREAMBLE_LONGS_NONEMPTY,
                pre_longs,
            ));
        }

        let active_items = cursor.read_u32_le().map_err(make_error("active_items"))?;
        let active_items = active_items as usize;
        cursor.read_u32_le().map_err(make_error("<unused>"))?;
        let stream_weight = cursor.read_u64_le().map_err(make_error("stream_weight"))?;
        let offset_val = cursor.read_u64_le().map_err(make_error("offset"))?;

        let mut values = Vec::with_capacity(active_items);
        for i in 0..active_items {
            values.push(cursor.read_u64_le().map_err(|_| {
                Error::insufficient_data(format!(
                    "expected {active_items} weights, failed at index {i}"
                ))
            })?);
        }

        let items = deserialize_items(cursor, active_items)?;
        if items.len() != active_items {
            return Err(Error::deserial(
                "item count mismatch during deserialization",
            ));
        }

        let mut sketch = Self::with_lg_map_sizes(lg_max, lg_cur);
        for (item, value) in items.into_iter().zip(values) {
            sketch.update_with_count(item, value);
        }
        sketch.stream_weight = stream_weight;
        sketch.offset = offset_val;
        Ok(sketch)
    }
}

impl FrequentItemsSketch<i64> {
    /// Serializes this sketch into a byte vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// # let mut sketch = FrequentItemsSketch::<i64>::new(64);
    /// # sketch.update_with_count(7, 2);
    /// let bytes = sketch.serialize();
    /// let decoded = FrequentItemsSketch::<i64>::deserialize(&bytes).unwrap();
    /// assert!(decoded.estimate(&7) >= 2);
    /// ```
    pub fn serialize(&self) -> Vec<u8> {
        self.serialize_inner(count_i64_items_bytes, serialize_i64_items)
    }

    /// Deserializes a sketch from bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// # let mut sketch = FrequentItemsSketch::<i64>::new(64);
    /// # sketch.update_with_count(7, 2);
    /// # let bytes = sketch.serialize();
    /// let decoded = FrequentItemsSketch::<i64>::deserialize(&bytes).unwrap();
    /// assert!(decoded.estimate(&7) >= 2);
    /// ```
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        Self::deserialize_inner(bytes, deserialize_i64_items)
    }
}

impl FrequentItemsSketch<String> {
    /// Serializes this sketch into a byte vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// # let mut sketch = FrequentItemsSketch::<String>::new(64);
    /// # let apple = "apple".to_string();
    /// # sketch.update_with_count(apple.clone(), 2);
    /// let bytes = sketch.serialize();
    /// let decoded = FrequentItemsSketch::<String>::deserialize(&bytes).unwrap();
    /// assert!(decoded.estimate(&apple) >= 2);
    /// ```
    pub fn serialize(&self) -> Vec<u8> {
        self.serialize_inner(count_string_items_bytes, serialize_string_items)
    }

    /// Deserializes a sketch from bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::frequencies::FrequentItemsSketch;
    /// # let mut sketch = FrequentItemsSketch::<String>::new(64);
    /// # let apple = "apple".to_string();
    /// # sketch.update_with_count(apple.clone(), 2);
    /// # let bytes = sketch.serialize();
    /// let decoded = FrequentItemsSketch::<String>::deserialize(&bytes).unwrap();
    /// assert!(decoded.estimate(&apple) >= 2);
    /// ```
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        Self::deserialize_inner(bytes, deserialize_string_items)
    }
}

fn exact_log2(value: usize) -> u8 {
    assert!(value.is_power_of_two(), "value must be power of 2");
    value.trailing_zeros() as u8
}
