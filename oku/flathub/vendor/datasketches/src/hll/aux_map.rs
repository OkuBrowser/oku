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

//! Auxiliary hash map for HLL Array4 exceptions
//!
//! Stores slot-value pairs for values that don't fit in the 4-bit main array.
//! Uses open addressing with stride-based probing for collision resolution.

use crate::hll::RESIZE_DENOMINATOR;
use crate::hll::RESIZE_NUMERATOR;
use crate::hll::get_slot;
use crate::hll::get_value;
use crate::hll::pack_coupon;

const ENTRY_EMPTY: u32 = 0;

/// Open-addressing hash table for exception values (values >= 15)
///
/// This hash map stores (slot_number, value) pairs where values have exceeded
/// the 4-bit representation (after cur_min offset) in the main Array4.
///
/// # Entry Encoding
///
/// Each entry is an u32 packed as: [value (upper 6 bits) | slot_no (lower 26 bits)]
/// Empty entries are represented as 0.
#[derive(Debug, Clone)]
pub struct AuxMap {
    lg_size: u8,
    lg_config_k: u8,
    entries: Box<[u32]>,
    count: u32,
}

impl PartialEq for AuxMap {
    fn eq(&self, other: &Self) -> bool {
        // Two aux maps are equal if they have the same lg_config_k
        // and the same non-empty entries (regardless of internal storage order)
        if self.lg_config_k != other.lg_config_k || self.count != other.count {
            return false;
        }

        // Collect and sort non-empty entries from both maps
        let mut entries1: Vec<u32> = self
            .entries
            .iter()
            .filter(|&&e| e != ENTRY_EMPTY)
            .copied()
            .collect();
        let mut entries2: Vec<u32> = other
            .entries
            .iter()
            .filter(|&&e| e != ENTRY_EMPTY)
            .copied()
            .collect();

        entries1.sort_unstable();
        entries2.sort_unstable();

        entries1 == entries2
    }
}

/// Get lg_aux_arr_ints for a given lg_config_k
///
/// This determines the initial size of the auxiliary hash map
/// based on the sketch size.
fn lg_aux_arr_ints(lg_config_k: u8) -> u8 {
    const LG_AUX_ARR_INTS: &[u8] = &[
        0, 2, 2, 2, 2, 2, 2, 3, 3, 3, // 0-9
        4, 4, 5, 5, 6, 7, 8, 9, 10, 11, // 10-19
        12, 13, 14, 15, 16, 17, 18, // 20-26
    ];

    LG_AUX_ARR_INTS[lg_config_k as usize]
}

impl AuxMap {
    /// Create a new map with specified size
    pub fn new(lg_config_k: u8) -> Self {
        let lg_size = lg_aux_arr_ints(lg_config_k);
        Self {
            lg_size,
            lg_config_k,
            entries: vec![ENTRY_EMPTY; 1 << lg_size].into_boxed_slice(),
            count: 0,
        }
    }

    /// Insert a new slot-value pair
    pub fn insert(&mut self, slot: u32, value: u8) {
        let index = self.find(slot);
        match index {
            FindResult::Found(_) => {
                // Invariant: Array4 always check existence before inserting
                // a new value on the same slot.
                unreachable!("slot {} already exists in aux map", slot);
            }
            FindResult::Empty(idx) => {
                self.entries[idx] = pack_coupon(slot, value);
                self.count += 1;
                self.check_grow();
            }
        }
    }

    /// Get value for a slot
    ///
    /// Returns `None` if the slot is not found
    pub fn get(&self, slot: u32) -> Option<u8> {
        match self.find(slot) {
            FindResult::Found(idx) => Some(get_value(self.entries[idx])),
            FindResult::Empty(_) => None,
        }
    }

    /// Replace value for existing slot
    pub fn replace(&mut self, slot: u32, value: u8) {
        match self.find(slot) {
            FindResult::Found(idx) => {
                self.entries[idx] = pack_coupon(slot, value);
            }
            FindResult::Empty(_) => {
                // Invariant: Array4 always check existence before replacing
                // an old value on the same slot.
                unreachable!("slot {} not found in aux map", slot);
            }
        }
    }

    /// Find slot in hash table using open addressing with stride
    ///
    /// Returns either the index where the slot is found, or the index
    /// of an empty slot where it could be inserted.
    fn find(&self, slot: u32) -> FindResult {
        let mask = (1 << self.lg_size) - 1;
        let config_k_mask = (1 << self.lg_config_k) - 1;
        let mut probe = slot & mask;
        let start = probe;

        loop {
            let entry = self.entries[probe as usize];

            if entry == ENTRY_EMPTY {
                return FindResult::Empty(probe as usize);
            }

            let entry_slot = get_slot(entry) & config_k_mask;
            if entry_slot == slot {
                return FindResult::Found(probe as usize);
            }

            // Open addressing with odd stride (guarantees full coverage)
            let stride = (slot >> self.lg_size) | 1;
            probe = (probe + stride) & mask;

            if probe == start {
                // Invariant: AuxMap::insert is responsible for
                // growing the AuxMap when a new entry is inserted
                // causing the map to be full.
                unreachable!("AuxMap full; no empty slots");
            }
        }
    }

    /// Check if we need to grow the hash table (75% load factor)
    fn check_grow(&mut self) {
        let size = 1 << self.lg_size;
        if (RESIZE_DENOMINATOR * self.count) > (RESIZE_NUMERATOR * size) {
            self.grow();
        }
    }

    /// Double the hash table size and rehash all entries
    fn grow(&mut self) {
        let new_lg_size = self.lg_size + 1;
        let new_size = 1 << new_lg_size;
        let new_mask = (1 << new_lg_size) - 1;
        let mut new_entries = vec![ENTRY_EMPTY; new_size].into_boxed_slice();

        // Rehash all entries into the larger table
        for &entry in self.entries.iter() {
            if entry != ENTRY_EMPTY {
                let slot = get_slot(entry);

                // Find position in new table
                let mut probe = slot & new_mask;
                let start_position = probe;

                loop {
                    if new_entries[probe as usize] == ENTRY_EMPTY {
                        new_entries[probe as usize] = entry;
                        break;
                    }

                    let stride = (slot >> new_lg_size) | 1;
                    probe = (probe + stride) & new_mask;
                    if probe == start_position {
                        // Invariant: there will always be space for all
                        // `self.entries` in the `new_entries` array.
                        unreachable!("AuxMap full; no empty slots");
                    }
                }
            }
        }

        self.entries = new_entries;
        self.lg_size = new_lg_size;
    }

    /// Iterate over (slot, value) pairs without consuming the map
    pub fn iter(&self) -> impl Iterator<Item = (u32, u8)> + '_ {
        let config_k_mask = (1 << self.lg_config_k) - 1;
        self.entries.iter().filter_map(move |&entry| {
            if entry != ENTRY_EMPTY {
                Some((get_slot(entry) & config_k_mask, get_value(entry)))
            } else {
                None
            }
        })
    }
}

/// Iterator over AuxMap entries
pub struct AuxMapIter {
    entries: std::vec::IntoIter<u32>,
    config_k_mask: u32,
}

impl Iterator for AuxMapIter {
    type Item = (u32, u8);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.entries.next() {
                Some(entry) if entry != ENTRY_EMPTY => {
                    let slot = get_slot(entry) & self.config_k_mask;
                    let value = get_value(entry);
                    return Some((slot, value));
                }
                Some(_) => continue, // Skip empty entries
                None => return None,
            }
        }
    }
}

impl IntoIterator for AuxMap {
    type Item = (u32, u8);
    type IntoIter = AuxMapIter;

    fn into_iter(self) -> Self::IntoIter {
        AuxMapIter {
            entries: self.entries.into_vec().into_iter(),
            config_k_mask: (1 << self.lg_config_k) - 1,
        }
    }
}

/// Result of a find operation
enum FindResult {
    Found(usize),
    Empty(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aux_map_basic_operations() {
        let mut map = AuxMap::new(10);

        // Insert some values
        map.insert(10, 20);
        map.insert(50, 30);
        map.insert(100, 40);

        // Get values
        assert_eq!(map.get(10), Some(20));
        assert_eq!(map.get(50), Some(30));
        assert_eq!(map.get(100), Some(40));
        assert_eq!(map.get(999), None);

        // Replace value
        map.replace(50, 35);
        assert_eq!(map.get(50), Some(35));
    }

    #[test]
    fn test_aux_map_growth() {
        let mut map = AuxMap::new(8);

        // Insert enough to trigger resize (75% load factor)
        map.insert(1, 15);
        map.insert(2, 16);
        map.insert(3, 17);
        // This should trigger a resize
        map.insert(4, 18);

        // All values should still be accessible
        assert_eq!(map.get(1), Some(15));
        assert_eq!(map.get(2), Some(16));
        assert_eq!(map.get(3), Some(17));
        assert_eq!(map.get(4), Some(18));
    }

    #[test]
    #[should_panic(expected = "already exists")]
    fn test_aux_map_duplicate_insert() {
        let mut map = AuxMap::new(10);
        map.insert(10, 20);
        map.insert(10, 30); // Should panic
    }

    #[test]
    #[should_panic(expected = "not found")]
    fn test_aux_map_replace_missing() {
        let mut map = AuxMap::new(10);
        map.replace(999, 20); // Should panic
    }
}
