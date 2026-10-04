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

//! Binary serialization format constants for HLL sketches
//!
//! This module contains all constants related to the Apache DataSketches
//! binary serialization format, shared across all sketch modes.

/// Family ID for HLL sketches in DataSketches format
pub const HLL_FAMILY_ID: u8 = 7;

/// Current serialization version
pub const SERIAL_VERSION: u8 = 1;

/// Flag indicating sketch is empty (no values inserted)
pub const EMPTY_FLAG_MASK: u8 = 4;
/// Flag indicating compact serialization (no empty slots stored)
pub const COMPACT_FLAG_MASK: u8 = 8;
/// Flag indicating out-of-order mode (HIP estimator invalid)
pub const OUT_OF_ORDER_FLAG_MASK: u8 = 16;

/// Preamble size for LIST mode (8 bytes = 2 ints)
pub const LIST_PREINTS: u8 = 2;
/// Preamble size for SET mode (12 bytes = 3 ints)
pub const HASH_SET_PREINTS: u8 = 3;
/// Preamble size for HLL mode (40 bytes = 10 ints)
pub const HLL_PREINTS: u8 = 10;

/// Total size of LIST preamble in bytes
pub const LIST_PREAMBLE_SIZE: usize = 8;
/// Total size of SET preamble in bytes
pub const SET_PREAMBLE_SIZE: usize = 12;
/// Total size of HLL preamble in bytes
pub const HLL_PREAMBLE_SIZE: usize = 40;

/// Extract current mode from mode byte (low 2 bits)
///
/// Returns: 0 = LIST, 1 = SET, 2 = HLL
#[inline]
pub fn extract_cur_mode(mode_byte: u8) -> u8 {
    mode_byte & 0x3
}

/// Extract target HLL type from mode byte (bits 2-3)
///
/// Returns: 0 = HLL4, 1 = HLL6, 2 = HLL8
#[inline]
pub fn extract_tgt_hll_type(mode_byte: u8) -> u8 {
    (mode_byte >> 2) & 0x3
}

/// Encode mode byte from current mode and target type
///
/// # Arguments
///
/// * `cur_mode` - 0 = LIST, 1 = SET, 2 = HLL
/// * `tgt_type` - 0 = HLL4, 1 = HLL6, 2 = HLL8
#[inline]
pub fn encode_mode_byte(cur_mode: u8, tgt_type: u8) -> u8 {
    (cur_mode & 0x3) | ((tgt_type & 0x3) << 2)
}

/// Current mode: LIST
pub const CUR_MODE_LIST: u8 = 0;

/// Current mode: SET
pub const CUR_MODE_SET: u8 = 1;

/// Current mode: HLL
pub const CUR_MODE_HLL: u8 = 2;

/// Target type: HLL4 (4-bit packed)
pub const TGT_HLL4: u8 = 0;

/// Target type: HLL6 (6-bit packed)
pub const TGT_HLL6: u8 = 1;

/// Target type: HLL8 (8-bit unpacked)
pub const TGT_HLL8: u8 = 2;

/// Size of a single coupon in bytes (u32)
pub const COUPON_SIZE_BYTES: usize = 4;
