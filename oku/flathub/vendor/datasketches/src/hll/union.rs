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

//! HyperLogLog Union for combining multiple HLL sketches
//!
//! The HLL Union allows combining multiple HLL sketches into a single unified
//! sketch, enabling set union operations for cardinality estimation.
//!
//! # Overview
//!
//! The union maintains an internal "gadget" sketch that accumulates the union
//! of all input sketches. It can handle sketches with:
//! - Different lg_k values (automatically resizes as needed)
//! - Different modes (List, Set, Array4/6/8)
//! - Different target HLL types

use std::hash::Hash;

use crate::common::NumStdDev;
use crate::hll::HllSketch;
use crate::hll::HllType;
use crate::hll::array4::Array4;
use crate::hll::array6::Array6;
use crate::hll::array8::Array8;
use crate::hll::mode::Mode;
use crate::hll::pack_coupon;

/// An HLL Union for combining multiple HLL sketches.
///
/// The union maintains an internal sketch (the "gadget") that accumulates
/// the union of all input sketches. It automatically handles sketches with
/// different configurations and modes.
///
/// See the [hll module level documentation](crate::hll) for more.
#[derive(Debug, Clone)]
pub struct HllUnion {
    /// Maximum lg_k that this union can handle
    lg_max_k: u8,
    /// Internal sketch that accumulates the union
    gadget: HllSketch,
}

impl HllUnion {
    /// Create a new HLL Union
    ///
    /// # Arguments
    ///
    /// * `lg_max_k` - Maximum log2 of the number of buckets. Must be in [4, 21]. This determines
    ///   the maximum precision the union can handle. Input sketches with larger lg_k will be
    ///   down-sampled.
    ///
    /// # Panics
    ///
    /// Panics if `lg_max_k` is not in the range [4, 21].
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::hll::HllType;
    /// # use datasketches::hll::HllUnion;
    /// let mut union = HllUnion::new(10);
    /// union.update_value("apple");
    /// let _result = union.get_result(HllType::Hll8);
    /// ```
    pub fn new(lg_max_k: u8) -> Self {
        assert!(
            (4..=21).contains(&lg_max_k),
            "lg_max_k must be in [4, 21], got {}",
            lg_max_k
        );

        // Start with an empty gadget at lg_max_k using Hll8
        let gadget = HllSketch::new(lg_max_k, HllType::Hll8);

        Self { lg_max_k, gadget }
    }

    /// Update the union's gadget with a value
    ///
    /// This accepts any type that implements `Hash`. The value is hashed
    /// and converted to a coupon, which is then inserted into the sketch.
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::hll::HllType;
    /// # use datasketches::hll::HllUnion;
    /// let mut union = HllUnion::new(10);
    /// union.update_value("apple");
    /// let _result = union.get_result(HllType::Hll8);
    /// ```
    pub fn update_value<T: Hash>(&mut self, value: T) {
        self.gadget.update(value);
    }

    /// Update the union with another sketch
    ///
    /// Merges the input sketch into the union's internal gadget, handling:
    /// - Sketches with different lg_k values (resizes/downsamples as needed)
    /// - Sketches in different modes (List, Set, Array4/6/8)
    /// - Sketches with different target HLL types
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::hll::HllSketch;
    /// # use datasketches::hll::HllType;
    /// # use datasketches::hll::HllUnion;
    /// let mut left = HllSketch::new(10, HllType::Hll8);
    /// let mut right = HllSketch::new(10, HllType::Hll8);
    /// left.update("apple");
    /// right.update("banana");
    ///
    /// let mut union = HllUnion::new(10);
    /// union.update(&left);
    /// union.update(&right);
    /// let result = union.get_result(HllType::Hll8);
    /// assert!(result.estimate() >= 2.0);
    /// ```
    pub fn update(&mut self, sketch: &HllSketch) {
        if sketch.is_empty() {
            return;
        }

        let src_lg_k = sketch.lg_config_k();
        let dst_lg_k = self.gadget.lg_config_k();
        let src_mode = sketch.mode();

        match src_mode {
            Mode::List { .. } | Mode::Set { .. } => {
                self.update_from_list_or_set(sketch, src_mode, src_lg_k, dst_lg_k);
            }
            Mode::Array4(_) | Mode::Array6(_) | Mode::Array8(_) => {
                self.update_from_array(src_mode, src_lg_k, dst_lg_k);
            }
        }
    }

    /// Update union from a List or Set mode sketch
    fn update_from_list_or_set(
        &mut self,
        sketch: &HllSketch,
        src_mode: &Mode,
        src_lg_k: u8,
        dst_lg_k: u8,
    ) {
        // Fast path: If gadget is empty and lg_k matches, directly copy as HLL_8
        if self.gadget.is_empty() && src_lg_k == dst_lg_k {
            self.gadget = if sketch.target_type() == HllType::Hll8 {
                sketch.clone()
            } else {
                // Convert to Hll8 by changing target type
                convert_coupon_mode_to_hll8(src_mode, src_lg_k)
            };
        } else {
            // Regular path: merge coupons into gadget
            merge_coupons_into_gadget(&mut self.gadget, src_mode);
        }
    }

    /// Update union from an Array mode sketch
    fn update_from_array(&mut self, src_mode: &Mode, src_lg_k: u8, dst_lg_k: u8) {
        // Fast path: If gadget is empty, just copy/downsample source
        if self.gadget.is_empty() {
            let new_array = copy_or_downsample(src_mode, src_lg_k, self.lg_max_k);
            let final_lg_k = new_array.num_registers().trailing_zeros() as u8;
            self.gadget = HllSketch::from_mode(final_lg_k, Mode::Array8(new_array));
            return;
        }

        let is_gadget_array = matches!(self.gadget.mode(), Mode::Array8(_));

        if is_gadget_array {
            self.merge_array_into_array_gadget(src_mode, src_lg_k, dst_lg_k);
        } else {
            self.promote_gadget_and_merge_array(src_mode, src_lg_k);
        }
    }

    /// Merge an array source into an array gadget
    fn merge_array_into_array_gadget(&mut self, src_mode: &Mode, src_lg_k: u8, dst_lg_k: u8) {
        if src_lg_k < dst_lg_k {
            // Source has lower precision - must downsize gadget
            let mut new_array = Array8::new(src_lg_k);

            match self.gadget.mode() {
                Mode::Array8(old_gadget) => {
                    merge_array_with_downsample(
                        &mut new_array,
                        src_lg_k,
                        &Mode::Array8(old_gadget.clone()),
                        dst_lg_k,
                    );
                }
                _ => {
                    unreachable!("gadget mode changed unexpectedly; should never be Array4/Array6")
                }
            }

            merge_array_same_lgk(&mut new_array, src_mode);
            self.gadget = HllSketch::from_mode(src_lg_k, Mode::Array8(new_array));
        } else {
            // Standard merge: src_lg_k >= dst_lg_k
            match self.gadget.mode_mut() {
                Mode::Array8(dst_array) => {
                    merge_array_into_array8(dst_array, dst_lg_k, src_mode, src_lg_k);
                }
                _ => {
                    unreachable!("gadget mode changed unexpectedly; should never be Array4/Array6")
                }
            }
        }
    }

    /// Promote gadget from List/Set to Array and merge array source
    fn promote_gadget_and_merge_array(&mut self, src_mode: &Mode, src_lg_k: u8) {
        let mut new_array = copy_or_downsample(src_mode, src_lg_k, self.lg_max_k);

        let old_gadget_mode = self.gadget.mode();
        merge_coupons_into_mode(&mut new_array, old_gadget_mode);

        let final_lg_k = new_array.num_registers().trailing_zeros() as u8;
        self.gadget = HllSketch::from_mode(final_lg_k, Mode::Array8(new_array));
    }

    /// Get the union result as a new sketch
    ///
    /// Returns a copy of the internal gadget sketch with the specified target HLL type.
    /// If the requested type differs from the gadget's type, conversion is performed.
    ///
    /// # Arguments
    ///
    /// * `hll_type` - The target HLL type for the result sketch (Hll4, Hll6, or Hll8)
    ///
    /// # Examples
    ///
    /// ```
    /// # use datasketches::hll::HllType;
    /// # use datasketches::hll::HllUnion;
    /// let mut union = HllUnion::new(10);
    /// union.update_value("apple");
    /// let result = union.get_result(HllType::Hll6);
    /// assert!(result.estimate() >= 1.0);
    /// ```
    pub fn get_result(&self, hll_type: HllType) -> HllSketch {
        let gadget_type = self.gadget.target_type();

        if hll_type == gadget_type {
            return self.gadget.clone();
        }

        match self.gadget.mode() {
            Mode::List { list, .. } => HllSketch::from_mode(
                self.gadget.lg_config_k(),
                Mode::List {
                    list: list.clone(),
                    hll_type,
                },
            ),
            Mode::Set { set, .. } => HllSketch::from_mode(
                self.gadget.lg_config_k(),
                Mode::Set {
                    set: set.clone(),
                    hll_type,
                },
            ),
            Mode::Array8(array8) => {
                convert_array8_to_type(array8, self.gadget.lg_config_k(), hll_type)
            }
            Mode::Array4(_) | Mode::Array6(_) => {
                unreachable!("gadget mode changed unexpectedly; should never be Array4/Array6")
            }
        }
    }

    /// Get the current lg_config_k of the internal gadget
    pub fn lg_config_k(&self) -> u8 {
        self.gadget.lg_config_k()
    }

    /// Get the maximum lg_k this union can handle
    pub fn lg_max_k(&self) -> u8 {
        self.lg_max_k
    }

    /// Check if the union is empty
    pub fn is_empty(&self) -> bool {
        self.gadget.is_empty()
    }

    /// Reset the union to its initial empty state
    ///
    /// Clears all data from the internal gadget, allowing the union to be reused
    /// for a new set of operations.
    pub fn reset(&mut self) {
        self.gadget = HllSketch::new(self.lg_max_k, HllType::Hll8);
    }

    /// Get the current cardinality estimate of the union
    pub fn estimate(&self) -> f64 {
        self.gadget.estimate()
    }

    /// Get upper bound for cardinality estimate of the union
    ///
    /// Returns the upper confidence bound for the cardinality estimate based on
    /// the number of standard deviations requested.
    pub fn upper_bound(&self, num_std_dev: NumStdDev) -> f64 {
        self.gadget.upper_bound(num_std_dev)
    }

    /// Get lower bound for cardinality estimate of the union
    ///
    /// Returns the lower confidence bound for the cardinality estimate based on
    /// the number of standard deviations requested.
    pub fn lower_bound(&self, num_std_dev: NumStdDev) -> f64 {
        self.gadget.lower_bound(num_std_dev)
    }
}

/// Convert a coupon mode (List or Set) to Hll8 target type
fn convert_coupon_mode_to_hll8(src_mode: &Mode, src_lg_k: u8) -> HllSketch {
    match src_mode {
        Mode::List { list, .. } => HllSketch::from_mode(
            src_lg_k,
            Mode::List {
                list: list.clone(),
                hll_type: HllType::Hll8,
            },
        ),
        Mode::Set { set, .. } => HllSketch::from_mode(
            src_lg_k,
            Mode::Set {
                set: set.clone(),
                hll_type: HllType::Hll8,
            },
        ),
        _ => unreachable!("convert_coupon_mode_to_hll8 called with non-coupon mode"),
    }
}

/// Merge coupons from a List or Set mode into the gadget
///
/// Iterates over all coupons in the source and updates the gadget.
/// The gadget handles mode transitions automatically (List → Set → Array).
fn merge_coupons_into_gadget(gadget: &mut HllSketch, src_mode: &Mode) {
    match src_mode {
        Mode::List { list, .. } => {
            for coupon in list.container().iter() {
                gadget.update_with_coupon(coupon);
            }
        }
        Mode::Set { set, .. } => {
            for coupon in set.container().iter() {
                gadget.update_with_coupon(coupon);
            }
        }
        Mode::Array4(_) | Mode::Array6(_) | Mode::Array8(_) => {
            unreachable!(
                "merge_coupons_into_gadget called with array mode; array modes should use merge_array_into_array8"
            );
        }
    }
}

/// Merge coupons from a List or Set mode into an Array8
fn merge_coupons_into_mode(dst: &mut Array8, src_mode: &Mode) {
    match src_mode {
        Mode::List { list, .. } => {
            for coupon in list.container().iter() {
                dst.update(coupon);
            }
        }
        Mode::Set { set, .. } => {
            for coupon in set.container().iter() {
                dst.update(coupon);
            }
        }
        Mode::Array4(_) | Mode::Array6(_) | Mode::Array8(_) => {
            unreachable!(
                "merge_coupons_into_mode called with array mode; array modes should use copy_or_downsample"
            );
        }
    }
}

/// Merge an HLL array into an Array8
///
/// Handles merging from Array4, Array6, or Array8 sources. Dispatches based on lg_k:
/// - Same lg_k: optimized bulk merge
/// - src lg_k > dst lg_k: downsample src into dst
/// - src lg_k < dst lg_k: handled by caller (requires gadget replacement)
fn merge_array_into_array8(dst_array8: &mut Array8, dst_lg_k: u8, src_mode: &Mode, src_lg_k: u8) {
    assert!(
        src_lg_k >= dst_lg_k,
        "merge_array_into_array8 requires src_lg_k >= dst_lg_k (got src={}, dst={})",
        src_lg_k,
        dst_lg_k
    );

    if dst_lg_k == src_lg_k {
        merge_array_same_lgk(dst_array8, src_mode);
    } else {
        merge_array_with_downsample(dst_array8, dst_lg_k, src_mode, src_lg_k);
    }
}

/// Extract HIP accumulator from an array mode
fn get_array_hip_accum(mode: &Mode) -> f64 {
    match mode {
        Mode::Array8(src) => src.hip_accum(),
        Mode::Array6(src) => src.hip_accum(),
        Mode::Array4(src) => src.hip_accum(),
        Mode::List { .. } | Mode::Set { .. } => {
            unreachable!("get_array_hip_accum called with non-array mode; List/Set not supported");
        }
    }
}

/// Merge Array4/Array6 into Array8 by iterating registers
fn merge_array46_same_lgk(dst: &mut Array8, num_registers: usize, get_value: impl Fn(u32) -> u8) {
    for slot in 0..num_registers {
        let val = get_value(slot as u32);
        let current = dst.values()[slot];
        if val > current {
            dst.set_register(slot, val);
        }
    }
    dst.rebuild_estimator_from_registers();
}

/// Merge arrays with same lg_k
///
/// Takes the max of corresponding registers. HIP accumulator is invalidated by the merge.
fn merge_array_same_lgk(dst: &mut Array8, src_mode: &Mode) {
    match src_mode {
        Mode::Array8(src) => {
            dst.merge_array_same_lgk(src.values());
        }
        Mode::Array6(src) => {
            merge_array46_same_lgk(dst, src.num_registers(), |slot| src.get(slot));
        }
        Mode::Array4(src) => {
            merge_array46_same_lgk(dst, src.num_registers(), |slot| src.get(slot));
        }
        _ => {
            unreachable!("merge_array_same_lgk called with non-array mode; List/Set not supported")
        }
    }
}

/// Merge Array4/Array6 into Array8 with downsampling
fn merge_array46_with_downsample(
    dst: &mut Array8,
    dst_lg_k: u8,
    num_registers: usize,
    get_value: impl Fn(u32) -> u8,
) {
    let dst_mask = (1 << dst_lg_k) - 1;
    for src_slot in 0..num_registers {
        let val = get_value(src_slot as u32);
        if val > 0 {
            let dst_slot = (src_slot as u32 & dst_mask) as usize;
            let current = dst.values()[dst_slot];
            if val > current {
                dst.set_register(dst_slot, val);
            }
        }
    }
    dst.rebuild_estimator_from_registers();
}

/// Merge arrays with downsampling (src lg_k > dst lg_k)
///
/// Multiple source registers map to each destination register via masking.
/// HIP accumulator is invalidated by the merge.
fn merge_array_with_downsample(dst: &mut Array8, dst_lg_k: u8, src_mode: &Mode, src_lg_k: u8) {
    assert!(
        src_lg_k > dst_lg_k,
        "merge_array_with_downsample requires src_lg_k > dst_lg_k (got src={}, dst={})",
        src_lg_k,
        dst_lg_k
    );

    match src_mode {
        Mode::Array8(src) => {
            dst.merge_array_with_downsample(src.values(), src_lg_k);
        }
        Mode::Array6(src) => {
            merge_array46_with_downsample(dst, dst_lg_k, src.num_registers(), |slot| src.get(slot));
        }
        Mode::Array4(src) => {
            merge_array46_with_downsample(dst, dst_lg_k, src.num_registers(), |slot| src.get(slot));
        }
        _ => unreachable!(
            "merge_array_with_downsample called with non-array mode; List/Set not supported"
        ),
    }
}

/// Convert Array8 to a different HLL type
///
/// Creates a new sketch with the requested type by copying register values
/// from the Array8 source. Preserves the HIP accumulator.
fn convert_array8_to_type(src: &Array8, lg_config_k: u8, target_type: HllType) -> HllSketch {
    match target_type {
        HllType::Hll8 => HllSketch::from_mode(lg_config_k, Mode::Array8(src.clone())),
        HllType::Hll6 => {
            let mut array6 = Array6::new(lg_config_k);
            for slot in 0..src.num_registers() {
                let val = src.values()[slot];
                if val > 0 {
                    let clamped_val = val.min(63);
                    let coupon = pack_coupon(slot as u32, clamped_val);
                    array6.update(coupon);
                }
            }

            let src_est = src.estimate();
            let arr6_est = array6.estimate();
            if src_est > arr6_est {
                array6.set_hip_accum(src_est);
            }

            HllSketch::from_mode(lg_config_k, Mode::Array6(array6))
        }
        HllType::Hll4 => {
            let mut array4 = Array4::new(lg_config_k);
            for slot in 0..src.num_registers() {
                let val = src.values()[slot];
                if val > 0 {
                    let coupon = pack_coupon(slot as u32, val);
                    array4.update(coupon);
                }
            }

            let src_est = src.estimate();
            let arr4_est = array4.estimate();
            if src_est > arr4_est {
                array4.set_hip_accum(src_est);
            }

            HllSketch::from_mode(lg_config_k, Mode::Array4(array4))
        }
    }
}

/// Copy Array4/Array6 registers into Array8 by converting to coupons
fn copy_array46_via_coupons(dst: &mut Array8, num_registers: usize, get_value: impl Fn(u32) -> u8) {
    for slot in 0..num_registers {
        let val = get_value(slot as u32);
        if val > 0 {
            let coupon = pack_coupon(slot as u32, val);
            dst.update(coupon);
        }
    }
}

/// Copy or downsample a source array to create a new Array8
///
/// Directly copies if src_lg_k <= tgt_lg_k, downsamples otherwise.
/// Result is marked as out-of-order and HIP accumulator is preserved.
fn copy_or_downsample(src_mode: &Mode, src_lg_k: u8, tgt_lg_k: u8) -> Array8 {
    if src_lg_k <= tgt_lg_k {
        let mut result = Array8::new(src_lg_k);
        let src_hip = get_array_hip_accum(src_mode);

        match src_mode {
            Mode::Array8(src) => {
                result.merge_array_same_lgk(src.values());
            }
            Mode::Array6(src) => {
                copy_array46_via_coupons(&mut result, src.num_registers(), |slot| src.get(slot));
            }
            Mode::Array4(src) => {
                copy_array46_via_coupons(&mut result, src.num_registers(), |slot| src.get(slot));
            }
            Mode::List { .. } | Mode::Set { .. } => {
                unreachable!(
                    "copy_or_downsample called with non-array mode; List/Set not supported"
                );
            }
        }

        result.set_hip_accum(src_hip);
        result
    } else {
        // Downsample from src to tgt
        let mut result = Array8::new(tgt_lg_k);
        merge_array_with_downsample(&mut result, tgt_lg_k, src_mode, src_lg_k);
        result
    }
}
