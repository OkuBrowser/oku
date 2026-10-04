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

use std::hash::Hasher;

use crate::hash::MurmurHash3X64128;

pub(super) const PREAMBLE_LONGS_SHORT: u8 = 2;
pub(super) const SERIAL_VERSION: u8 = 1;
pub(super) const COUNTMIN_FAMILY_ID: u8 = 18;
pub(super) const FLAGS_IS_EMPTY: u8 = 1 << 0;
pub(super) const LONG_SIZE_BYTES: usize = 8;

pub(super) fn compute_seed_hash(seed: u64) -> u16 {
    let mut hasher = MurmurHash3X64128::with_seed(0);
    hasher.write(&seed.to_le_bytes());
    let (h1, _) = hasher.finish128();
    (h1 & 0xffff) as u16
}
