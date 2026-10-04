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

use std::hash::Hash;

use crate::common::ResizeFactor;
use crate::hash::MurmurHash3X64128;

/// Maximum theta value (signed max for compatibility with Java)
pub const MAX_THETA: u64 = i64::MAX as u64;

/// Minimum log2 of K
pub const MIN_LG_K: u8 = 5;

/// Maximum log2 of K
pub const MAX_LG_K: u8 = 26;

/// Default log2 of K
pub const DEFAULT_LG_K: u8 = 12;

/// Resize threshold (0.5 = 50% load factor)
const RESIZE_THRESHOLD: f64 = 0.5;

/// Rebuild threshold (15/16 = 93.75% load factor)
const REBUILD_THRESHOLD: f64 = 15.0 / 16.0;

/// Stride hash bits (7 bits for stride calculation)
const STRIDE_HASH_BITS: u8 = 7;

/// Stride mask
const STRIDE_MASK: u64 = (1 << STRIDE_HASH_BITS) - 1;

/// Specific hash table for theta sketch
///
/// It maintains an array capacity max to 2^lg_max_size:
/// * Before it reaches the max capacity, it will extend the array based on resize_factor.
/// * After it reaches the capacity bigger than 2^lg_nom_size, every time the number of entries
///   exceeds the threshold, it will rebuild the table: only keep the min 2^lg_nom_size entries and
///   update the theta to the k-th smallest entry.
#[derive(Debug)]
pub(crate) struct ThetaHashTable {
    lg_cur_size: u8,
    lg_nom_size: u8,
    lg_max_size: u8,
    resize_factor: ResizeFactor,
    sampling_probability: f32,
    hash_seed: u64,

    theta: u64,

    entries: Vec<u64>,
    num_entries: usize,
}

impl ThetaHashTable {
    /// Create a new hash table
    pub fn new(
        lg_nom_size: u8,
        resize_factor: ResizeFactor,
        sampling_probability: f32,
        hash_seed: u64,
    ) -> Self {
        let lg_max_size = lg_nom_size + 1;
        let lg_cur_size = starting_sub_multiple(lg_max_size, MIN_LG_K, resize_factor.lg_value());
        let size = if lg_cur_size > 0 { 1 << lg_cur_size } else { 0 };
        let entries = vec![0u64; size];

        Self {
            lg_cur_size,
            lg_nom_size,
            lg_max_size,
            resize_factor,
            sampling_probability,
            theta: starting_theta_from_sampling_probability(sampling_probability),
            hash_seed,
            entries,
            num_entries: 0,
        }
    }

    /// Hash and screen a value
    ///
    /// Returns the hash value if it passes the theta threshold, otherwise 0.
    pub fn hash_and_screen<T: Hash>(&mut self, value: T) -> u64 {
        let mut hasher = MurmurHash3X64128::with_seed(self.hash_seed);
        value.hash(&mut hasher);
        let (h1, _) = hasher.finish128();
        let hash = h1 >> 1; // To make it compatible with Java version
        if hash >= self.theta {
            return 0; // hash == 0 is reserved for empty slots
        }
        hash
    }

    /// Find an entry in the hash table.
    ///
    /// Returns the index of the entry if found, otherwise None. The entry may have been inserted or
    /// empty.
    fn find_in_curr_entries(&self, key: u64) -> Option<usize> {
        Self::find_in_entries(&self.entries, key, self.lg_cur_size)
    }

    /// Find index in a given entries.
    ///
    /// Returns the index of the entry if found, otherwise None. The entry may have been inserted or
    /// empty.
    fn find_in_entries(entries: &[u64], key: u64, lg_size: u8) -> Option<usize> {
        if entries.is_empty() {
            return None;
        }

        let size = entries.len();
        let mask = size - 1;
        let stride = Self::get_stride(key, lg_size);
        let mut index = (key as usize) & mask;
        let loop_index = index;

        loop {
            let probe = entries[index];
            if probe == 0 || probe == key {
                return Some(index);
            }
            index = (index + stride) & mask;
            if index == loop_index {
                return None;
            }
        }
    }

    /// Insert a hash value into the table
    ///
    /// Returns true if the value was inserted (new), false otherwise.
    pub fn try_insert(&mut self, hash: u64) -> bool {
        if hash == 0 {
            return false;
        }

        let Some(index) = self.find_in_curr_entries(hash) else {
            unreachable!(
                "Resize or rebuild should be called to make sure it always can find the entry."
            );
        };

        // Already exists
        if self.entries[index] == hash {
            return false;
        }

        assert_eq!(self.entries[index], 0, "Entry should be empty");
        self.entries[index] = hash;
        self.num_entries += 1;

        // Check if we need to resize or rebuild
        let capacity = self.get_capacity();
        if self.num_entries > capacity {
            if self.lg_cur_size <= self.lg_nom_size {
                self.resize();
            } else {
                self.rebuild();
            }
        }
        true
    }

    /// Get capacity threshold
    fn get_capacity(&self) -> usize {
        let fraction = if self.lg_cur_size <= self.lg_nom_size {
            RESIZE_THRESHOLD
        } else {
            REBUILD_THRESHOLD
        };
        (fraction * self.entries.len() as f64) as usize
    }

    /// Resize the hash table
    fn resize(&mut self) {
        let new_lg_size = std::cmp::min(
            self.lg_cur_size + self.resize_factor.lg_value(),
            self.lg_max_size,
        );
        let new_size = 1 << new_lg_size;

        // Get new entries and rehash all entries
        let mut new_entries = vec![0u64; new_size];
        for &entry in &self.entries {
            if entry != 0 {
                let new_index = Self::find_in_entries(&new_entries, entry, new_lg_size);
                if let Some(idx) = new_index {
                    new_entries[idx] = entry;
                } else {
                    unreachable!(
                        "find_in_entries should always return Some if the entry is not empty."
                    );
                }
            }
        }

        self.entries = new_entries;
        self.lg_cur_size = new_lg_size;
    }

    /// Rebuild the hash table:
    /// The number of entries will be reduced to the nominal size k.
    fn rebuild(&mut self) {
        // Select the k-th smallest entry as new theta and keep the lesser entries.
        self.entries.retain(|&e| e != 0);
        let k = 1u64 << self.lg_nom_size;
        let (lesser, kth, _) = self.entries.select_nth_unstable(k as usize);
        self.theta = *kth;

        // Rebuild the table with the lesser entries.
        let size = 1 << self.lg_cur_size;
        let mut new_entries = vec![0u64; size];
        let mut num_inserted = 0;
        for entry in lesser {
            if let Some(idx) = Self::find_in_entries(&new_entries, *entry, self.lg_cur_size) {
                new_entries[idx] = *entry;
                num_inserted += 1;
            } else {
                unreachable!(
                    "find_in_entries should always return Some if the entry is not empty."
                );
            }
        }

        assert_eq!(
            num_inserted, k as usize,
            "Number of inserted entries should be equal to k."
        );
        self.num_entries = num_inserted;
        self.entries = new_entries;
    }

    /// Trim the table to nominal size k
    pub fn trim(&mut self) {
        if self.num_entries > (1 << self.lg_nom_size) {
            self.rebuild();
        }
    }

    /// Reset the table to empty state
    pub fn reset(&mut self) {
        let init_theta = starting_theta_from_sampling_probability(self.sampling_probability);
        let init_lg_cur = starting_sub_multiple(
            self.lg_nom_size + 1,
            MIN_LG_K,
            self.resize_factor.lg_value(),
        );

        // clear entries
        if self.entries.len() != 1 << init_lg_cur {
            self.entries.resize(1 << init_lg_cur, 0);
        }
        self.entries.fill(0);
        self.num_entries = 0;
        self.theta = init_theta;
        self.lg_cur_size = init_lg_cur;
    }

    /// Get number of entries
    pub fn num_entries(&self) -> usize {
        self.num_entries
    }

    /// Get theta
    pub fn theta(&self) -> u64 {
        self.theta
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.num_entries == 0
    }

    /// Get iterator over entries
    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        self.entries.iter().copied().filter(|&e| e != 0)
    }

    /// Get log2 of nominal size
    pub fn lg_nom_size(&self) -> u8 {
        self.lg_nom_size
    }

    /// Get stride for hash table probing
    fn get_stride(key: u64, lg_size: u8) -> usize {
        (2 * ((key >> (lg_size)) & STRIDE_MASK) + 1) as usize
    }
}

/// Compute initial lg_size for hash table based on target lg_size, minimum lg_size, and resize
/// factor. Make sure `lg_target = lg_init + n * lg_resize_factor`, where `n` is an integer and
/// `lg_init >= lg_min`
fn starting_sub_multiple(lg_target: u8, lg_min: u8, lg_resize_factor: u8) -> u8 {
    if lg_target <= lg_min {
        lg_min
    } else if lg_resize_factor == 0 {
        lg_target
    } else {
        ((lg_target - lg_min) % lg_resize_factor) + lg_min
    }
}

/// Compute initial theta for hash table based on sampling probability.
fn starting_theta_from_sampling_probability(sampling_probability: f32) -> u64 {
    if sampling_probability < 1.0 {
        (MAX_THETA as f64 * sampling_probability as f64) as u64
    } else {
        MAX_THETA
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::DEFAULT_UPDATE_SEED;

    #[test]
    fn test_new_hash_table() {
        let table = ThetaHashTable::new(8, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        assert_eq!(
            table.lg_cur_size,
            starting_sub_multiple(8 + 1, MIN_LG_K, ResizeFactor::X8.lg_value())
        );
        assert_eq!(table.theta, starting_theta_from_sampling_probability(1.0));
        assert_eq!(table.num_entries(), 0);
        assert!(table.is_empty());
        assert_eq!(table.iter().count(), 0);
    }

    #[test]
    fn test_hash_and_screen() {
        let mut table = ThetaHashTable::new(8, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        // With MAX_THETA, all hashes should pass
        let hash1 = table.hash_and_screen("test1");
        let hash2 = table.hash_and_screen("test2");
        assert_ne!(hash1, 0);
        assert_ne!(hash2, 0);
        assert_ne!(hash1, hash2);

        // With low theta, some hashes should be filtered
        table.theta = 1;
        let hash3 = table.hash_and_screen("test3");
        assert_eq!(hash3, 0);
    }

    #[test]
    fn test_try_insert() {
        let mut table = ThetaHashTable::new(5, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        // Insert a hash value
        let hash = table.hash_and_screen("test_value");
        assert_ne!(hash, 0);
        assert!(table.try_insert(hash));
        assert_eq!(table.num_entries(), 1);
        assert!(!table.is_empty());

        // Try to insert the same hash again (should fail)
        assert!(!table.try_insert(hash));
        assert_eq!(table.num_entries(), 1);

        // Try to insert 0 (should fail)
        assert!(!table.try_insert(0));
        assert_eq!(table.num_entries(), 1);
    }

    #[test]
    fn test_insert_multiple_values() {
        let mut table = ThetaHashTable::new(8, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        // Insert multiple distinct values
        let mut inserted_count = 0;
        for i in 0..10 {
            let hash = table.hash_and_screen(format!("value_{}", i));
            if hash != 0 && table.try_insert(hash) {
                inserted_count += 1;
            }
        }

        assert_eq!(table.num_entries(), inserted_count);
        assert!(!table.is_empty());
        assert_eq!(table.iter().count(), inserted_count);
    }

    #[test]
    fn test_resize() {
        fn populate_values(table: &mut ThetaHashTable, count: usize) -> usize {
            let mut inserted = 0;
            for i in 0..count {
                let hash = table.hash_and_screen(format!("value_{}", i));
                if hash != 0 && table.try_insert(hash) {
                    inserted += 1;
                }
            }
            inserted
        }

        {
            let mut table = ThetaHashTable::new(8, ResizeFactor::X2, 1.0, DEFAULT_UPDATE_SEED);

            assert_eq!(table.entries.len(), 32);

            // Insert enough values to trigger resize (50% threshold)
            // Capacity = 32 * 0.5 = 16
            let inserted = populate_values(&mut table, 20);

            // Table should have resized and all values should be inserted
            assert!(table.num_entries() > 0);
            assert_eq!(table.num_entries(), inserted);
            assert_eq!(table.entries.len(), 64);
        }

        // Test different resize factors
        {
            let mut table = ThetaHashTable::new(8, ResizeFactor::X4, 1.0, DEFAULT_UPDATE_SEED);

            assert_eq!(table.entries.len(), 32);

            // Insert enough values to trigger resize (50% threshold)
            // Capacity = 32 * 0.5 = 16
            let inserted = populate_values(&mut table, 20);

            // Table should have resized and all values should be inserted
            assert!(table.num_entries() > 0);
            assert_eq!(table.num_entries(), inserted);
            assert_eq!(table.entries.len(), 128);
        }
    }

    #[test]
    fn test_rebuild() {
        let mut table = ThetaHashTable::new(5, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        assert_eq!(table.lg_cur_size, 6);
        assert_eq!(table.entries.len(), 64);
        assert_eq!(table.theta, MAX_THETA);

        // Insert many values to trigger rebuild
        for i in 0..100 {
            let hash = table.hash_and_screen(format!("value_{}", i));
            if hash != 0 {
                table.try_insert(hash);
            }
        }

        // After rebuild, theta should be reduced (rebuild is called automatically during insert)
        let new_theta = table.theta();
        assert!(
            new_theta < MAX_THETA,
            "Theta should be reduced after rebuild"
        );

        // Continue to insert values to trigger rebuild again
        for i in 100..200 {
            let hash = table.hash_and_screen(format!("value_{}", i));
            if hash != 0 {
                table.try_insert(hash);
            }
        }

        assert_eq!(table.lg_cur_size, 6);
        assert!(table.entries.len() >= 64);
        assert!(table.theta < new_theta);
    }

    #[test]
    fn test_trim() {
        let mut table = ThetaHashTable::new(5, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        // Insert more than k values
        for i in 0..100 {
            let hash = table.hash_and_screen(format!("value_{}", i));
            if hash != 0 {
                table.try_insert(hash);
            }
        }

        let before_trim = table.num_entries();
        assert!(before_trim > 32);

        table.trim();
        let after_trim = table.num_entries();
        assert!(after_trim <= 32);
        assert!(table.theta() < MAX_THETA);
    }

    #[test]
    fn test_trim_when_not_needed() {
        let mut table = ThetaHashTable::new(8, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        // Insert fewer than k values
        for i in 0..10 {
            let hash = table.hash_and_screen(format!("value_{}", i));
            if hash != 0 {
                table.try_insert(hash);
            }
        }

        let before_trim = table.num_entries();
        let before_theta = table.theta();
        table.trim();
        let after_trim = table.num_entries();

        // Should not change if already <= k
        assert_eq!(before_trim, after_trim);
        assert_eq!(before_theta, table.theta());
    }

    #[test]
    fn test_reset() {
        let mut table = ThetaHashTable::new(8, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);
        let init_theta = table.theta();
        let init_lg_cur = table.lg_cur_size;
        let init_entries = table.entries.len();

        // Insert some values
        for i in 0..10 {
            let hash = table.hash_and_screen(format!("value_{}", i));
            if hash != 0 {
                table.try_insert(hash);
            }
        }

        assert!(!table.is_empty());
        assert!(table.num_entries() > 0);

        // Reset
        table.reset();

        assert!(table.is_empty());
        assert_eq!(table.num_entries(), 0);
        assert_eq!(table.theta(), init_theta);
        assert_eq!(table.lg_cur_size, init_lg_cur);
        assert_eq!(table.entries.len(), init_entries);
        assert_eq!(table.iter().count(), 0);
    }

    #[test]
    fn test_table_with_sampling() {
        let mut table = ThetaHashTable::new(
            8,
            ResizeFactor::X8,
            0.5, // sampling_probability = 0.5
            DEFAULT_UPDATE_SEED,
        );
        assert_eq!(table.theta(), (MAX_THETA as f64 * 0.5) as u64);

        // Insert some values
        for i in 0..10 {
            let hash = table.hash_and_screen(format!("value_{}", i));
            if hash != 0 {
                table.try_insert(hash);
            }
        }

        table.reset();

        // With sampling_probability = 0.5, theta should be MAX_THETA * 0.5
        assert_eq!(table.theta(), (MAX_THETA as f64 * 0.5) as u64);
        assert!(table.is_empty());
    }

    #[test]
    fn test_iterator() {
        let mut table = ThetaHashTable::new(8, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        // Insert some values
        let mut inserted_hashes = Vec::new();
        for i in 0..10 {
            let hash = table.hash_and_screen(format!("value_{}", i));
            if hash != 0 && table.try_insert(hash) {
                inserted_hashes.push(hash);
            }
        }

        // Check iterator
        let iter_hashes: Vec<u64> = table.iter().collect();
        assert_eq!(iter_hashes.len(), table.num_entries());
        assert_eq!(iter_hashes.len(), inserted_hashes.len());

        // All inserted hashes should be in iterator
        for hash in &inserted_hashes {
            assert!(iter_hashes.contains(hash));
        }

        // Iterator should not contain 0
        assert!(!iter_hashes.contains(&0));
    }

    #[test]
    fn test_empty_table_operations() {
        let mut table = ThetaHashTable::new(8, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);

        assert!(table.is_empty());
        assert_eq!(table.num_entries(), 0);
        assert_eq!(table.iter().count(), 0);

        // Trim on empty table should not panic
        table.trim();
        assert!(table.is_empty());

        // Reset on empty table should not panic
        table.reset();
        assert!(table.is_empty());
    }

    #[test]
    fn test_rebuild_preserves_entries_less_than_kth() {
        let mut table = ThetaHashTable::new(5, ResizeFactor::X8, 1.0, DEFAULT_UPDATE_SEED);
        let k = 1u64 << 5; // k = 32

        // Insert many values to trigger rebuild
        let mut i = 0;
        let mut inserted_hashes = Vec::new();
        loop {
            let hash = table.hash_and_screen(format!("value_{}", i));
            i += 1;
            if hash != 0 {
                table.try_insert(hash);
                inserted_hashes.push(hash);
            }
            if table.num_entries() >= k as usize {
                break;
            }
        }

        let rebuild_threshold = table.get_capacity();

        loop {
            let hash = table.hash_and_screen(format!("value_{}", i));
            i += 1;
            if hash != 0 {
                table.try_insert(hash);
                inserted_hashes.push(hash);
            }
            if table.num_entries() >= rebuild_threshold {
                break;
            }
        }

        // trigger rebuild
        loop {
            let hash = table.hash_and_screen(format!("value_{}", i));
            i += 1;
            if hash != 0 {
                table.try_insert(hash);
                inserted_hashes.push(hash);
                break;
            }
        }

        // assert all entries are less than kth
        inserted_hashes.sort();
        let kth = inserted_hashes[k as usize];
        assert!(table.iter().all(|e| e < kth));
        assert_eq!(table.theta(), kth);
    }
}
