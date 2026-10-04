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

//! HyperLogLog Array8 mode - 8-bit (1 byte per slot) representation
//!
//! Array8 is the simplest HLL array implementation, storing one byte per slot.
//! This provides the maximum value range (0-255) with no bit-packing complexity.

use crate::codec::SketchBytes;
use crate::codec::SketchSlice;
use crate::common::NumStdDev;
use crate::error::Error;
use crate::hll::estimator::HipEstimator;
use crate::hll::get_slot;
use crate::hll::get_value;

/// Core Array8 data structure - one byte per slot, no packing
#[derive(Debug, Clone, PartialEq)]
pub struct Array8 {
    lg_config_k: u8,
    /// Direct byte array: bytes[slot] = value
    bytes: Box<[u8]>,
    /// Count of slots with value 0
    num_zeros: u32,
    /// HIP estimator for cardinality estimation
    estimator: HipEstimator,
}

impl Array8 {
    pub fn new(lg_config_k: u8) -> Self {
        let k = 1 << lg_config_k;

        Self {
            lg_config_k,
            bytes: vec![0u8; k as usize].into_boxed_slice(),
            num_zeros: k,
            estimator: HipEstimator::new(lg_config_k),
        }
    }

    /// Get value from a slot
    ///
    /// Direct array access - no bit manipulation required.
    #[inline]
    pub fn get(&self, slot: u32) -> u8 {
        self.bytes[slot as usize]
    }

    /// Set value in a slot
    ///
    /// Direct array write - no bit manipulation required.
    #[inline]
    fn put(&mut self, slot: u32, value: u8) {
        self.bytes[slot as usize] = value;
    }

    /// Update with a coupon
    pub fn update(&mut self, coupon: u32) {
        let mask = (1 << self.lg_config_k) - 1;
        let slot = get_slot(coupon) & mask;
        let new_value = get_value(coupon);

        let old_value = self.get(slot);

        if new_value > old_value {
            // Update HIP and KxQ registers via estimator
            self.estimator
                .update(self.lg_config_k, old_value, new_value);

            // Update the slot
            self.put(slot, new_value);

            // Track num_zeros (count of slots with value 0)
            if old_value == 0 {
                self.num_zeros -= 1;
            }
        }
    }

    /// Get the current cardinality estimate using HIP estimator
    pub fn estimate(&self) -> f64 {
        // Array8 doesn't use cur_min (always 0), so num_at_cur_min = num_zeros
        self.estimator.estimate(self.lg_config_k, 0, self.num_zeros)
    }

    /// Get upper bound for cardinality estimate
    pub fn upper_bound(&self, num_std_dev: NumStdDev) -> f64 {
        self.estimator
            .upper_bound(self.lg_config_k, 0, self.num_zeros, num_std_dev)
    }

    /// Get lower bound for cardinality estimate
    pub fn lower_bound(&self, num_std_dev: NumStdDev) -> f64 {
        self.estimator
            .lower_bound(self.lg_config_k, 0, self.num_zeros, num_std_dev)
    }

    /// Set the HIP accumulator value
    ///
    /// This is used when promoting from coupon modes to carry forward the estimate
    pub fn set_hip_accum(&mut self, value: f64) {
        self.estimator.set_hip_accum(value);
    }

    /// Check if the sketch is empty (all slots are zero)
    pub fn is_empty(&self) -> bool {
        self.num_zeros == (1 << self.lg_config_k)
    }

    /// Get read access to register values (one byte per register)
    pub(super) fn values(&self) -> &[u8] {
        &self.bytes
    }

    /// Get the number of registers (K = 2^lg_config_k)
    pub(super) fn num_registers(&self) -> usize {
        1 << self.lg_config_k
    }

    /// Get the current HIP accumulator value
    pub(super) fn hip_accum(&self) -> f64 {
        self.estimator.hip_accum()
    }

    /// Directly set a register value
    ///
    /// This bypasses the normal update path and directly modifies the register.
    /// Caller must call rebuild_estimator_from_registers() after all modifications.
    pub(super) fn set_register(&mut self, slot: usize, value: u8) {
        self.bytes[slot] = value;
    }

    /// Rebuild estimator state from current register values
    ///
    /// Recomputes num_zeros, kxq0, kxq1, and marks estimator as out-of-order.
    /// Should be called after bulk register modifications.
    pub(super) fn rebuild_estimator_from_registers(&mut self) {
        self.rebuild_cached_values();
        self.estimator.set_out_of_order(true);
    }

    /// Merge another Array8 with the same lg_k
    ///
    /// Performs register-by-register max merge. Marks estimator as
    /// out-of-order since HIP cannot be maintained during bulk operations.
    ///
    /// # Panics
    ///
    /// Panics if src length doesn't match self length (different lg_k).
    pub(super) fn merge_array_same_lgk(&mut self, src: &[u8]) {
        assert_eq!(
            src.len(),
            self.bytes.len(),
            "Source and destination must have same lg_k"
        );

        for (i, &val) in src.iter().enumerate() {
            self.bytes[i] = self.bytes[i].max(val);
        }

        self.rebuild_cached_values();
        self.estimator.set_out_of_order(true);
    }

    /// Merge an array with larger lg_k (downsampling)
    ///
    /// When merging a source with lg_k > dst lg_k, multiple source registers
    /// map to each destination register using the masking operation:
    /// `dst_slot = src_slot & ((1 << dst_lg_k) - 1)`
    ///
    /// The destination takes the max of all source values that map to it.
    ///
    /// # Parameters
    ///
    /// * `src` - Source register values (length must be 2^src_lg_k)
    /// * `src_lg_k` - Log2 of source register count
    ///
    /// # Panics
    ///
    /// Panics if src_lg_k <= self.lg_config_k (not downsampling).
    pub(super) fn merge_array_with_downsample(&mut self, src: &[u8], src_lg_k: u8) {
        assert!(
            src_lg_k > self.lg_config_k,
            "Source lg_k must be greater than destination lg_k for downsampling"
        );
        assert_eq!(
            src.len(),
            1 << src_lg_k,
            "Source length must match 2^src_lg_k"
        );

        let dst_mask = (1 << self.lg_config_k) - 1;

        for (src_slot, &val) in src.iter().enumerate() {
            let dst_slot = (src_slot as u32 & dst_mask) as usize;
            self.bytes[dst_slot] = self.bytes[dst_slot].max(val);
        }

        self.rebuild_cached_values();
        self.estimator.set_out_of_order(true);
    }

    /// Rebuild cached values after bulk modifications
    ///
    /// Recomputes num_zeros by counting zero-valued registers.
    /// This is needed after merge operations that bypass normal update paths.
    fn rebuild_cached_values(&mut self) {
        self.num_zeros = self.bytes.iter().filter(|&&v| v == 0).count() as u32;

        // Recompute kxq values from actual register values
        // This is essential after bulk merges where registers change but estimator isn't updated
        // incrementally
        let mut kxq0_sum = 0.0;
        let mut kxq1_sum = 0.0;

        for &val in self.bytes.iter() {
            if val == 0 {
                kxq0_sum += 1.0;
            } else if val < 32 {
                kxq0_sum += 1.0 / (1u64 << val) as f64;
            } else {
                kxq1_sum += 1.0 / (1u64 << val) as f64;
            }
        }

        self.estimator.set_kxq0(kxq0_sum);
        self.estimator.set_kxq1(kxq1_sum);
    }

    /// Deserialize Array8 from HLL mode bytes
    ///
    /// Expects full HLL preamble (40 bytes) followed by k bytes of data.
    pub fn deserialize(
        mut cursor: SketchSlice,
        lg_config_k: u8,
        compact: bool,
        ooo: bool,
    ) -> Result<Self, Error> {
        fn make_error(tag: &'static str) -> impl FnOnce(std::io::Error) -> Error {
            move |_| Error::insufficient_data(tag)
        }

        let k = 1usize << lg_config_k;

        // Read HIP estimator values from preamble
        let hip_accum = cursor.read_f64_le().map_err(make_error("hip_accum"))?;
        let kxq0 = cursor.read_f64_le().map_err(make_error("kxq0"))?;
        let kxq1 = cursor.read_f64_le().map_err(make_error("kxq1"))?;

        // Read num_at_cur_min (for Array8, this is num_zeros since cur_min=0)
        let num_zeros = cursor.read_u32_le().map_err(make_error("num_zeros"))?;
        let _aux_count = cursor.read_u32_le().map_err(make_error("aux_count"))?; // always 0

        // Read byte array from offset HLL_BYTE_ARR_START
        let mut data = vec![0u8; k];
        if !compact {
            cursor.read_exact(&mut data).map_err(make_error("data"))?;
        } else {
            cursor.advance(k as u64);
        }

        // Create estimator and restore state
        let mut estimator = HipEstimator::new(lg_config_k);
        estimator.set_hip_accum(hip_accum);
        estimator.set_kxq0(kxq0);
        estimator.set_kxq1(kxq1);
        estimator.set_out_of_order(ooo);

        Ok(Self {
            lg_config_k,
            bytes: data.into_boxed_slice(),
            num_zeros,
            estimator,
        })
    }

    /// Serialize Array8 to bytes
    ///
    /// Produces full HLL preamble (40 bytes) followed by k bytes of data.
    pub fn serialize(&self, lg_config_k: u8) -> Vec<u8> {
        use crate::hll::serialization::*;

        let k = 1 << lg_config_k;
        let total_size = HLL_PREAMBLE_SIZE + k as usize;
        let mut bytes = SketchBytes::with_capacity(total_size);

        // Write standard header
        bytes.write_u8(HLL_PREINTS);
        bytes.write_u8(SERIAL_VERSION);
        bytes.write_u8(HLL_FAMILY_ID);
        bytes.write_u8(lg_config_k);
        bytes.write_u8(0); // unused for HLL mode

        // Write flags
        let mut flags = 0u8;
        if self.estimator.is_out_of_order() {
            flags |= OUT_OF_ORDER_FLAG_MASK;
        }
        bytes.write_u8(flags);

        // cur_min is always 0 for Array8
        bytes.write_u8(0);

        // Mode byte: HLL mode with HLL8 type
        bytes.write_u8(encode_mode_byte(CUR_MODE_HLL, TGT_HLL8));

        // Write HIP estimator values
        bytes.write_f64_le(self.estimator.hip_accum());
        bytes.write_f64_le(self.estimator.kxq0());
        bytes.write_f64_le(self.estimator.kxq1());

        // Write num_at_cur_min (num_zeros for Array8)
        bytes.write_u32_le(self.num_zeros);

        // Write aux_count (always 0 for Array8)
        bytes.write_u32_le(0);

        // Write byte array
        bytes.write(&self.bytes);

        bytes.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hll::coupon;
    use crate::hll::pack_coupon;

    #[test]
    fn test_array8_basic() {
        let arr = Array8::new(10); // 1024 buckets

        // Initially all slots should be 0
        assert_eq!(arr.get(0), 0);
        assert_eq!(arr.get(100), 0);
        assert_eq!(arr.get(1023), 0);
    }

    #[test]
    fn test_get_set() {
        let mut arr = Array8::new(4); // 16 slots

        // Test all possible 8-bit values
        for slot in 0..16 {
            arr.put(slot, (slot * 17) as u8); // Various values
        }

        for slot in 0..16 {
            assert_eq!(arr.get(slot), (slot * 17) as u8);
        }

        // Test full range (0-255)
        arr.put(0, 0);
        arr.put(1, 127);
        arr.put(2, 255);

        assert_eq!(arr.get(0), 0);
        assert_eq!(arr.get(1), 127);
        assert_eq!(arr.get(2), 255);
    }

    #[test]
    fn test_update_basic() {
        let mut arr = Array8::new(4);

        // Update slot 0 with value 5
        arr.update(pack_coupon(0, 5));
        assert_eq!(arr.get(0), 5);

        // Update with a smaller value (should be ignored)
        arr.update(pack_coupon(0, 3));
        assert_eq!(arr.get(0), 5);

        // Update with a larger value
        arr.update(pack_coupon(0, 42));
        assert_eq!(arr.get(0), 42);

        // Test value at max coupon range (63)
        // Note: pack_coupon only stores 6 bits (0-63)
        arr.update(pack_coupon(1, 63));
        assert_eq!(arr.get(1), 63);
    }

    #[test]
    fn test_hip_estimator() {
        let mut arr = Array8::new(10); // 1024 buckets

        // Initially estimate should be 0
        assert_eq!(arr.estimate(), 0.0);

        // Add some unique values using real coupon hashing
        for i in 0..10_000u32 {
            let coupon = coupon(i);
            arr.update(coupon);
        }

        let estimate = arr.estimate();

        // Sanity checks
        assert!(estimate > 0.0, "Estimate should be positive");
        assert!(estimate.is_finite(), "Estimate should be finite");

        // Rough bounds for 10K unique items (very loose)
        assert!(estimate > 1_000.0, "Estimate seems too low");
        assert!(estimate < 100_000.0, "Estimate seems too high");
    }

    #[test]
    fn test_full_value_range() {
        let mut arr = Array8::new(8); // 256 slots

        // Test all possible 8-bit values (0-255)
        for val in 0..=255u8 {
            arr.put(val as u32, val);
        }

        for val in 0..=255u8 {
            assert_eq!(arr.get(val as u32), val);
        }
    }

    #[test]
    fn test_high_value_direct() {
        let mut arr = Array8::new(6); // 64 slots

        // Test that Array8 CAN store full range (0-255) directly
        // Even though coupons are limited to 6 bits (0-63)
        // Direct put/get bypasses coupon encoding
        let test_values = [16, 32, 64, 128, 200, 255];

        for (slot, &value) in test_values.iter().enumerate() {
            arr.put(slot as u32, value);
            assert_eq!(arr.get(slot as u32), value);
        }

        // Verify no cross-slot corruption
        for (slot, &value) in test_values.iter().enumerate() {
            assert_eq!(arr.get(slot as u32), value);
        }
    }

    #[test]
    fn test_kxq_register_split() {
        let mut arr = Array8::new(8); // 256 buckets

        // Test that values < 32 and >= 32 are handled correctly
        arr.update(pack_coupon(0, 10)); // value < 32, goes to kxq0
        arr.update(pack_coupon(1, 50)); // value >= 32, goes to kxq1

        // Initial kxq0 = 256 (all zeros = 1.0 each)
        assert!(arr.estimator.kxq0() < 256.0, "kxq0 should have decreased");

        // kxq1 should have a positive value (from 1/2^50)
        assert!(arr.estimator.kxq1() > 0.0, "kxq1 should be positive");
        assert!(
            arr.estimator.kxq1() < 1e-10,
            "kxq1 should be very small (1/2^50 ≈ 8.9e-16)"
        );
    }

    #[test]
    fn test_values_access() {
        let mut arr = Array8::new(4); // 16 slots

        // Set some values
        arr.put(0, 10);
        arr.put(5, 25);
        arr.put(15, 63);

        // Test read access via values()
        let vals = arr.values();
        assert_eq!(vals.len(), 16);
        assert_eq!(vals[0], 10);
        assert_eq!(vals[5], 25);
        assert_eq!(vals[15], 63);
        assert_eq!(vals[1], 0); // Untouched slot
    }

    #[test]
    fn test_merge_array_same_lgk() {
        let mut dst = Array8::new(4); // 16 slots
        let mut src = Array8::new(4); // 16 slots

        // Set up dst with some values
        dst.put(0, 10);
        dst.put(1, 20);
        dst.put(2, 30);

        // Set up src with overlapping and new values
        src.put(1, 15); // Smaller than dst[1]=20, should keep 20
        src.put(2, 35); // Larger than dst[2]=30, should update to 35
        src.put(3, 40); // New value

        // Merge src into dst
        dst.merge_array_same_lgk(src.values());

        // Check results
        assert_eq!(dst.get(0), 10, "dst[0] unchanged");
        assert_eq!(dst.get(1), 20, "dst[1] kept max value");
        assert_eq!(dst.get(2), 35, "dst[2] updated to larger value");
        assert_eq!(dst.get(3), 40, "dst[3] got new value");

        // Verify estimator marked as OOO
        assert!(dst.estimator.is_out_of_order());

        // Verify num_zeros updated (should be 12: 16 - 4 non-zero)
        assert_eq!(dst.num_zeros, 12);
    }

    #[test]
    fn test_merge_array_with_downsample() {
        // Downsampling from lg_k=5 (32 slots) to lg_k=4 (16 slots)
        let mut dst = Array8::new(4); // 16 slots
        let mut src = Array8::new(5); // 32 slots

        // Set up dst
        dst.put(0, 10);
        dst.put(1, 20);

        // Set up src - slots 0 and 16 both map to dst slot 0
        src.put(0, 15); // maps to dst[0], max(10, 15) = 15
        src.put(16, 25); // maps to dst[0], max(15, 25) = 25
        src.put(1, 18); // maps to dst[1], max(20, 18) = 20
        src.put(17, 30); // maps to dst[1], max(20, 30) = 30

        // Merge with downsampling
        dst.merge_array_with_downsample(src.values(), 5);

        // Check results - dst takes max of all src slots that map to it
        assert_eq!(dst.get(0), 25, "dst[0] = max(10, 15, 25)");
        assert_eq!(dst.get(1), 30, "dst[1] = max(20, 18, 30)");

        // Verify estimator marked as OOO
        assert!(dst.estimator.is_out_of_order());
    }

    #[test]
    #[should_panic(expected = "Source and destination must have same lg_k")]
    fn test_merge_same_lgk_panics_on_size_mismatch() {
        let mut dst = Array8::new(4); // 16 slots
        let src = Array8::new(5); // 32 slots - wrong size!

        dst.merge_array_same_lgk(src.values());
    }

    #[test]
    #[should_panic(expected = "Source lg_k must be greater")]
    fn test_merge_downsample_panics_if_not_downsampling() {
        let mut dst = Array8::new(5); // 32 slots
        let src = Array8::new(4); // 16 slots - can't upsample!

        dst.merge_array_with_downsample(src.values(), 4);
    }

    #[test]
    fn test_rebuild_cached_values() {
        let mut arr = Array8::new(4); // 16 slots

        // Set some non-zero values
        arr.put(0, 10);
        arr.put(1, 20);
        arr.put(2, 30);

        // Manually corrupt num_zeros
        arr.num_zeros = 999;

        // Rebuild should fix it
        arr.rebuild_cached_values();

        // Should be 13 zeros (16 total - 3 non-zero)
        assert_eq!(arr.num_zeros, 13);
    }

    #[test]
    fn test_merge_preserves_max_semantics() {
        let mut dst = Array8::new(4);
        let mut src = Array8::new(4);

        // Fill dst with ascending values
        for i in 0..16 {
            dst.put(i, i as u8);
        }

        // Fill src with descending values
        for i in 0..16 {
            src.put(i, (15 - i) as u8);
        }

        dst.merge_array_same_lgk(src.values());

        // Result should be max at each position
        for i in 0..16 {
            let expected = (i as u8).max((15 - i) as u8);
            assert_eq!(
                dst.get(i),
                expected,
                "slot {} should be max({}, {}) = {}",
                i,
                i,
                15 - i,
                expected
            );
        }
    }
}
