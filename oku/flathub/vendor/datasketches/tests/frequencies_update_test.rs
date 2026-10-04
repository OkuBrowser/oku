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

use datasketches::frequencies::ErrorType;
use datasketches::frequencies::FrequentItemsSketch;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestItem(i32);

#[test]
fn test_longs_update_with_zero_count_is_noop() {
    let mut sketch: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch.update_with_count(1, 0);

    assert!(sketch.is_empty());
    assert_eq!(sketch.total_weight(), 0);
    assert_eq!(sketch.num_active_items(), 0);
}

#[test]
fn test_items_update_with_zero_count_is_noop() {
    let mut sketch = FrequentItemsSketch::new(8);
    sketch.update_with_count("a".to_string(), 0);

    assert!(sketch.is_empty());
    assert_eq!(sketch.total_weight(), 0);
    assert_eq!(sketch.num_active_items(), 0);
}

#[test]
fn test_capacity_and_epsilon_helpers() {
    let longs: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    assert_eq!(longs.current_map_capacity(), 6);
    assert_eq!(longs.maximum_map_capacity(), 6);
    assert_eq!(longs.lg_cur_map_size(), 3);
    assert_eq!(longs.lg_max_map_size(), 3);

    let epsilon = FrequentItemsSketch::<i64>::epsilon_for_lg(10);
    let expected = 3.5 / 1024.0;
    assert!((epsilon - expected).abs() < 1e-12);

    let apriori = FrequentItemsSketch::<i64>::apriori_error(10, 10_000);
    assert!((apriori - expected * 10_000.0).abs() < 1e-9);

    let items: FrequentItemsSketch<i32> = FrequentItemsSketch::new(1024);
    assert!((items.epsilon() - expected).abs() < 1e-12);
    assert_eq!(items.current_map_capacity(), 6);
    assert_eq!(items.maximum_map_capacity(), 768);
    assert_eq!(items.lg_max_map_size(), 10);
}

#[test]
fn test_longs_empty() {
    let sketch: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);

    assert!(sketch.is_empty());
    assert_eq!(sketch.num_active_items(), 0);
    assert_eq!(sketch.total_weight(), 0);
    assert_eq!(sketch.estimate(&42), 0);
    assert_eq!(sketch.lower_bound(&42), 0);
    assert_eq!(sketch.upper_bound(&42), 0);
    assert_eq!(sketch.maximum_error(), 0);
}

#[test]
fn test_items_empty() {
    let sketch: FrequentItemsSketch<String> = FrequentItemsSketch::new(8);
    let item = "a".to_string();

    assert!(sketch.is_empty());
    assert_eq!(sketch.num_active_items(), 0);
    assert_eq!(sketch.total_weight(), 0);
    assert_eq!(sketch.estimate(&item), 0);
    assert_eq!(sketch.lower_bound(&item), 0);
    assert_eq!(sketch.upper_bound(&item), 0);
    assert_eq!(sketch.maximum_error(), 0);
}

#[test]
fn test_longs_one_item() {
    let mut sketch: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch.update(10);

    assert!(!sketch.is_empty());
    assert_eq!(sketch.num_active_items(), 1);
    assert_eq!(sketch.total_weight(), 1);
    assert_eq!(sketch.estimate(&10), 1);
    assert_eq!(sketch.lower_bound(&10), 1);
    assert_eq!(sketch.upper_bound(&10), 1);
}

#[test]
fn test_items_one_item() {
    let mut sketch = FrequentItemsSketch::new(8);
    let item = "a".to_string();
    sketch.update(item.clone());

    assert!(!sketch.is_empty());
    assert_eq!(sketch.num_active_items(), 1);
    assert_eq!(sketch.total_weight(), 1);
    assert_eq!(sketch.estimate(&item), 1);
    assert_eq!(sketch.lower_bound(&item), 1);
    assert_eq!(sketch.upper_bound(&item), 1);
}

#[test]
fn test_longs_several_items_no_resize_no_purge() {
    let mut sketch: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch.update(1);
    sketch.update(2);
    sketch.update(3);
    sketch.update(4);
    sketch.update(2);
    sketch.update(3);
    sketch.update(2);

    assert!(!sketch.is_empty());
    assert_eq!(sketch.total_weight(), 7);
    assert_eq!(sketch.num_active_items(), 4);
    assert_eq!(sketch.estimate(&1), 1);
    assert_eq!(sketch.estimate(&2), 3);
    assert_eq!(sketch.estimate(&3), 2);
    assert_eq!(sketch.estimate(&4), 1);
    assert_eq!(sketch.maximum_error(), 0);
}

#[test]
fn test_items_several_items_no_resize_no_purge() {
    let mut sketch = FrequentItemsSketch::new(8);
    let a = "a".to_string();
    let b = "b".to_string();
    let c = "c".to_string();
    let d = "d".to_string();
    sketch.update(a.clone());
    sketch.update(b.clone());
    sketch.update(c.clone());
    sketch.update(d.clone());
    sketch.update(b.clone());
    sketch.update(c.clone());
    sketch.update(b.clone());

    assert!(!sketch.is_empty());
    assert_eq!(sketch.total_weight(), 7);
    assert_eq!(sketch.num_active_items(), 4);
    assert_eq!(sketch.estimate(&a), 1);
    assert_eq!(sketch.estimate(&b), 3);
    assert_eq!(sketch.estimate(&c), 2);
    assert_eq!(sketch.estimate(&d), 1);
    assert_eq!(sketch.maximum_error(), 0);

    let rows = sketch.frequent_items(ErrorType::NoFalsePositives);
    assert_eq!(rows.len(), 4);

    let rows = sketch.frequent_items_with_threshold(ErrorType::NoFalsePositives, 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].item(), &b);

    sketch.reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.num_active_items(), 0);
    assert_eq!(sketch.total_weight(), 0);
}

#[test]
fn test_items_several_items_with_resize_no_purge() {
    let mut sketch = FrequentItemsSketch::new(16);
    let a = "a".to_string();
    let b = "b".to_string();
    let c = "c".to_string();
    let d = "d".to_string();
    sketch.update(a.clone());
    sketch.update(b.clone());
    sketch.update(c.clone());
    sketch.update(d.clone());
    sketch.update(b.clone());
    sketch.update(c.clone());
    sketch.update(b.clone());
    for item in ["e", "f", "g", "h", "i", "j", "k", "l"] {
        sketch.update(item.to_string());
    }

    assert!(!sketch.is_empty());
    assert_eq!(sketch.total_weight(), 15);
    assert_eq!(sketch.num_active_items(), 12);
    assert_eq!(sketch.estimate(&a), 1);
    assert_eq!(sketch.estimate(&b), 3);
    assert_eq!(sketch.estimate(&c), 2);
    assert_eq!(sketch.estimate(&d), 1);
    assert_eq!(sketch.maximum_error(), 0);
}

#[test]
fn test_longs_estimation_mode() {
    let mut sketch: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch.update_with_count(1, 10);
    for item in 2..=6 {
        sketch.update(item);
    }
    sketch.update_with_count(7, 15);
    for item in 8..=12 {
        sketch.update(item);
    }

    assert!(!sketch.is_empty());
    assert_eq!(sketch.total_weight(), 35);
    assert!(sketch.maximum_error() > 0);

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].item(), &7);
    assert_eq!(items[0].estimate(), 15);
    assert_eq!(items[1].item(), &1);
    assert_eq!(items[1].estimate(), 10);

    let items = sketch.frequent_items(ErrorType::NoFalseNegatives);
    assert!(items.len() >= 2);
    assert!(items.len() <= 12);
}

#[test]
fn test_items_estimation_mode() {
    let mut sketch: FrequentItemsSketch<i32> = FrequentItemsSketch::new(8);
    sketch.update_with_count(1, 10);
    for item in 2..=6 {
        sketch.update(item);
    }
    sketch.update_with_count(7, 15);
    for item in 8..=12 {
        sketch.update(item);
    }

    assert!(!sketch.is_empty());
    assert_eq!(sketch.total_weight(), 35);
    assert!(sketch.maximum_error() > 0);

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].item(), &7);
    assert_eq!(items[0].estimate(), 15);
    assert_eq!(items[1].item(), &1);
    assert_eq!(items[1].estimate(), 10);

    let items = sketch.frequent_items(ErrorType::NoFalseNegatives);
    assert!(items.len() >= 2);
    assert!(items.len() <= 12);
}

#[test]
fn test_longs_purge_keeps_heavy_hitters() {
    let mut sketch: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch.update_with_count(1, 10);
    for item in 2..=7 {
        sketch.update(item);
    }

    assert_eq!(sketch.total_weight(), 16);
    assert_eq!(sketch.maximum_error(), 1);
    assert_eq!(sketch.estimate(&1), 10);
    assert_eq!(sketch.lower_bound(&1), 9);

    let rows = sketch.frequent_items(ErrorType::NoFalsePositives);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].item(), &1);
    assert_eq!(rows[0].estimate(), 10);
}

#[test]
fn test_items_purge_keeps_heavy_hitters() {
    let mut sketch = FrequentItemsSketch::new(8);
    sketch.update_with_count("a".to_string(), 10);
    for item in ["b", "c", "d", "e", "f", "g"] {
        sketch.update(item.to_string());
    }

    assert_eq!(sketch.total_weight(), 16);
    assert_eq!(sketch.maximum_error(), 1);
    assert_eq!(sketch.estimate(&"a".to_string()), 10);
    assert_eq!(sketch.lower_bound(&"a".to_string()), 9);

    let rows = sketch.frequent_items(ErrorType::NoFalsePositives);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].item(), "a");
    assert_eq!(rows[0].estimate(), 10);
}

#[test]
fn test_items_custom_type() {
    let mut sketch: FrequentItemsSketch<TestItem> = FrequentItemsSketch::new(8);
    sketch.update_with_count(TestItem(1), 10);
    for item in 2..=7 {
        sketch.update(TestItem(item));
    }
    let item = TestItem(8);
    sketch.update(item);

    assert!(!sketch.is_empty());
    assert_eq!(sketch.total_weight(), 17);
    assert_eq!(sketch.estimate(&TestItem(1)), 10);

    let rows = sketch.frequent_items(ErrorType::NoFalsePositives);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].item(), &TestItem(1));
    assert_eq!(rows[0].estimate(), 10);
}

#[test]
fn test_longs_merge_estimation_mode() {
    let mut sketch1: FrequentItemsSketch<i64> = FrequentItemsSketch::new(16);
    sketch1.update_with_count(1, 9);
    for item in 2..=14 {
        sketch1.update(item);
    }
    assert!(sketch1.maximum_error() > 0);

    let mut sketch2: FrequentItemsSketch<i64> = FrequentItemsSketch::new(16);
    for item in 8..=20 {
        sketch2.update(item);
    }
    sketch2.update_with_count(21, 11);
    assert!(sketch2.maximum_error() > 0);

    sketch1.merge(&sketch2);
    assert!(!sketch1.is_empty());
    assert_eq!(sketch1.total_weight(), 46);
    assert!(sketch1.num_active_items() >= 2);

    let items = sketch1.frequent_items_with_threshold(ErrorType::NoFalsePositives, 2);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].item(), &21);
    assert!(items[0].estimate() >= 11);
    assert_eq!(items[1].item(), &1);
    assert!(items[1].estimate() >= 9);
}

#[test]
fn test_items_merge_estimation_mode() {
    let mut sketch1: FrequentItemsSketch<i32> = FrequentItemsSketch::new(16);
    sketch1.update_with_count(1, 9);
    for item in 2..=14 {
        sketch1.update(item);
    }
    assert!(sketch1.maximum_error() > 0);

    let mut sketch2: FrequentItemsSketch<i32> = FrequentItemsSketch::new(16);
    for item in 8..=20 {
        sketch2.update(item);
    }
    sketch2.update_with_count(21, 11);
    assert!(sketch2.maximum_error() > 0);

    sketch1.merge(&sketch2);
    assert!(!sketch1.is_empty());
    assert_eq!(sketch1.total_weight(), 46);
    assert!(sketch1.num_active_items() >= 2);

    let items = sketch1.frequent_items_with_threshold(ErrorType::NoFalsePositives, 2);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].item(), &21);
    assert!(items[0].estimate() >= 11);
    assert_eq!(items[1].item(), &1);
    assert!(items[1].estimate() >= 9);
}

#[test]
fn test_longs_merge_exact_mode() {
    let mut sketch1: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch1.update(1);
    sketch1.update(2);
    sketch1.update(2);

    let mut sketch2: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch2.update(2);
    sketch2.update(3);

    sketch1.merge(&sketch2);

    assert!(!sketch1.is_empty());
    assert_eq!(sketch1.total_weight(), 5);
    assert_eq!(sketch1.num_active_items(), 3);
    assert_eq!(sketch1.estimate(&1), 1);
    assert_eq!(sketch1.estimate(&2), 3);
    assert_eq!(sketch1.estimate(&3), 1);
    assert_eq!(sketch1.maximum_error(), 0);
}

#[test]
fn test_items_merge_exact_mode() {
    let mut sketch1 = FrequentItemsSketch::new(8);
    let a = "a".to_string();
    let b = "b".to_string();
    let c = "c".to_string();
    sketch1.update(a.clone());
    sketch1.update(b.clone());
    sketch1.update(b.clone());

    let mut sketch2 = FrequentItemsSketch::new(8);
    sketch2.update(b.clone());
    sketch2.update(c.clone());

    sketch1.merge(&sketch2);

    assert!(!sketch1.is_empty());
    assert_eq!(sketch1.total_weight(), 5);
    assert_eq!(sketch1.num_active_items(), 3);
    assert_eq!(sketch1.estimate(&a), 1);
    assert_eq!(sketch1.estimate(&b), 3);
    assert_eq!(sketch1.estimate(&c), 1);
    assert_eq!(sketch1.maximum_error(), 0);
}

#[test]
fn test_longs_merge_empty_is_noop() {
    let mut sketch: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch.update(1);

    let empty: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch.merge(&empty);

    assert_eq!(sketch.total_weight(), 1);
    assert_eq!(sketch.num_active_items(), 1);
    assert_eq!(sketch.estimate(&1), 1);
}

#[test]
fn test_items_merge_empty_is_noop() {
    let mut sketch: FrequentItemsSketch<i32> = FrequentItemsSketch::new(8);
    sketch.update(1);

    let empty: FrequentItemsSketch<i32> = FrequentItemsSketch::new(8);
    sketch.merge(&empty);

    assert_eq!(sketch.total_weight(), 1);
    assert_eq!(sketch.num_active_items(), 1);
    assert_eq!(sketch.estimate(&1), 1);
}

#[test]
fn test_row_equality_changes_with_updates() {
    let mut sketch: FrequentItemsSketch<i32> = FrequentItemsSketch::new(8);
    sketch.update(1);
    let rows1 = sketch.frequent_items(ErrorType::NoFalsePositives);
    assert_eq!(rows1.len(), 1);
    let row1 = rows1[0].clone();

    sketch.update(1);
    let rows2 = sketch.frequent_items(ErrorType::NoFalsePositives);
    assert_eq!(rows2.len(), 1);
    let row2 = rows2[0].clone();

    assert_ne!(row1, row2);
    assert_eq!(row2.item(), &1);
    assert_eq!(row2.estimate(), 2);
}

#[test]
fn test_longs_reset() {
    let mut sketch: FrequentItemsSketch<i64> = FrequentItemsSketch::new(8);
    sketch.update_with_count(1, 3);
    sketch.update_with_count(2, 2);
    sketch.reset();

    assert!(sketch.is_empty());
    assert_eq!(sketch.total_weight(), 0);
    assert_eq!(sketch.num_active_items(), 0);
    assert_eq!(sketch.lg_max_map_size(), 3);
}

#[test]
#[should_panic(expected = "value must be power of 2")]
fn test_longs_invalid_map_size_panics() {
    FrequentItemsSketch::<i64>::new(6);
}

#[test]
#[should_panic(expected = "value must be power of 2")]
fn test_items_invalid_map_size_panics() {
    let _ = FrequentItemsSketch::<String>::new(6);
}
