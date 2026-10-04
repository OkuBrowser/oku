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

use datasketches::countmin::CountMinSketch;

#[test]
fn test_init_defaults() {
    let sketch = CountMinSketch::new(3, 5);
    assert_eq!(sketch.num_hashes(), 3);
    assert_eq!(sketch.num_buckets(), 5);
    assert_eq!(sketch.seed(), 9001);
    assert!(sketch.is_empty());
    assert_eq!(sketch.total_weight(), 0);
    assert_eq!(sketch.estimate("missing"), 0);
}

#[test]
fn test_parameter_suggestions() {
    assert_eq!(CountMinSketch::suggest_num_buckets(0.2), 14);
    assert_eq!(CountMinSketch::suggest_num_buckets(0.1), 28);
    assert_eq!(CountMinSketch::suggest_num_buckets(0.05), 55);
    assert_eq!(CountMinSketch::suggest_num_buckets(0.01), 272);

    assert_eq!(CountMinSketch::suggest_num_hashes(0.682689492), 2);
    assert_eq!(CountMinSketch::suggest_num_hashes(0.954499736), 4);
    assert_eq!(CountMinSketch::suggest_num_hashes(0.997300204), 6);

    let buckets = CountMinSketch::suggest_num_buckets(0.1);
    let sketch = CountMinSketch::new(3, buckets);
    assert!(sketch.relative_error() <= 0.1);
}

#[test]
fn test_update_and_bounds() {
    let mut sketch = CountMinSketch::with_seed(3, 128, 123);
    sketch.update("x");
    sketch.update_with_weight("x", 9);
    assert_eq!(sketch.estimate("x"), 10);
    assert_eq!(sketch.total_weight(), 10);
    let estimate = sketch.estimate("x");
    let upper = sketch.upper_bound("x");
    let lower = sketch.lower_bound("x");
    assert!(lower <= estimate);
    assert!(estimate <= upper);
}

#[test]
fn test_negative_weights() {
    let mut sketch = CountMinSketch::with_seed(2, 32, 123);
    sketch.update_with_weight("y", -1);
    assert_eq!(sketch.total_weight(), 1);
    assert_eq!(sketch.estimate("y"), -1);
    sketch.update_with_weight("x", 2);
    assert_eq!(sketch.total_weight(), 3);
}

#[test]
fn test_merge() {
    let mut left = CountMinSketch::new(3, 64);
    let mut right = CountMinSketch::new(3, 64);
    for _ in 0..10 {
        left.update("a");
    }
    for _ in 0..4 {
        right.update("a");
        right.update("b");
    }
    left.merge(&right);
    assert_eq!(left.total_weight(), 18);
    assert!(left.estimate("a") >= 14);
    assert!(left.estimate("b") >= 4);
}

#[test]
fn test_serialize_deserialize_empty() {
    let sketch = CountMinSketch::with_seed(2, 5, 123);
    let bytes = sketch.serialize();
    let decoded = CountMinSketch::deserialize_with_seed(&bytes, 123).unwrap();
    assert!(decoded.is_empty());
    assert_eq!(decoded.num_hashes(), 2);
    assert_eq!(decoded.num_buckets(), 5);
    assert_eq!(decoded.seed(), 123);
}

#[test]
fn test_serialize_deserialize_non_empty() {
    let mut sketch = CountMinSketch::with_seed(3, 32, 123);
    for i in 0..100i64 {
        sketch.update(i);
    }
    let bytes = sketch.serialize();
    let decoded = CountMinSketch::deserialize_with_seed(&bytes, 123).unwrap();
    assert_eq!(decoded.total_weight(), sketch.total_weight());
    assert_eq!(decoded.estimate(42i64), sketch.estimate(42i64));
}

#[test]
#[should_panic(expected = "num_hashes must be at least 1")]
fn test_invalid_hashes() {
    CountMinSketch::new(0, 5);
}

#[test]
#[should_panic(expected = "num_buckets must be at least 3")]
fn test_invalid_buckets() {
    CountMinSketch::new(1, 2);
}

#[test]
#[should_panic(expected = "Incompatible sketch configuration.")]
fn test_merge_incompatible() {
    let mut left = CountMinSketch::new(3, 64);
    let right = CountMinSketch::new(2, 64);
    left.merge(&right);
}

#[test]
fn test_increment_single_key_like_rust_count_min_sketch() {
    let mut sketch = CountMinSketch::new(4, 32);
    for _ in 0..300 {
        sketch.update("key");
    }
    assert_eq!(sketch.estimate("key"), 300);
}

#[test]
fn test_increment_multi_like_rust_count_min_sketch() {
    let mut sketch = CountMinSketch::new(6, 128);
    for i in 0..1_000_000u64 {
        sketch.update(i % 100);
    }
    for key in 0..100u64 {
        assert!(sketch.estimate(key) >= 9_000);
    }
}
