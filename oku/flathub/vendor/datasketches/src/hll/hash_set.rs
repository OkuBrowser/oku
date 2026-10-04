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

//! Hash set for storing unique coupons with linear probing
//!
//! Uses open addressing with a custom stride function to handle collisions.
//! Provides better performance than List when many coupons are stored.

use crate::codec::SketchBytes;
use crate::codec::SketchSlice;
use crate::error::Error;
use crate::hll::HllType;
use crate::hll::KEY_MASK_26;
use crate::hll::container::COUPON_EMPTY;
use crate::hll::container::Container;
use crate::hll::serialization::*;

/// Hash set for efficient coupon storage with collision handling
#[derive(Debug, Clone, PartialEq)]
pub struct HashSet {
    container: Container,
}

impl Default for HashSet {
    fn default() -> Self {
        const LG_INIT_SET_SIZE: usize = 5;
        Self::new(LG_INIT_SET_SIZE)
    }
}

impl HashSet {
    pub fn new(lg_size: usize) -> Self {
        Self {
            container: Container::new(lg_size),
        }
    }

    /// Insert coupon into hash set, ignoring duplicates
    pub fn update(&mut self, coupon: u32) {
        let mask = (1 << self.container.lg_size()) - 1;

        // Initial probe position from low bits of coupon
        let mut probe = coupon & mask;
        let starting_position = probe;

        loop {
            let value = &mut self.container.coupons[probe as usize];
            if value == &COUPON_EMPTY {
                // Found empty slot, insert new coupon
                *value = coupon;
                self.container.len += 1;
                break;
            } else if value == &coupon {
                // Duplicate found, nothing to do
                break;
            }

            // Collision: compute stride and probe next position
            // Stride is always odd to ensure all slots are visited
            let stride = ((coupon & KEY_MASK_26) >> self.container.lg_size()) | 1;
            probe = (probe + stride) & mask;
            if probe == starting_position {
                // Invariant: the caller (HllSketch) is responsible for
                // growing / upgrading the HashSet when it's full
                unreachable!("HashSet full; no empty slots");
            }
        }
    }

    pub fn container(&self) -> &Container {
        &self.container
    }

    /// Deserialize a HashSet from bytes
    pub fn deserialize(
        mut cursor: SketchSlice,
        lg_arr: usize,
        compact: bool,
    ) -> Result<Self, Error> {
        // Read coupon count from bytes 8-11
        let coupon_count = cursor
            .read_u32_le()
            .map_err(|_| Error::insufficient_data("coupon_count"))?;
        let coupon_count = coupon_count as usize;

        if compact {
            // Compact mode: only couponCount coupons are stored
            // Create a new hash set and insert coupons one by one
            let mut hash_set = HashSet::new(lg_arr);
            for i in 0..coupon_count {
                let coupon = cursor.read_u32_le().map_err(|_| {
                    Error::insufficient_data(format!(
                        "expected {coupon_count} coupons, failed at index {i}"
                    ))
                })?;
                hash_set.update(coupon);
            }
            Ok(hash_set)
        } else {
            // Non-compact mode: full hash table with empty slots
            let array_size = 1 << lg_arr;

            // Read entire hash table including empty slots
            let mut coupons = vec![0u32; array_size];
            for (i, coupon) in coupons.iter_mut().enumerate() {
                *coupon = cursor.read_u32_le().map_err(|_| {
                    Error::insufficient_data(format!(
                        "expected {array_size} coupons, failed at index {i}"
                    ))
                })?;
            }

            Ok(Self {
                container: Container::from_coupons(
                    lg_arr,
                    coupons.into_boxed_slice(),
                    coupon_count,
                ),
            })
        }
    }

    /// Serialize a HashSet to bytes
    pub fn serialize(&self, lg_config_k: u8, hll_type: HllType) -> Vec<u8> {
        let compact = true; // Always use compact format
        let coupon_count = self.container.len();
        let lg_arr = self.container.lg_size();

        // Compute size
        let array_size = if compact { coupon_count } else { 1 << lg_arr };
        let total_size = SET_PREAMBLE_SIZE + (array_size * 4);

        let mut bytes = SketchBytes::with_capacity(total_size);

        // Write preamble
        bytes.write_u8(HASH_SET_PREINTS);
        bytes.write_u8(SERIAL_VERSION);
        bytes.write_u8(HLL_FAMILY_ID);
        bytes.write_u8(lg_config_k);
        bytes.write_u8(lg_arr as u8);

        // Write flags
        let mut flags = 0u8;
        if compact {
            flags |= COMPACT_FLAG_MASK;
        }
        bytes.write_u8(flags);

        // Write unused byte
        bytes.write_u8(0);

        // Write mode byte: SET mode with target HLL type
        bytes.write_u8(encode_mode_byte(CUR_MODE_SET, hll_type as u8));

        // Write coupon count
        bytes.write_u32_le(coupon_count as u32);

        // Write coupons
        if compact {
            // Compact mode: collect non-empty coupons and sort for deterministic output
            let mut coupons_vec: Vec<u32> = self
                .container
                .coupons
                .iter()
                .filter(|&&c| c != 0)
                .copied()
                .collect();
            coupons_vec.sort_unstable();

            for coupon in coupons_vec.iter().copied() {
                bytes.write_u32_le(coupon);
            }
        } else {
            // Non-compact mode: write entire hash table
            for coupon in self.container.coupons.iter().copied() {
                bytes.write_u32_le(coupon);
            }
        }

        bytes.into_bytes()
    }
}
