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

//! HIP (Historical Inverse Probability) Estimator for HyperLogLog
//!
//! The HIP estimator provides improved cardinality estimation by maintaining
//! an accumulator that tracks the historical sequence of register updates.
//! This is more accurate than the standard HLL estimator, especially for
//! moderate cardinalities.

use crate::common::NumStdDev;
use crate::hll::composite_interpolation;
use crate::hll::cubic_interpolation;
use crate::hll::harmonic_numbers;

/// HIP estimator with KxQ registers for improved cardinality estimation
///
/// This struct encapsulates all estimation-related state and logic,
/// allowing it to be composed into Array4, Array6, and Array8.
///
/// The estimator supports two modes:
/// - **In-order mode**: Uses HIP (Historical Inverse Probability) accumulator for accurate
///   sequential updates
/// - **Out-of-order mode**: Uses composite estimator (raw HLL + linear counting) after
///   deserialization or merging
#[derive(Debug, Clone, PartialEq)]
pub struct HipEstimator {
    /// HIP estimator accumulator
    hip_accum: f64,
    /// KxQ register for values < 32 (larger inverse powers)
    kxq0: f64,
    /// KxQ register for values >= 32 (tiny inverse powers)
    kxq1: f64,
    /// Out-of-order flag: when true, HIP updates are skipped
    out_of_order: bool,
}

impl HipEstimator {
    /// Create a new HIP estimator for a sketch with 2^lg_config_k registers
    pub fn new(lg_config_k: u8) -> Self {
        let k = 1 << lg_config_k;
        Self {
            hip_accum: 0.0,
            kxq0: k as f64, // All registers start at 0, so kxq0 = k * (1/2^0) = k
            kxq1: 0.0,
            out_of_order: false,
        }
    }

    /// Update the estimator when a register changes from old_value to new_value
    ///
    /// This should be called BEFORE actually updating the register in the array.
    ///
    /// # Algorithm
    ///
    /// 1. Update HIP accumulator (unless out-of-order)
    /// 2. Update KxQ registers (always)
    ///
    /// The KxQ registers are split for numerical precision:
    /// - kxq0: sum of 1/2^v for v < 32
    /// - kxq1: sum of 1/2^v for v >= 32
    pub fn update(&mut self, lg_config_k: u8, old_value: u8, new_value: u8) {
        let k = (1 << lg_config_k) as f64;

        // Update HIP accumulator FIRST (unless out-of-order)
        // When out-of-order (from deserialization or merge), HIP is invalid
        if !self.out_of_order {
            self.hip_accum += k / (self.kxq0 + self.kxq1);
        }

        // Always update KxQ registers (regardless of OOO flag)
        self.update_kxq(old_value, new_value);
    }

    /// Update only the KxQ registers (internal helper)
    fn update_kxq(&mut self, old_value: u8, new_value: u8) {
        // Subtract old value contribution
        if old_value < 32 {
            self.kxq0 -= inv_pow2(old_value);
        } else {
            self.kxq1 -= inv_pow2(old_value);
        }

        // Add new value contribution
        if new_value < 32 {
            self.kxq0 += inv_pow2(new_value);
        } else {
            self.kxq1 += inv_pow2(new_value);
        }
    }

    /// Get the current cardinality estimate
    ///
    /// Dispatches to either HIP or composite estimator based on out-of-order flag.
    ///
    /// # Arguments
    ///
    /// * `lg_config_k` - Log2 of number of registers (k)
    /// * `cur_min` - Current minimum register value (for Array4, 0 for Array6/8)
    /// * `num_at_cur_min` - Number of registers at cur_min value
    pub fn estimate(&self, lg_config_k: u8, cur_min: u8, num_at_cur_min: u32) -> f64 {
        if self.out_of_order {
            self.get_composite_estimate(lg_config_k, cur_min, num_at_cur_min)
        } else {
            self.hip_accum
        }
    }

    /// Get upper bound for cardinality estimate
    ///
    /// Returns the upper confidence bound for the cardinality estimate.
    ///
    /// # Arguments
    ///
    /// * `lg_config_k` - Log2 of number of registers (k)
    /// * `cur_min` - Current minimum register value (for Array4, 0 for Array6/8)
    /// * `num_at_cur_min` - Number of registers at cur_min value
    /// * `num_std_dev` - Number of standard deviations (1, 2, or 3)
    pub fn upper_bound(
        &self,
        lg_config_k: u8,
        cur_min: u8,
        num_at_cur_min: u32,
        num_std_dev: NumStdDev,
    ) -> f64 {
        let estimate = self.estimate(lg_config_k, cur_min, num_at_cur_min);
        let rse = get_rel_err(lg_config_k, true, self.out_of_order, num_std_dev);
        // RSE is negative for upper bounds, so (1 + rse) < 1, making bound > estimate
        estimate / (1.0 + rse)
    }

    /// Get lower bound for cardinality estimate
    ///
    /// Returns the lower confidence bound for the cardinality estimate.
    ///
    /// # Arguments
    ///
    /// * `lg_config_k` - Log2 of number of registers (k)
    /// * `cur_min` - Current minimum register value (for Array4, 0 for Array6/8)
    /// * `num_at_cur_min` - Number of registers at cur_min value
    /// * `num_std_dev` - Number of standard deviations (1, 2, or 3)
    pub fn lower_bound(
        &self,
        lg_config_k: u8,
        cur_min: u8,
        num_at_cur_min: u32,
        num_std_dev: NumStdDev,
    ) -> f64 {
        let estimate = self.estimate(lg_config_k, cur_min, num_at_cur_min);
        let rse = get_rel_err(lg_config_k, false, self.out_of_order, num_std_dev);
        // RSE is positive for lower bounds, so (1 + rse) > 1, making bound < estimate
        estimate / (1.0 + rse)
    }

    /// Get raw HLL estimate using standard HyperLogLog formula
    ///
    /// Formula: correctionFactor * k^2 / (kxq0 + kxq1)
    ///
    /// Uses lg_k-specific correction factors for small k.
    fn get_raw_estimate(&self, lg_config_k: u8) -> f64 {
        let k = (1 << lg_config_k) as f64;

        // Correction factors from empirical analysis
        let correction_factor = match lg_config_k {
            4 => 0.673,
            5 => 0.697,
            6 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / k),
        };

        (correction_factor * k * k) / (self.kxq0 + self.kxq1)
    }

    /// Get linear counting (bitmap) estimate for small cardinalities
    ///
    /// Uses harmonic numbers to estimate based on empty registers.
    fn get_bitmap_estimate(&self, lg_config_k: u8, cur_min: u8, num_at_cur_min: u32) -> f64 {
        let k = 1 << lg_config_k;

        // Number of unhit (empty) buckets
        let num_unhit = if cur_min == 0 { num_at_cur_min } else { 0 };

        // Edge case: all buckets hit
        if num_unhit == 0 {
            return (k as f64) * (k as f64 / 0.5).ln();
        }

        let num_hit = k - num_unhit;
        harmonic_numbers::bitmap_estimate(k, num_hit)
    }

    /// Get composite estimate (blends raw HLL and linear counting)
    ///
    /// This is the primary estimator used when in out-of-order mode.
    /// It uses cubic interpolation on raw HLL estimate, then blends
    /// with linear counting for small cardinalities.
    fn get_composite_estimate(&self, lg_config_k: u8, cur_min: u8, num_at_cur_min: u32) -> f64 {
        let raw_est = self.get_raw_estimate(lg_config_k);

        // Get composite interpolation table
        let x_arr = composite_interpolation::get_x_arr(lg_config_k);
        let x_arr_len = composite_interpolation::get_x_arr_length();
        let y_stride = composite_interpolation::get_y_stride(lg_config_k) as f64;

        // Handle edge cases
        if raw_est < x_arr[0] {
            return 0.0;
        }

        let x_arr_len_m1 = x_arr_len - 1;

        // Above interpolation range: extrapolate linearly
        if raw_est > x_arr[x_arr_len_m1] {
            let final_y = y_stride * (x_arr_len_m1 as f64);
            let factor = final_y / x_arr[x_arr_len_m1];
            return raw_est * factor;
        }

        // Interpolate using cubic interpolation
        let adj_est = cubic_interpolation::using_x_arr_and_y_stride(x_arr, y_stride, raw_est);

        // Avoid linear counting if estimate is high
        // (threshold: 3*k ensures we're above potential linear counting instability)
        let k = 1 << lg_config_k;
        if adj_est > (3 * k) as f64 {
            return adj_est;
        }

        // Get linear counting estimate
        let lin_est = self.get_bitmap_estimate(lg_config_k, cur_min, num_at_cur_min);

        // Blend estimates based on crossover threshold
        // Use average to reduce bias from threshold comparison
        let avg_est = (adj_est + lin_est) / 2.0;

        // Crossover thresholds (empirically determined)
        let crossover = match lg_config_k {
            4 => 0.718,
            5 => 0.672,
            _ => 0.64,
        };

        let threshold = crossover * (k as f64);

        if avg_est > threshold {
            adj_est
        } else {
            lin_est
        }
    }

    /// Get the HIP accumulator value
    pub fn hip_accum(&self) -> f64 {
        self.hip_accum
    }

    /// Get the kxq0 register value
    pub fn kxq0(&self) -> f64 {
        self.kxq0
    }

    /// Get the kxq1 register value
    pub fn kxq1(&self) -> f64 {
        self.kxq1
    }

    /// Check if this estimator is in out-of-order mode
    pub fn is_out_of_order(&self) -> bool {
        self.out_of_order
    }

    /// Set the out-of-order flag
    ///
    /// This should be set to true when:
    /// - Deserializing a sketch from bytes
    /// - After a merge/union operation
    pub fn set_out_of_order(&mut self, ooo: bool) {
        self.out_of_order = ooo;
        if ooo {
            // When going out-of-order, invalidate HIP accumulator
            // (it will be recomputed if needed via composite estimator)
            self.hip_accum = 0.0;
        }
    }

    /// Set the HIP accumulator directly
    pub fn set_hip_accum(&mut self, value: f64) {
        self.hip_accum = value;
    }

    /// Set the kxq0 register directly
    pub fn set_kxq0(&mut self, value: f64) {
        self.kxq0 = value;
    }

    /// Set the kxq1 register directly
    pub fn set_kxq1(&mut self, value: f64) {
        self.kxq1 = value;
    }
}

/// Compute 1 / 2^value (inverse power of 2)
#[inline]
fn inv_pow2(value: u8) -> f64 {
    if value == 0 {
        1.0
    } else if value <= 63 {
        1.0 / (1u64 << value) as f64
    } else {
        f64::exp2(-(value as f64))
    }
}

/// Get relative error for HLL estimates
///
/// This matches the implementation in datasketches-cpp HllUtil.hpp and RelativeErrorTables.hpp
///
/// # Arguments
///
/// * `lg_config_k` - Log2 of number of registers (must be 4-21)
/// * `upper_bound` - Whether computing upper bound (vs lower bound)
/// * `ooo` - Whether sketch is out-of-order (merged/deserialized)
/// * `num_std_dev` - Number of standard deviations (1, 2, or 3)
///
/// # Returns
///
/// Relative error factor to apply to estimate
fn get_rel_err(lg_config_k: u8, upper_bound: bool, ooo: bool, num_std_dev: NumStdDev) -> f64 {
    // For lg_k > 12, use analytical formula with RSE factors
    if lg_config_k > 12 {
        // RSE factors from Apache DataSketches C++ implementation
        // HLL_HIP_RSE_FACTOR = sqrt(ln(2)) ≈ 0.8325546
        // HLL_NON_HIP_RSE_FACTOR = sqrt((3 * ln(2)) - 1) ≈ 1.03896
        let rse_factor = if ooo {
            1.03896 // Non-HIP (out-of-order)
        } else {
            0.8325546 // HIP (in-order)
        };

        let k = (1 << lg_config_k) as f64;
        let sign = if upper_bound { -1.0 } else { 1.0 };

        return sign * (num_std_dev as u8 as f64) * rse_factor / k.sqrt();
    }

    // For lg_k <= 12, use empirically measured lookup tables
    // Tables are indexed by: ((lg_k - 4) * 3) + (num_std_dev - 1)
    let idx = ((lg_config_k as usize) - 4) * 3 + ((num_std_dev as usize) - 1);

    // Select the appropriate table based on ooo and upper_bound flags
    match (ooo, upper_bound) {
        (false, false) => HIP_LB[idx],    // Case 0: HIP, Lower Bound
        (false, true) => HIP_UB[idx],     // Case 1: HIP, Upper Bound
        (true, false) => NON_HIP_LB[idx], // Case 2: Non-HIP, Lower Bound
        (true, true) => NON_HIP_UB[idx],  // Case 3: Non-HIP, Upper Bound
    }
}

// Relative error lookup tables from Apache DataSketches C++ implementation
// RelativeErrorTables-internal.hpp

/// HIP (in-order) Lower Bound errors for lg_k 4-12, std_dev 1-3
/// Q(.84134), Q(.97725), Q(.99865) quantiles
const HIP_LB: [f64; 27] = [
    0.207316195,
    0.502865572,
    0.882303765, //4
    0.146981579,
    0.335426881,
    0.557052, //5
    0.104026721,
    0.227683872,
    0.365888317, //6
    0.073614601,
    0.156781585,
    0.245740374, //7
    0.05205248,
    0.108783763,
    0.168030442, //8
    0.036770852,
    0.075727545,
    0.11593785, //9
    0.025990219,
    0.053145536,
    0.080772263, //10
    0.018373987,
    0.037266176,
    0.056271814, //11
    0.012936253,
    0.02613829,
    0.039387631, //12
];

/// HIP (in-order) Upper Bound errors for lg_k 4-12, std_dev 1-3
/// Q(.15866), Q(.02275), Q(.00135) quantiles
const HIP_UB: [f64; 27] = [
    -0.207805347,
    -0.355574279,
    -0.475535095, //4
    -0.146988328,
    -0.262390832,
    -0.360864026, //5
    -0.103877775,
    -0.191503663,
    -0.269311582, //6
    -0.073452978,
    -0.138513438,
    -0.198487447, //7
    -0.051982806,
    -0.099703123,
    -0.144128618, //8
    -0.036768609,
    -0.07138158,
    -0.104430324, //9
    -0.025991325,
    -0.050854296,
    -0.0748143, //10
    -0.01834533,
    -0.036121138,
    -0.05327616, //11
    -0.012920332,
    -0.025572893,
    -0.037896952, //12
];

/// Non-HIP (out-of-order) Lower Bound errors for lg_k 4-12, std_dev 1-3
/// Q(.84134), Q(.97725), Q(.99865) quantiles
const NON_HIP_LB: [f64; 27] = [
    0.254409839,
    0.682266712,
    1.304022158, //4
    0.181817353,
    0.443389054,
    0.778776219, //5
    0.129432281,
    0.295782195,
    0.49252279, //6
    0.091640655,
    0.201175925,
    0.323664385, //7
    0.064858051,
    0.138523393,
    0.218805328, //8
    0.045851855,
    0.095925072,
    0.148635751, //9
    0.032454144,
    0.067009668,
    0.102660669, //10
    0.022921382,
    0.046868565,
    0.071307398, //11
    0.016155679,
    0.032825719,
    0.049677541, //12
];

/// Non-HIP (out-of-order) Upper Bound errors for lg_k 4-12, std_dev 1-3
/// Q(.15866), Q(.02275), Q(.00135) quantiles
const NON_HIP_UB: [f64; 27] = [
    -0.256980172,
    -0.411905944,
    -0.52651057, // lg_k=4
    -0.182332109,
    -0.310275547,
    -0.412660505, // lg_k=5
    -0.129314228,
    -0.230142294,
    -0.315636197, // lg_k=6
    -0.091584836,
    -0.16834013,
    -0.236346847, // lg_k=7
    -0.06487411,
    -0.122045231,
    -0.174112107, // lg_k=8
    -0.04591465,
    -0.08784505,
    -0.126917615, // lg_k=9
    -0.032433119,
    -0.062897613,
    -0.091862929, // lg_k=10
    -0.022960633,
    -0.044875401,
    -0.065736049, // lg_k=11
    -0.016186662,
    -0.031827816,
    -0.046973459, // lg_k=12
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimator_initialization() {
        let est = HipEstimator::new(10); // 1024 registers

        assert_eq!(est.hip_accum(), 0.0);
        assert_eq!(est.kxq0(), 1024.0); // All zeros = 1.0 each
        assert_eq!(est.kxq1(), 0.0);
        assert!(!est.is_out_of_order());
    }

    #[test]
    fn test_estimator_update() {
        let mut est = HipEstimator::new(8); // 256 registers

        // Update from 0 to 10
        est.update(8, 0, 10);

        // HIP should have increased
        assert!(est.hip_accum() > 0.0);

        // kxq0 should have changed (10 < 32)
        assert!(est.kxq0() < 256.0);
        assert_eq!(est.kxq1(), 0.0); // kxq1 unchanged
    }

    #[test]
    fn test_kxq_split() {
        let mut est = HipEstimator::new(8);

        // Update to value < 32 (goes to kxq0)
        est.update(8, 0, 10);
        let kxq0_after_10 = est.kxq0();
        let kxq1_after_10 = est.kxq1();

        assert!(kxq0_after_10 < 256.0);
        assert_eq!(kxq1_after_10, 0.0);

        // Update from 10 to 50 (crosses the 32 boundary)
        est.update(8, 10, 50);
        let kxq0_after_50 = est.kxq0();
        let kxq1_after_50 = est.kxq1();

        assert!(kxq0_after_50 < kxq0_after_10); // Removed 1/2^10 from kxq0 (decreases kxq0)
        assert!(kxq1_after_50 > 0.0); // Added 1/2^50 to kxq1
    }

    #[test]
    fn test_out_of_order_flag() {
        let mut est = HipEstimator::new(10);

        // Normal update
        est.update(8, 0, 5);
        let hip_normal = est.hip_accum();
        assert!(hip_normal > 0.0);

        // Set out-of-order
        est.set_out_of_order(true);
        assert!(est.is_out_of_order());
        assert_eq!(est.hip_accum(), 0.0); // HIP invalidated

        // Update while OOO - HIP should not change, but kxq should
        let kxq0_before = est.kxq0();
        est.update(8, 5, 10);
        assert_eq!(est.hip_accum(), 0.0); // HIP still 0
        assert_ne!(est.kxq0(), kxq0_before); // kxq changed
    }

    #[test]
    fn test_setters() {
        let mut est = HipEstimator::new(10);

        est.set_hip_accum(123.45);
        est.set_kxq0(678.9);
        est.set_kxq1(0.0012);

        assert_eq!(est.hip_accum(), 123.45);
        assert_eq!(est.kxq0(), 678.9);
        assert_eq!(est.kxq1(), 0.0012);
    }
}
