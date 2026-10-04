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

use std::cmp::Ordering;
use std::convert::identity;
use std::num::NonZeroU64;

use crate::codec::SketchBytes;
use crate::codec::SketchSlice;
use crate::error::Error;
use crate::error::ErrorKind;
use crate::tdigest::serialization::*;

/// The default value of K if one is not specified.
const DEFAULT_K: u16 = 200;
/// Multiplier for buffer size relative to centroids capacity.
const BUFFER_MULTIPLIER: usize = 4;
/// Default weight for single values.
const DEFAULT_WEIGHT: NonZeroU64 = NonZeroU64::new(1).unwrap();

/// T-Digest sketch for estimating quantiles and ranks.
///
/// See the [tdigest module level documentation](crate::tdigest) for more.
#[derive(Debug, Clone)]
pub struct TDigestMut {
    k: u16,

    reverse_merge: bool,
    min: f64,
    max: f64,

    centroids: Vec<Centroid>,
    centroids_weight: u64,
    centroids_capacity: usize,
    buffer: Vec<f64>,
}

impl Default for TDigestMut {
    fn default() -> Self {
        TDigestMut::new(DEFAULT_K)
    }
}

impl TDigestMut {
    /// Creates a tdigest instance with the given value of k.
    ///
    /// The fallible version of this method is [`TDigestMut::try_new`].
    ///
    /// # Panics
    ///
    /// Panics if k is less than 10
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// let sketch = TDigestMut::new(100);
    /// assert_eq!(sketch.k(), 100);
    /// ```
    pub fn new(k: u16) -> Self {
        Self::make(
            k,
            false,
            f64::INFINITY,
            f64::NEG_INFINITY,
            vec![],
            0,
            vec![],
        )
    }

    /// Creates a tdigest instance with the given value of k.
    ///
    /// The panicking version of this method is [`TDigestMut::new`].
    ///
    /// # Errors
    ///
    /// If k is less than 10, returns [`ErrorKind::InvalidArgument`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// let sketch = TDigestMut::try_new(20).unwrap();
    /// assert_eq!(sketch.k(), 20);
    /// ```
    pub fn try_new(k: u16) -> Result<Self, Error> {
        if k < 10 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!("k must be at least 10, got {k}"),
            ));
        }

        Ok(Self::make(
            k,
            false,
            f64::INFINITY,
            f64::NEG_INFINITY,
            vec![],
            0,
            vec![],
        ))
    }

    // for deserialization
    fn make(
        k: u16,
        reverse_merge: bool,
        min: f64,
        max: f64,
        mut centroids: Vec<Centroid>,
        centroids_weight: u64,
        mut buffer: Vec<f64>,
    ) -> Self {
        assert!(k >= 10, "k must be at least 10");

        let fudge = if k < 30 { 30 } else { 10 };
        let centroids_capacity = (k as usize * 2) + fudge;

        centroids.reserve(centroids_capacity);
        buffer.reserve(centroids_capacity * BUFFER_MULTIPLIER);

        TDigestMut {
            k,
            reverse_merge,
            min,
            max,
            centroids,
            centroids_weight,
            centroids_capacity,
            buffer,
        }
    }

    /// Update this TDigest with the given value.
    ///
    /// [f64::NAN], [f64::INFINITY], and [f64::NEG_INFINITY] values are ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// let mut sketch = TDigestMut::new(100);
    /// sketch.update(1.0);
    /// assert!(sketch.total_weight() >= 1);
    /// ```
    pub fn update(&mut self, value: f64) {
        if value.is_nan() || value.is_infinite() {
            return;
        }

        if self.buffer.len() == self.centroids_capacity * BUFFER_MULTIPLIER {
            self.compress();
        }

        self.buffer.push(value);
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    /// Returns parameter k (compression) that was used to configure this TDigest.
    pub fn k(&self) -> u16 {
        self.k
    }

    /// Returns true if TDigest has not seen any data.
    pub fn is_empty(&self) -> bool {
        self.centroids.is_empty() && self.buffer.is_empty()
    }

    /// Returns minimum value seen by TDigest; `None` if TDigest is empty.
    pub fn min_value(&self) -> Option<f64> {
        if self.is_empty() {
            None
        } else {
            Some(self.min)
        }
    }

    /// Returns maximum value seen by TDigest; `None` if TDigest is empty.
    pub fn max_value(&self) -> Option<f64> {
        if self.is_empty() {
            None
        } else {
            Some(self.max)
        }
    }

    /// Returns total weight.
    pub fn total_weight(&self) -> u64 {
        self.centroids_weight + self.buffer.len() as u64
    }

    /// Merge the given TDigest into this one
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// let mut left = TDigestMut::new(100);
    /// let mut right = TDigestMut::new(100);
    /// left.update(1.0);
    /// right.update(2.0);
    /// left.merge(&right);
    /// assert_eq!(left.total_weight(), 2);
    /// ```
    pub fn merge(&mut self, other: &TDigestMut) {
        if other.is_empty() {
            return;
        }

        let mut tmp = Vec::with_capacity(
            self.centroids.len() + self.buffer.len() + other.centroids.len() + other.buffer.len(),
        );
        for &v in &self.buffer {
            tmp.push(Centroid {
                mean: v,
                weight: DEFAULT_WEIGHT,
            });
        }
        for &v in &other.buffer {
            tmp.push(Centroid {
                mean: v,
                weight: DEFAULT_WEIGHT,
            });
        }
        for &c in &other.centroids {
            tmp.push(c);
        }
        self.do_merge(tmp, self.buffer.len() as u64 + other.total_weight())
    }

    /// Freezes this TDigest into an immutable one.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// let mut sketch = TDigestMut::new(100);
    /// sketch.update(1.0);
    /// let frozen = sketch.freeze();
    /// assert!(!frozen.is_empty());
    /// ```
    pub fn freeze(mut self) -> TDigest {
        self.compress();
        TDigest {
            k: self.k,
            reverse_merge: self.reverse_merge,
            min: self.min,
            max: self.max,
            centroids: self.centroids,
            centroids_weight: self.centroids_weight,
        }
    }

    fn view(&mut self) -> TDigestView<'_> {
        self.compress(); // side effect
        TDigestView {
            min: self.min,
            max: self.max,
            centroids: &self.centroids,
            centroids_weight: self.centroids_weight,
        }
    }

    /// See [`TDigest::cdf`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # for value in [1.0, 2.0, 3.0] {
    /// #     sketch.update(value);
    /// # }
    /// let cdf = sketch.cdf(&[1.5]).unwrap();
    /// assert_eq!(cdf.len(), 2);
    /// ```
    pub fn cdf(&mut self, split_points: &[f64]) -> Option<Vec<f64>> {
        check_split_points(split_points);

        if self.is_empty() {
            return None;
        }

        self.view().cdf(split_points)
    }

    /// See [`TDigest::pmf`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # for value in [1.0, 2.0, 3.0] {
    /// #     sketch.update(value);
    /// # }
    /// let pmf = sketch.pmf(&[1.5]).unwrap();
    /// assert_eq!(pmf.len(), 2);
    /// ```
    pub fn pmf(&mut self, split_points: &[f64]) -> Option<Vec<f64>> {
        check_split_points(split_points);

        if self.is_empty() {
            return None;
        }

        self.view().pmf(split_points)
    }

    /// See [`TDigest::rank`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # for value in [1.0, 2.0, 3.0] {
    /// #     sketch.update(value);
    /// # }
    /// let rank = sketch.rank(2.0).unwrap();
    /// assert!((0.0..=1.0).contains(&rank));
    /// ```
    pub fn rank(&mut self, value: f64) -> Option<f64> {
        assert!(!value.is_nan(), "value must not be NaN");

        if self.is_empty() {
            return None;
        }
        if value < self.min {
            return Some(0.0);
        }
        if value > self.max {
            return Some(1.0);
        }
        // one centroid and value == min == max
        if self.centroids.len() + self.buffer.len() == 1 {
            return Some(0.5);
        }

        self.view().rank(value)
    }

    /// See [`TDigest::quantile`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # for value in [1.0, 2.0, 3.0] {
    /// #     sketch.update(value);
    /// # }
    /// let median = sketch.quantile(0.5).unwrap();
    /// assert!((1.0..=3.0).contains(&median));
    /// ```
    pub fn quantile(&mut self, rank: f64) -> Option<f64> {
        assert!((0.0..=1.0).contains(&rank), "rank must be in [0.0, 1.0]");

        if self.is_empty() {
            return None;
        }

        self.view().quantile(rank)
    }

    /// Serializes this TDigest to bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # sketch.update(1.0);
    /// let bytes = sketch.serialize();
    /// let decoded = TDigestMut::deserialize(&bytes, false).unwrap();
    /// assert_eq!(decoded.max_value(), Some(1.0));
    /// ```
    pub fn serialize(&mut self) -> Vec<u8> {
        self.compress();

        let mut total_size = 0;
        if self.is_empty() || self.is_single_value() {
            // 1 byte preamble
            // + 1 byte serial version
            // + 1 byte family
            // + 2 bytes k
            // + 1 byte flags
            // + 2 bytes unused
            total_size += size_of::<u64>();
        } else {
            // all of the above
            // + 4 bytes num centroids
            // + 4 bytes num buffered
            total_size += size_of::<u64>() * 2;
        }
        if self.is_empty() {
            // nothing more
        } else if self.is_single_value() {
            // + 8 bytes single value
            total_size += size_of::<f64>();
        } else {
            // + 8 bytes min
            // + 8 bytes max
            total_size += size_of::<f64>() * 2;
            // + (8+8) bytes per centroid
            total_size += self.centroids.len() * (size_of::<f64>() + size_of::<u64>());
        }

        let mut bytes = SketchBytes::with_capacity(total_size);
        bytes.write_u8(match self.total_weight() {
            0 => PREAMBLE_LONGS_EMPTY_OR_SINGLE,
            1 => PREAMBLE_LONGS_EMPTY_OR_SINGLE,
            _ => PREAMBLE_LONGS_MULTIPLE,
        });
        bytes.write_u8(SERIAL_VERSION);
        bytes.write_u8(TDIGEST_FAMILY_ID);
        bytes.write_u16_le(self.k);
        bytes.write_u8({
            let mut flags = 0;
            if self.is_empty() {
                flags |= FLAGS_IS_EMPTY;
            }
            if self.is_single_value() {
                flags |= FLAGS_IS_SINGLE_VALUE;
            }
            if self.reverse_merge {
                flags |= FLAGS_REVERSE_MERGE;
            }
            flags
        });
        bytes.write_u16_le(0); // unused
        if self.is_empty() {
            return bytes.into_bytes();
        }
        if self.is_single_value() {
            bytes.write_f64_le(self.min);
            return bytes.into_bytes();
        }
        bytes.write_u32_le(self.centroids.len() as u32);
        bytes.write_u32_le(0); // unused
        bytes.write_f64_le(self.min);
        bytes.write_f64_le(self.max);
        for centroid in &self.centroids {
            bytes.write_f64_le(centroid.mean);
            bytes.write_u64_le(centroid.weight.get());
        }
        bytes.into_bytes()
    }

    /// Deserializes a TDigest from bytes.
    ///
    /// Supports reading compact format with (float, int) centroids as opposed to (double, long) to
    /// represent (mean, weight). [^1]
    ///
    /// Supports reading format of the reference implementation (auto-detected) [^2].
    ///
    /// [^1]: This is to support reading the `tdigest<float>` format from the C++ implementation.
    /// [^2]: <https://github.com/tdunning/t-digest>
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # sketch.update(1.0);
    /// # sketch.update(2.0);
    /// # let bytes = sketch.serialize();
    /// let decoded = TDigestMut::deserialize(&bytes, false).unwrap();
    /// assert_eq!(decoded.max_value(), Some(2.0));
    /// ```
    pub fn deserialize(bytes: &[u8], is_f32: bool) -> Result<Self, Error> {
        fn make_error(tag: &'static str) -> impl FnOnce(std::io::Error) -> Error {
            move |_| Error::insufficient_data(tag)
        }

        let mut cursor = SketchSlice::new(bytes);

        let preamble_longs = cursor.read_u8().map_err(make_error("preamble_longs"))?;
        let serial_version = cursor.read_u8().map_err(make_error("serial_version"))?;
        let family_id = cursor.read_u8().map_err(make_error("family_id"))?;
        if family_id != TDIGEST_FAMILY_ID {
            return if preamble_longs == 0 && serial_version == 0 && family_id == 0 {
                Self::deserialize_compat(bytes)
            } else {
                Err(Error::invalid_family(
                    TDIGEST_FAMILY_ID,
                    family_id,
                    "TDigest",
                ))
            };
        }
        if serial_version != SERIAL_VERSION {
            return Err(Error::unsupported_serial_version(
                SERIAL_VERSION,
                serial_version,
            ));
        }
        let k = cursor.read_u16_le().map_err(make_error("k"))?;
        if k < 10 {
            return Err(Error::deserial(format!("k must be at least 10, got {k}")));
        }
        let flags = cursor.read_u8().map_err(make_error("flags"))?;
        let is_empty = (flags & FLAGS_IS_EMPTY) != 0;
        let is_single_value = (flags & FLAGS_IS_SINGLE_VALUE) != 0;
        let expected_preamble_longs = if is_empty || is_single_value {
            PREAMBLE_LONGS_EMPTY_OR_SINGLE
        } else {
            PREAMBLE_LONGS_MULTIPLE
        };
        if preamble_longs != expected_preamble_longs {
            return Err(Error::invalid_preamble_longs(
                expected_preamble_longs,
                preamble_longs,
            ));
        }
        cursor.read_u16_le().map_err(make_error("<unused>"))?; // unused
        if is_empty {
            return Ok(TDigestMut::new(k));
        }

        let reverse_merge = (flags & FLAGS_REVERSE_MERGE) != 0;
        if is_single_value {
            let value = if is_f32 {
                cursor.read_f32_le().map_err(make_error("single_value"))? as f64
            } else {
                cursor.read_f64_le().map_err(make_error("single_value"))?
            };
            check_non_nan(value, "single_value")?;
            check_finite(value, "single_value")?;
            return Ok(TDigestMut::make(
                k,
                reverse_merge,
                value,
                value,
                vec![Centroid {
                    mean: value,
                    weight: DEFAULT_WEIGHT,
                }],
                1,
                vec![],
            ));
        }
        let num_centroids = cursor.read_u32_le().map_err(make_error("num_centroids"))? as usize;
        let num_buffered = cursor.read_u32_le().map_err(make_error("num_buffered"))? as usize;
        let (min, max) = if is_f32 {
            (
                cursor.read_f32_le().map_err(make_error("min"))? as f64,
                cursor.read_f32_le().map_err(make_error("max"))? as f64,
            )
        } else {
            (
                cursor.read_f64_le().map_err(make_error("min"))?,
                cursor.read_f64_le().map_err(make_error("max"))?,
            )
        };
        check_non_nan(min, "min")?;
        check_non_nan(max, "max")?;
        let mut centroids = Vec::with_capacity(num_centroids);
        let mut centroids_weight = 0u64;
        for _ in 0..num_centroids {
            let (mean, weight) = if is_f32 {
                (
                    cursor.read_f32_le().map_err(make_error("mean"))? as f64,
                    cursor.read_u32_le().map_err(make_error("weight"))? as u64,
                )
            } else {
                (
                    cursor.read_f64_le().map_err(make_error("mean"))?,
                    cursor.read_u64_le().map_err(make_error("weight"))?,
                )
            };
            check_non_nan(mean, "centroid mean")?;
            check_finite(mean, "centroid")?;
            let weight = check_nonzero(weight, "centroid weight")?;
            centroids_weight += weight.get();
            centroids.push(Centroid { mean, weight });
        }
        let mut buffer = Vec::with_capacity(num_buffered);
        for _ in 0..num_buffered {
            let value = if is_f32 {
                cursor.read_f32_le().map_err(make_error("buffered_value"))? as f64
            } else {
                cursor.read_f64_le().map_err(make_error("buffered_value"))?
            };
            check_non_nan(value, "buffered_value mean")?;
            check_finite(value, "buffered_value mean")?;
            buffer.push(value);
        }
        Ok(TDigestMut::make(
            k,
            reverse_merge,
            min,
            max,
            centroids,
            centroids_weight,
            buffer,
        ))
    }

    // compatibility with the format of the reference implementation
    // default byte order of ByteBuffer is used there, which is big endian
    fn deserialize_compat(bytes: &[u8]) -> Result<Self, Error> {
        fn make_error(tag: &'static str) -> impl FnOnce(std::io::Error) -> Error {
            move |_| Error::insufficient_data_of("compat format", tag)
        }

        let mut cursor = SketchSlice::new(bytes);

        let ty = cursor.read_u32_be().map_err(make_error("type"))?;
        match ty {
            COMPAT_DOUBLE => {
                fn make_error(tag: &'static str) -> impl FnOnce(std::io::Error) -> Error {
                    move |_| Error::insufficient_data_of("compat double format", tag)
                }
                // compatibility with asBytes()
                let min = cursor.read_f64_be().map_err(make_error("min"))?;
                let max = cursor.read_f64_be().map_err(make_error("max"))?;
                check_non_nan(min, "min in compat double format")?;
                check_non_nan(max, "max in compat double format")?;
                let k = cursor.read_f64_be().map_err(make_error("k"))? as u16;
                if k < 10 {
                    return Err(Error::deserial(format!(
                        "k must be at least 10 in compat double format, got {k}"
                    )));
                }
                let num_centroids =
                    cursor.read_u32_be().map_err(make_error("num_centroids"))? as usize;
                let mut total_weight = 0u64;
                let mut centroids = Vec::with_capacity(num_centroids);
                for _ in 0..num_centroids {
                    let weight = cursor.read_f64_be().map_err(make_error("weight"))? as u64;
                    let mean = cursor.read_f64_be().map_err(make_error("mean"))?;
                    let weight = check_nonzero(weight, "centroid weight in compat double format")?;
                    check_non_nan(mean, "centroid mean in compat double format")?;
                    check_finite(mean, "centroid mean in compat double format")?;
                    total_weight += weight.get();
                    centroids.push(Centroid { mean, weight });
                }
                Ok(TDigestMut::make(
                    k,
                    false,
                    min,
                    max,
                    centroids,
                    total_weight,
                    vec![],
                ))
            }
            COMPAT_FLOAT => {
                fn make_error(tag: &'static str) -> impl FnOnce(std::io::Error) -> Error {
                    move |_| Error::insufficient_data_of("compat float format", tag)
                }
                // COMPAT_FLOAT: compatibility with asSmallBytes()
                // reference implementation uses doubles for min and max
                let min = cursor.read_f64_be().map_err(make_error("min"))?;
                let max = cursor.read_f64_be().map_err(make_error("max"))?;
                check_non_nan(min, "min in compat float format")?;
                check_non_nan(max, "max in compat float format")?;
                let k = cursor.read_f32_be().map_err(make_error("k"))? as u16;
                if k < 10 {
                    return Err(Error::deserial(format!(
                        "k must be at least 10 in compat float format, got {k}"
                    )));
                }
                // reference implementation stores capacities of the array of centroids and the
                // buffer as shorts they can be derived from k in the constructor
                cursor.read_u32_be().map_err(make_error("<unused>"))?;
                let num_centroids =
                    cursor.read_u16_be().map_err(make_error("num_centroids"))? as usize;
                let mut total_weight = 0u64;
                let mut centroids = Vec::with_capacity(num_centroids);
                for _ in 0..num_centroids {
                    let weight = cursor.read_f32_be().map_err(make_error("weight"))? as u64;
                    let mean = cursor.read_f32_be().map_err(make_error("mean"))? as f64;
                    let weight = check_nonzero(weight, "centroid weight in compat float format")?;
                    check_non_nan(mean, "centroid mean in compat float format")?;
                    check_finite(mean, "centroid mean in compat float format")?;
                    total_weight += weight.get();
                    centroids.push(Centroid { mean, weight });
                }
                Ok(TDigestMut::make(
                    k,
                    false,
                    min,
                    max,
                    centroids,
                    total_weight,
                    vec![],
                ))
            }
            ty => Err(Error::deserial(format!("unknown TDigest compat type {ty}"))),
        }
    }

    fn is_single_value(&self) -> bool {
        self.total_weight() == 1
    }

    /// Process buffered values and merge centroids if needed.
    fn compress(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let mut tmp = Vec::with_capacity(self.buffer.len() + self.centroids.len());
        for &v in &self.buffer {
            tmp.push(Centroid {
                mean: v,
                weight: DEFAULT_WEIGHT,
            });
        }
        self.do_merge(tmp, self.buffer.len() as u64)
    }

    /// Merges the given buffer of centroids into this TDigest.
    ///
    /// # Contract
    ///
    /// * `buffer` must have at least one centroid.
    /// * `buffer` is generated from `self.buffer`, and thus:
    ///     * No `NAN` values are present in `buffer`.
    ///     * We should clear `self.buffer` after merging.
    fn do_merge(&mut self, mut buffer: Vec<Centroid>, weight: u64) {
        buffer.extend(std::mem::take(&mut self.centroids));
        buffer.sort_by(centroid_cmp);
        if self.reverse_merge {
            buffer.reverse();
        }
        self.centroids_weight += weight;

        let mut num_centroids = 0;
        let len = buffer.len();
        self.centroids.push(buffer[0]);
        num_centroids += 1;
        let mut current = 1;
        let mut weight_so_far = 0.;
        while current < len {
            let c = buffer[current];
            let proposed_weight = self.centroids[num_centroids - 1].weight() + c.weight();
            let mut add_this = false;
            if (current != 1) && (current != (len - 1)) {
                let centroids_weight = self.centroids_weight as f64;
                let q0 = weight_so_far / centroids_weight;
                let q2 = (weight_so_far + proposed_weight) / centroids_weight;
                let normalizer = scale_function::normalizer((2 * self.k) as f64, centroids_weight);
                add_this = proposed_weight
                    <= (centroids_weight
                        * scale_function::max(q0, normalizer)
                            .min(scale_function::max(q2, normalizer)));
            }
            if add_this {
                // merge into existing centroid
                self.centroids[num_centroids - 1].add(c);
            } else {
                // copy to a new centroid
                weight_so_far += self.centroids[num_centroids - 1].weight();
                self.centroids.push(c);
                num_centroids += 1;
            }
            current += 1;
        }

        if self.reverse_merge {
            self.centroids.reverse();
        }
        self.min = self.min.min(self.centroids[0].mean);
        self.max = self.max.max(self.centroids[num_centroids - 1].mean);
        self.reverse_merge = !self.reverse_merge;
        self.buffer.clear();
    }
}

/// Immutable (frozen) T-Digest sketch for estimating quantiles and ranks.
///
/// See the [module documentation](super) for more details.
pub struct TDigest {
    k: u16,

    reverse_merge: bool,
    min: f64,
    max: f64,

    centroids: Vec<Centroid>,
    centroids_weight: u64,
}

impl TDigest {
    /// Returns parameter k (compression) that was used to configure this TDigest.
    pub fn k(&self) -> u16 {
        self.k
    }

    /// Returns true if TDigest has not seen any data.
    pub fn is_empty(&self) -> bool {
        self.centroids.is_empty()
    }

    /// Returns minimum value seen by TDigest; `None` if TDigest is empty.
    pub fn min_value(&self) -> Option<f64> {
        if self.is_empty() {
            None
        } else {
            Some(self.min)
        }
    }

    /// Returns maximum value seen by TDigest; `None` if TDigest is empty.
    pub fn max_value(&self) -> Option<f64> {
        if self.is_empty() {
            None
        } else {
            Some(self.max)
        }
    }

    /// Returns total weight.
    pub fn total_weight(&self) -> u64 {
        self.centroids_weight
    }

    fn view(&self) -> TDigestView<'_> {
        TDigestView {
            min: self.min,
            max: self.max,
            centroids: &self.centroids,
            centroids_weight: self.centroids_weight,
        }
    }

    /// Returns an approximation to the Cumulative Distribution Function (CDF), which is the
    /// cumulative analog of the PMF, of the input stream given a set of split points.
    ///
    /// # Arguments
    ///
    /// * `split_points`: An array of _m_ unique, monotonically increasing values that divide the
    ///   input domain into _m+1_ consecutive disjoint intervals.
    ///
    /// # Returns
    ///
    /// An array of m+1 doubles, which are a consecutive approximation to the CDF of the input
    /// stream given the split points. The value at array position j of the returned CDF array
    /// is the sum of the returned values in positions 0 through j of the returned PMF array.
    /// This can be viewed as array of ranks of the given split points plus one more value that
    /// is always 1.
    ///
    /// Returns `None` if TDigest is empty.
    ///
    /// # Panics
    ///
    /// Panics if `split_points` is not unique, not monotonically increasing, or contains `NaN`
    /// values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # for value in [1.0, 2.0, 3.0] {
    /// #     sketch.update(value);
    /// # }
    /// let digest = sketch.freeze();
    /// let cdf = digest.cdf(&[1.5]).unwrap();
    /// assert_eq!(cdf.len(), 2);
    /// ```
    pub fn cdf(&self, split_points: &[f64]) -> Option<Vec<f64>> {
        self.view().cdf(split_points)
    }

    /// Returns an approximation to the Probability Mass Function (PMF) of the input stream
    /// given a set of split points.
    ///
    /// # Arguments
    ///
    /// * `split_points`: An array of _m_ unique, monotonically increasing values that divide the
    ///   input domain into _m+1_ consecutive disjoint intervals (bins).
    ///
    /// # Returns
    ///
    /// An array of m+1 doubles each of which is an approximation to the fraction of the input
    /// stream values (the mass) that fall into one of those intervals.
    ///
    /// Returns `None` if TDigest is empty.
    ///
    /// # Panics
    ///
    /// Panics if `split_points` is not unique, not monotonically increasing, or contains `NaN`
    /// values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # for value in [1.0, 2.0, 3.0] {
    /// #     sketch.update(value);
    /// # }
    /// let digest = sketch.freeze();
    /// let pmf = digest.pmf(&[1.5]).unwrap();
    /// assert_eq!(pmf.len(), 2);
    /// ```
    pub fn pmf(&self, split_points: &[f64]) -> Option<Vec<f64>> {
        self.view().pmf(split_points)
    }

    /// Compute approximate normalized rank (from 0 to 1 inclusive) of the given value.
    ///
    /// Returns `None` if TDigest is empty.
    ///
    /// # Panics
    ///
    /// Panics if the value is `NaN`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # for value in [1.0, 2.0, 3.0] {
    /// #     sketch.update(value);
    /// # }
    /// let digest = sketch.freeze();
    /// let rank = digest.rank(2.0).unwrap();
    /// assert!((0.0..=1.0).contains(&rank));
    /// ```
    pub fn rank(&self, value: f64) -> Option<f64> {
        assert!(!value.is_nan(), "value must not be NaN");
        self.view().rank(value)
    }

    /// Compute approximate quantile value corresponding to the given normalized rank.
    ///
    /// Returns `None` if TDigest is empty.
    ///
    /// # Panics
    ///
    /// Panics if rank is not in [0.0, 1.0].
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # for value in [1.0, 2.0, 3.0] {
    /// #     sketch.update(value);
    /// # }
    /// let digest = sketch.freeze();
    /// let q = digest.quantile(0.5).unwrap();
    /// assert!((1.0..=3.0).contains(&q));
    /// ```
    pub fn quantile(&self, rank: f64) -> Option<f64> {
        assert!((0.0..=1.0).contains(&rank), "rank must be in [0.0, 1.0]");
        self.view().quantile(rank)
    }

    /// Converts this immutable TDigest into a mutable one.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::tdigest::TDigestMut;
    /// # let mut sketch = TDigestMut::new(100);
    /// # sketch.update(1.0);
    /// # let digest = sketch.freeze();
    /// let mut mutable = digest.unfreeze();
    /// mutable.update(2.0);
    /// assert_eq!(mutable.total_weight(), 2);
    /// ```
    pub fn unfreeze(self) -> TDigestMut {
        TDigestMut::make(
            self.k,
            self.reverse_merge,
            self.min,
            self.max,
            self.centroids,
            self.centroids_weight,
            vec![],
        )
    }
}

struct TDigestView<'a> {
    min: f64,
    max: f64,
    centroids: &'a [Centroid],
    centroids_weight: u64,
}

impl TDigestView<'_> {
    fn pmf(&self, split_points: &[f64]) -> Option<Vec<f64>> {
        let mut buckets = self.cdf(split_points)?;
        for i in (1..buckets.len()).rev() {
            buckets[i] -= buckets[i - 1];
        }
        Some(buckets)
    }

    fn cdf(&self, split_points: &[f64]) -> Option<Vec<f64>> {
        check_split_points(split_points);

        if self.centroids.is_empty() {
            return None;
        }

        let mut ranks = Vec::with_capacity(split_points.len() + 1);
        for &p in split_points {
            match self.rank(p) {
                Some(rank) => ranks.push(rank),
                None => unreachable!("checked non-empty above"),
            }
        }
        ranks.push(1.0);
        Some(ranks)
    }

    fn rank(&self, value: f64) -> Option<f64> {
        debug_assert!(!value.is_nan(), "value must not be NaN");

        if self.centroids.is_empty() {
            return None;
        }
        if value < self.min {
            return Some(0.0);
        }
        if value > self.max {
            return Some(1.0);
        }
        // one centroid and value == min == max
        if self.centroids.len() == 1 {
            return Some(0.5);
        }

        let centroids_weight = self.centroids_weight as f64;
        let num_centroids = self.centroids.len();

        // left tail
        let first_mean = self.centroids[0].mean;
        if value < first_mean {
            if first_mean - self.min > 0. {
                return Some(if value == self.min {
                    0.5 / centroids_weight
                } else {
                    1. + (((value - self.min) / (first_mean - self.min))
                        * ((self.centroids[0].weight() / 2.) - 1.))
                });
            }
            return Some(0.); // should never happen
        }

        // right tail
        let last_mean = self.centroids[num_centroids - 1].mean;
        if value > last_mean {
            if self.max - last_mean > 0. {
                return Some(if value == self.max {
                    1. - (0.5 / centroids_weight)
                } else {
                    1.0 - ((1.0
                        + (((self.max - value) / (self.max - last_mean))
                            * ((self.centroids[num_centroids - 1].weight() / 2.) - 1.)))
                        / centroids_weight)
                });
            }
            return Some(1.); // should never happen
        }

        let mut lower = self
            .centroids
            .binary_search_by(|c| centroid_lower_bound(c, value))
            .unwrap_or_else(identity);
        assert_ne!(lower, num_centroids, "get_rank: lower == end");
        let mut upper = self
            .centroids
            .binary_search_by(|c| centroid_upper_bound(c, value))
            .unwrap_or_else(identity);
        assert_ne!(upper, 0, "get_rank: upper == begin");
        if value < self.centroids[lower].mean {
            lower -= 1;
        }
        if (upper == num_centroids) || (self.centroids[upper - 1].mean >= value) {
            upper -= 1;
        }

        let mut weight_below = 0.;
        let mut i = 0;
        while i < lower {
            weight_below += self.centroids[i].weight();
            i += 1;
        }
        weight_below += self.centroids[lower].weight() / 2.;

        let mut weight_delta = 0.;
        while i < upper {
            weight_delta += self.centroids[i].weight();
            i += 1;
        }
        weight_delta -= self.centroids[lower].weight() / 2.;
        weight_delta += self.centroids[upper].weight() / 2.;
        Some(
            if self.centroids[upper].mean - self.centroids[lower].mean > 0. {
                (weight_below
                    + (weight_delta * (value - self.centroids[lower].mean)
                        / (self.centroids[upper].mean - self.centroids[lower].mean)))
                    / centroids_weight
            } else {
                (weight_below + weight_delta / 2.) / centroids_weight
            },
        )
    }

    fn quantile(&self, rank: f64) -> Option<f64> {
        debug_assert!((0.0..=1.0).contains(&rank), "rank must be in [0.0, 1.0]");

        if self.centroids.is_empty() {
            return None;
        }

        if self.centroids.len() == 1 {
            return Some(self.centroids[0].mean);
        }

        // at least 2 centroids
        let centroids_weight = self.centroids_weight as f64;
        let num_centroids = self.centroids.len();
        let weight = rank * centroids_weight;
        if weight < 1. {
            return Some(self.min);
        }
        if weight > centroids_weight - 1. {
            return Some(self.max);
        }
        let first_weight = self.centroids[0].weight();
        if first_weight > 1. && weight < first_weight / 2. {
            return Some(
                self.min
                    + (((weight - 1.) / ((first_weight / 2.) - 1.))
                        * (self.centroids[0].mean - self.min)),
            );
        }
        let last_weight = self.centroids[num_centroids - 1].weight();
        if last_weight > 1. && (centroids_weight - weight <= last_weight / 2.) {
            return Some(
                self.max
                    + (((centroids_weight - weight - 1.) / ((last_weight / 2.) - 1.))
                        * (self.max - self.centroids[num_centroids - 1].mean)),
            );
        }

        // interpolate between extremes
        let mut weight_so_far = first_weight / 2.;
        for i in 0..(num_centroids - 1) {
            let dw = (self.centroids[i].weight() + self.centroids[i + 1].weight()) / 2.;
            if weight_so_far + dw > weight {
                // the target weight is between centroids i and i+1
                let mut left_weight = 0.;
                if self.centroids[i].weight.get() == 1 {
                    if weight - weight_so_far < 0.5 {
                        return Some(self.centroids[i].mean);
                    }
                    left_weight = 0.5;
                }
                let mut right_weight = 0.;
                if self.centroids[i + 1].weight.get() == 1 {
                    if weight_so_far + dw - weight <= 0.5 {
                        return Some(self.centroids[i + 1].mean);
                    }
                    right_weight = 0.5;
                }
                let w1 = weight - weight_so_far - left_weight;
                let w2 = weight_so_far + dw - weight - right_weight;
                return Some(weighted_average(
                    self.centroids[i].mean,
                    w1,
                    self.centroids[i + 1].mean,
                    w2,
                ));
            }
            weight_so_far += dw;
        }

        let w1 = weight - (centroids_weight) - ((self.centroids[num_centroids - 1].weight()) / 2.);
        let w2 = (self.centroids[num_centroids - 1].weight() / 2.) - w1;
        Some(weighted_average(
            self.centroids[num_centroids - 1].mean,
            w1,
            self.max,
            w2,
        ))
    }
}

/// Checks the sequential validity of the given array of double values.
/// They must be unique, monotonically increasing and not NaN.
#[track_caller]
fn check_split_points(split_points: &[f64]) {
    let len = split_points.len();
    if len == 1 && split_points[0].is_nan() {
        panic!("split_points must not contain NaN values: {split_points:?}");
    }
    for i in 0..len - 1 {
        if split_points[i] < split_points[i + 1] {
            // we must use this positive condition because NaN comparisons are always false
            continue;
        }
        panic!("split_points must be unique and monotonically increasing: {split_points:?}");
    }
}

fn centroid_cmp(a: &Centroid, b: &Centroid) -> Ordering {
    match a.mean.partial_cmp(&b.mean) {
        Some(order) => order,
        None => unreachable!("NaN values should never be present in centroids"),
    }
}

fn centroid_lower_bound(c: &Centroid, value: f64) -> Ordering {
    if c.mean < value {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

fn centroid_upper_bound(c: &Centroid, value: f64) -> Ordering {
    if c.mean > value {
        Ordering::Greater
    } else {
        Ordering::Less
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Centroid {
    mean: f64,
    weight: NonZeroU64,
}

impl Centroid {
    fn add(&mut self, other: Centroid) {
        let (self_weight, other_weight) = (self.weight(), other.weight());
        let total_weight = self_weight + other_weight;
        self.weight = self.weight.saturating_add(other.weight.get());

        let (self_mean, other_mean) = (self.mean, other.mean);
        let ratio_self = self_weight / total_weight;
        let ratio_other = other_weight / total_weight;
        self.mean = self_mean.mul_add(ratio_self, other_mean * ratio_other);
        debug_assert!(
            !self.mean.is_nan(),
            "NaN values should never be present in centroids; self: {}, other: {}",
            self_mean,
            other_mean
        );
    }

    fn weight(&self) -> f64 {
        self.weight.get() as f64
    }
}

fn check_non_nan(value: f64, tag: &'static str) -> Result<(), Error> {
    if value.is_nan() {
        return Err(Error::deserial(format!(
            "malformed data: {tag} cannot be NaN"
        )));
    }

    Ok(())
}

fn check_finite(value: f64, tag: &'static str) -> Result<(), Error> {
    if value.is_infinite() {
        return Err(Error::deserial(format!(
            "malformed data: {tag} cannot be infinite"
        )));
    }

    Ok(())
}

fn check_nonzero(value: u64, tag: &'static str) -> Result<NonZeroU64, Error> {
    NonZeroU64::new(value)
        .ok_or_else(|| Error::deserial(format!("malformed data: {tag} cannot be zero")))
}

/// Generates cluster sizes proportional to `q*(1-q)`.
///
/// The use of a normalizing function results in a strictly bounded number of clusters no matter
/// how many samples.
///
/// Corresponds to K_2 in the reference implementation
mod scale_function {
    pub(super) fn max(q: f64, normalizer: f64) -> f64 {
        q * (1. - q) / normalizer
    }

    pub(super) fn normalizer(compression: f64, n: f64) -> f64 {
        compression / z(compression, n)
    }

    pub(super) fn z(compression: f64, n: f64) -> f64 {
        4. * (n / compression).ln() + 24.
    }
}

const fn weighted_average(x1: f64, w1: f64, x2: f64, w2: f64) -> f64 {
    (x1 * w1 + x2 * w2) / (w1 + w2)
}
