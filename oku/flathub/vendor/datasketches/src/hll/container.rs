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

//! Base container for coupon storage with cardinality estimation
//!
//! Provides a simple array-based storage for coupons (hash values) with
//! cubic interpolation-based cardinality estimation and confidence bounds.

use crate::common::NumStdDev;
use crate::hll::COUPON_RSE;
use crate::hll::coupon_mapping::X_ARR;
use crate::hll::coupon_mapping::Y_ARR;
use crate::hll::cubic_interpolation::using_x_and_y_tables;

/// Sentinel value indicating an empty coupon slot
pub const COUPON_EMPTY: u32 = 0;

/// Container for storing coupons with basic cardinality estimation
#[derive(Debug, Clone)]
pub struct Container {
    /// Log2 of container size
    lg_size: usize,
    /// Array of coupon values (0 = empty)
    pub coupons: Box<[u32]>,
    /// Number of non-empty coupons
    pub len: usize,
}

impl PartialEq for Container {
    fn eq(&self, other: &Self) -> bool {
        // Two containers are equal if they have the same non-empty coupons
        // (regardless of order or internal storage)
        if self.len != other.len {
            return false;
        }

        let mut coupons1: Vec<u32> = self
            .coupons
            .iter()
            .filter(|&&c| c != COUPON_EMPTY)
            .copied()
            .collect();
        let mut coupons2: Vec<u32> = other
            .coupons
            .iter()
            .filter(|&&c| c != COUPON_EMPTY)
            .copied()
            .collect();

        coupons1.sort_unstable();
        coupons2.sort_unstable();

        coupons1 == coupons2
    }
}

impl Container {
    pub fn new(lg_size: usize) -> Self {
        Self {
            lg_size,
            coupons: vec![COUPON_EMPTY; 1 << lg_size].into_boxed_slice(),
            len: 0,
        }
    }

    /// Create container from existing coupons
    pub fn from_coupons(lg_size: usize, coupons: Box<[u32]>, len: usize) -> Self {
        Self {
            lg_size,
            coupons,
            len,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn lg_size(&self) -> usize {
        self.lg_size
    }

    pub fn is_full(&self) -> bool {
        self.len == self.coupons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.coupons.len()
    }

    /// Get cardinality estimate using cubic interpolation
    pub fn estimate(&self) -> f64 {
        let len = self.len as f64;
        let est = using_x_and_y_tables(&X_ARR, &Y_ARR, len);
        len.max(est)
    }

    /// Get upper confidence bound for cardinality estimate
    pub fn upper_bound(&self, num_std_dev: NumStdDev) -> f64 {
        let len = self.len as f64;
        let est = using_x_and_y_tables(&X_ARR, &Y_ARR, len);
        // Upper bound: negative RSE means (1 + rse) < 1, so bound > estimate
        let rse = -(num_std_dev as u8 as f64) * COUPON_RSE;
        let bound = est / (1.0 + rse);
        len.max(bound)
    }

    /// Get lower confidence bound for cardinality estimate
    pub fn lower_bound(&self, num_std_dev: NumStdDev) -> f64 {
        let len = self.len as f64;
        let est = using_x_and_y_tables(&X_ARR, &Y_ARR, len);
        // Lower bound: positive RSE means (1 + rse) > 1, so bound < estimate
        let rse = (num_std_dev as u8 as f64) * COUPON_RSE;
        let bound = est / (1.0 + rse);
        len.max(bound)
    }

    /// Iterate over all non-empty coupons
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.coupons.iter().filter(|&&c| c != COUPON_EMPTY).copied()
    }
}
