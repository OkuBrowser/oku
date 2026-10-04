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

//! HyperLogLog Array6 mode - 6-bit packed representation
//!
//! Array6 stores HLL register values using 6 bits per slot, providing a range of 0-63.
//! This is sufficient for most HLL use cases without needing exception handling or
//! cur_min optimization like Array4.

use crate::codec::SketchBytes;
use crate::codec::SketchSlice;
use crate::common::NumStdDev;
use crate::error::Error;
use crate::hll::estimator::HipEstimator;
use crate::hll::get_slot;
use crate::hll::get_value;

const VAL_MASK_6: u16 = 0x3F; // 6 bits: 0b0011_1111

/// Core Array6 data structure - stores 6-bit values with cross-byte packing
#[derive(Debug, Clone, PartialEq)]
pub struct Array6 {
    lg_config_k: u8,
    /// Packed 6-bit values, may cross byte boundaries
    bytes: Box<[u8]>,
    /// Count of slots with value 0
    num_zeros: u32,
    /// HIP estimator for cardinality estimation
    estimator: HipEstimator,
}

impl Array6 {
    pub fn new(lg_config_k: u8) -> Self {
        let k = 1 << lg_config_k;
        let num_bytes = num_bytes_for_k(k);

        Self {
            lg_config_k,
            bytes: vec![0u8; num_bytes].into_boxed_slice(),
            num_zeros: k,
            estimator: HipEstimator::new(lg_config_k),
        }
    }

    /// Get value from a slot (6-bit value)
    ///
    /// Uses 16-bit window reads to handle values crossing byte boundaries.
    #[inline]
    fn get_raw(&self, slot: u32) -> u8 {
        let start_bit = slot * 6;
        let byte_idx = (start_bit >> 3) as usize; // Divide by 8
        let shift = (start_bit & 7) as u8; // Mod 8

        // Read 2 bytes as u16 (little-endian)
        let two_bytes = u16::from_le_bytes([self.bytes[byte_idx], self.bytes[byte_idx + 1]]);

        // Extract 6 bits at the shift position
        ((two_bytes >> shift) & VAL_MASK_6) as u8
    }

    /// Get the unpacked 6-bit value (0-63) at the given slot
    #[inline]
    pub(super) fn get(&self, slot: u32) -> u8 {
        self.get_raw(slot)
    }

    /// Get the number of registers (K = 2^lg_config_k)
    pub(super) fn num_registers(&self) -> usize {
        1 << self.lg_config_k
    }

    /// Get the current HIP accumulator value
    pub(super) fn hip_accum(&self) -> f64 {
        self.estimator.hip_accum()
    }

    /// Set value in a slot (6-bit value)
    ///
    /// Uses read-modify-write on 16-bit window to preserve surrounding bits.
    #[inline]
    fn put_raw(&mut self, slot: u32, value: u8) {
        debug_assert!(value <= 63, "6-bit value must be 0-63");

        let start_bit = slot * 6;
        let byte_idx = (start_bit >> 3) as usize;
        let shift = (start_bit & 0x7) as u8;

        // Read current 2 bytes
        let mut two_bytes = u16::from_le_bytes([self.bytes[byte_idx], self.bytes[byte_idx + 1]]);

        // Clear the 6-bit slot
        two_bytes &= !(VAL_MASK_6 << shift);

        // Insert new value
        two_bytes |= ((value as u16) & VAL_MASK_6) << shift;

        // Write back
        let bytes_out = two_bytes.to_le_bytes();
        self.bytes[byte_idx] = bytes_out[0];
        self.bytes[byte_idx + 1] = bytes_out[1];
    }

    /// Update with a coupon
    pub fn update(&mut self, coupon: u32) {
        let mask = (1 << self.lg_config_k) - 1;
        let slot = get_slot(coupon) & mask;
        let new_value = get_value(coupon);

        let old_value = self.get_raw(slot);

        if new_value > old_value {
            // Update HIP and KxQ registers via estimator
            self.estimator
                .update(self.lg_config_k, old_value, new_value);

            // Update the slot
            self.put_raw(slot, new_value);

            // Track num_zeros (count of slots with value 0)
            if old_value == 0 {
                self.num_zeros -= 1;
            }
        }
    }

    /// Get the current cardinality estimate using HIP estimator
    pub fn estimate(&self) -> f64 {
        // Array6 doesn't use cur_min (always 0), so num_at_cur_min = num_zeros
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

    /// Deserialize Array6 from HLL mode bytes
    ///
    /// Expects full HLL preamble (40 bytes) followed by packed 6-bit data.
    pub fn deserialize(
        mut cursor: SketchSlice,
        lg_config_k: u8,
        compact: bool,
        ooo: bool,
    ) -> Result<Self, Error> {
        fn make_error(tag: &'static str) -> impl FnOnce(std::io::Error) -> Error {
            move |_| Error::insufficient_data(tag)
        }

        let k = 1 << lg_config_k;
        let num_bytes = num_bytes_for_k(k);

        // Read HIP estimator values from preamble
        let hip_accum = cursor.read_f64_le().map_err(make_error("hip_accum"))?;
        let kxq0 = cursor.read_f64_le().map_err(make_error("kxq0"))?;
        let kxq1 = cursor.read_f64_le().map_err(make_error("kxq1"))?;

        // Read num_at_cur_min (for Array6, this is num_zeros since cur_min=0)
        let num_zeros = cursor.read_u32_le().map_err(make_error("num_zeros"))?;
        let _aux_count = cursor.read_u32_le().map_err(make_error("aux_count"))?; // always 0

        // Read packed byte array from offset HLL_BYTE_ARR_START
        let mut data = vec![0u8; num_bytes];
        if !compact {
            cursor.read_exact(&mut data).map_err(make_error("data"))?;
        } else {
            cursor.advance(num_bytes as u64);
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

    /// Serialize Array6 to bytes
    ///
    /// Produces full HLL preamble (40 bytes) followed by packed 6-bit data.
    pub fn serialize(&self, lg_config_k: u8) -> Vec<u8> {
        use crate::hll::serialization::*;

        let k = 1 << lg_config_k;
        let num_bytes = num_bytes_for_k(k);
        let total_size = HLL_PREAMBLE_SIZE + num_bytes;
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

        // cur_min is always 0 for Array6
        bytes.write_u8(0);

        // Mode byte: HLL mode with HLL6 type
        bytes.write_u8(encode_mode_byte(CUR_MODE_HLL, TGT_HLL6));

        // Write HIP estimator values
        bytes.write_f64_le(self.estimator.hip_accum());
        bytes.write_f64_le(self.estimator.kxq0());
        bytes.write_f64_le(self.estimator.kxq1());

        // Write num_at_cur_min (num_zeros for Array6)
        bytes.write_u32_le(self.num_zeros);

        // Write aux_count (always 0 for Array6)
        bytes.write_u32_le(0);

        // Write packed byte array
        bytes.write(&self.bytes);

        bytes.into_bytes()
    }
}

/// Calculate number of bytes needed for k slots with 6 bits each
fn num_bytes_for_k(k: u32) -> usize {
    // k slots * 6 bits = k * 6/8 bytes = k * 3/4 bytes
    // Add 1 for 16-bit window read safety
    (((k * 3) >> 2) + 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hll::coupon;
    use crate::hll::pack_coupon;

    #[test]
    fn test_num_bytes_calculation() {
        // k=16 slots: 16 * 6 bits = 96 bits = 12 bytes
        assert_eq!(num_bytes_for_k(16), (16 * 3 / 4) + 1);

        // k=1024: 1024 * 6 bits = 6144 bits = 768 bytes
        assert_eq!(num_bytes_for_k(1024), (1024 * 3 / 4) + 1);
    }

    #[test]
    fn test_get_set_raw() {
        let mut arr = Array6::new(4); // 16 slots

        // Test various 6-bit values across different slots
        arr.put_raw(0, 0);
        arr.put_raw(1, 1);
        arr.put_raw(2, 31);
        arr.put_raw(3, 63); // Max 6-bit value

        assert_eq!(arr.get_raw(0), 0);
        assert_eq!(arr.get_raw(1), 1);
        assert_eq!(arr.get_raw(2), 31);
        assert_eq!(arr.get_raw(3), 63);

        // Test that values don't interfere with each other
        arr.put_raw(5, 42);
        assert_eq!(arr.get_raw(5), 42);
        assert_eq!(arr.get_raw(3), 63); // Earlier value unchanged

        // Test all slots to ensure no cross-slot corruption
        for slot in 0..16 {
            arr.put_raw(slot, (slot % 64) as u8);
        }
        for slot in 0..16 {
            assert_eq!(arr.get_raw(slot), (slot % 64) as u8);
        }
    }

    #[test]
    fn test_boundary_crossing() {
        let mut arr = Array6::new(8); // 256 slots

        // Test values that will cross byte boundaries
        // Slot 1: starts at bit 6 (crosses byte 0/1 boundary)
        arr.put_raw(1, 0b111111);
        assert_eq!(arr.get_raw(1), 63);

        // Slot 2: starts at bit 12 (in byte 1)
        arr.put_raw(2, 0b101010);
        assert_eq!(arr.get_raw(2), 42);

        // Slot 3: starts at bit 18 (crosses byte 2/3 boundary)
        arr.put_raw(3, 0b110011);
        assert_eq!(arr.get_raw(3), 51);

        // Verify no interference
        assert_eq!(arr.get_raw(1), 63);
        assert_eq!(arr.get_raw(2), 42);
        assert_eq!(arr.get_raw(3), 51);
    }

    #[test]
    fn test_hip_estimator() {
        let mut arr = Array6::new(10); // 1024 buckets

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
    fn test_full_range() {
        let mut arr = Array6::new(6); // 64 slots

        // Test all possible 6-bit values (0-63)
        for val in 0..64u8 {
            arr.put_raw(val as u32, val);
        }

        for val in 0..64u8 {
            assert_eq!(arr.get_raw(val as u32), val);
        }
    }

    #[test]
    fn test_kxq_register_split() {
        let mut arr = Array6::new(8); // 256 buckets

        // Test that values < 32 and >= 32 are handled correctly
        arr.update(pack_coupon(0, 10)); // value < 32, goes to kxq0
        arr.update(pack_coupon(1, 40)); // value >= 32, goes to kxq1

        // Initial kxq0 = 256 (all zeros = 1.0 each)
        assert!(arr.estimator.kxq0() < 256.0, "kxq0 should have decreased");

        // kxq1 should have a small positive value (from 1/2^40)
        assert!(arr.estimator.kxq1() > 0.0, "kxq1 should be positive");
        assert!(
            arr.estimator.kxq1() < 0.001,
            "kxq1 should be small (1/2^40 is tiny)"
        );
    }
}
