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

//! HyperLogLog Union Integration Tests
//!
//! These tests verify the public API behavior of HllUnion, focusing on:
//! - Basic union operations
//! - Mode transitions and mixed-mode unions
//! - Different HLL types and lg_k values
//! - Bounds and statistical properties
//! - Mathematical properties (commutativity, associativity, idempotency)
//! - Reset and reuse patterns
//!
//! This mirrors the testing strategy used in hll_update_test.rs

use datasketches::common::NumStdDev;
use datasketches::hll::HllSketch;
use datasketches::hll::HllType;
use datasketches::hll::HllUnion;

#[test]
fn test_union_basic_operations() {
    let mut union = HllUnion::new(12);

    // Empty union
    assert!(union.is_empty());
    assert_eq!(union.estimate(), 0.0);

    // Update with empty sketch should not change state
    let empty_sketch = HllSketch::new(12, HllType::Hll8);
    union.update(&empty_sketch);
    assert!(union.is_empty());

    // Create sketches with overlapping values
    let mut sketch1 = HllSketch::new(12, HllType::Hll8);
    for i in 0..500 {
        sketch1.update(i);
    }

    let mut sketch2 = HllSketch::new(12, HllType::Hll8);
    for i in 400..900 {
        sketch2.update(i);
    }

    union.update(&sketch1);
    union.update(&sketch2);

    // Should estimate ~900 unique values (0-899)
    let estimate = union.estimate();
    assert!(
        estimate > 800.0 && estimate < 1000.0,
        "Expected estimate around 900, got {}",
        estimate
    );
    assert!(!union.is_empty());

    // Adding empty sketch should not affect estimate
    let estimate_before = union.estimate();
    union.update(&empty_sketch);
    assert_eq!(estimate_before, union.estimate());

    // Test update_value with different types
    union.update_value("hello");
    union.update_value(42i32);
    union.update_value(vec![1, 2, 3]);
    assert!(union.estimate() > estimate_before);

    // Test duplicate handling - same sketch added multiple times
    let mut dup_union = HllUnion::new(12);
    let mut sketch = HllSketch::new(12, HllType::Hll8);
    for i in 0..100 {
        sketch.update(i);
    }
    for _ in 0..5 {
        dup_union.update(&sketch);
    }
    let dup_estimate = dup_union.estimate();
    assert!(
        (dup_estimate - 100.0).abs() < 20.0,
        "Duplicates should not inflate estimate, got {}",
        dup_estimate
    );
}

#[test]
fn test_union_mode_transitions() {
    let mut union = HllUnion::new(12);

    // Start with List mode (small cardinality)
    let mut sketch1 = HllSketch::new(12, HllType::Hll8);
    for i in 0..10 {
        sketch1.update(i);
    }

    let mut sketch2 = HllSketch::new(12, HllType::Hll8);
    for i in 5..15 {
        sketch2.update(i);
    }

    union.update(&sketch1);
    union.update(&sketch2);

    let estimate = union.estimate();
    assert!(
        (estimate - 15.0).abs() < 5.0,
        "List mode: expected estimate around 15, got {}",
        estimate
    );

    // Trigger Set mode promotion
    let mut sketch3 = HllSketch::new(12, HllType::Hll8);
    for i in 0..600 {
        sketch3.update(i);
    }
    union.update(&sketch3);

    let estimate = union.estimate();
    assert!(
        (estimate - 600.0).abs() < 100.0,
        "Set mode: estimate should be close to 600, got {}",
        estimate
    );

    // Trigger HLL mode promotion
    let mut sketch4 = HllSketch::new(12, HllType::Hll8);
    for i in 500..10_000 {
        sketch4.update(i);
    }
    union.update(&sketch4);

    let estimate = union.estimate();
    assert!(
        estimate > 9_000.0 && estimate < 11_000.0,
        "HLL mode: expected estimate around 10000, got {}",
        estimate
    );
}

#[test]
fn test_union_mixed_modes() {
    let mut union = HllUnion::new(12);

    // Small sketch (List mode)
    let mut sketch1 = HllSketch::new(12, HllType::Hll8);
    sketch1.update("a");
    sketch1.update("b");
    sketch1.update("c");

    // Large sketch (Array mode)
    let mut sketch2 = HllSketch::new(12, HllType::Hll8);
    for i in 0..10_000 {
        sketch2.update(i);
    }

    union.update(&sketch1);
    union.update(&sketch2);

    let result = union.get_result(HllType::Hll8);
    let estimate = result.estimate();

    // Should estimate ~10,003 unique values
    assert!(
        estimate > 9_500.0 && estimate < 10_500.0,
        "Expected estimate around 10000, got {}",
        estimate
    );
}

#[test]
fn test_union_mixed_hll_types() {
    let mut union = HllUnion::new(12);

    // Mix Hll4, Hll6, and Hll8 sketches
    let mut sketch1 = HllSketch::new(12, HllType::Hll4);
    for i in 0..3_000 {
        sketch1.update(i);
    }

    let mut sketch2 = HllSketch::new(12, HllType::Hll6);
    for i in 2_000..5_000 {
        sketch2.update(i);
    }

    let mut sketch3 = HllSketch::new(12, HllType::Hll8);
    for i in 4_000..7_000 {
        sketch3.update(i);
    }

    union.update(&sketch1);
    union.update(&sketch2);
    union.update(&sketch3);

    // Test getting result in different types
    let result4 = union.get_result(HllType::Hll4);
    let result6 = union.get_result(HllType::Hll6);
    let result8 = union.get_result(HllType::Hll8);

    assert_eq!(result4.target_type(), HllType::Hll4);
    assert_eq!(result6.target_type(), HllType::Hll6);
    assert_eq!(result8.target_type(), HllType::Hll8);

    // Should estimate ~7,000 unique values (0-6,999)
    for (result, type_name) in [
        (result4.estimate(), "Hll4"),
        (result6.estimate(), "Hll6"),
        (result8.estimate(), "Hll8"),
    ] {
        assert!(
            result > 6_000.0 && result < 8_000.0,
            "{}: expected estimate around 7000, got {}",
            type_name,
            result
        );
    }
}

#[test]
fn test_union_lg_k_handling() {
    // Test multiple downsizing operations: 12 → 10 → 8
    let mut union = HllUnion::new(12);

    // Start with lg_k=12
    let mut sketch1 = HllSketch::new(12, HllType::Hll8);
    for i in 0..5_000 {
        sketch1.update(i);
    }
    union.update(&sketch1);
    assert_eq!(union.lg_config_k(), 12);

    // Add sketch with lg_k=10 (triggers downsizing)
    let mut sketch2 = HllSketch::new(10, HllType::Hll8);
    for i in 4_000..8_000 {
        sketch2.update(i);
    }
    union.update(&sketch2);
    assert_eq!(union.lg_config_k(), 10, "Gadget should downsize to lg_k=10");

    // Add sketch with lg_k=8 (triggers another downsizing)
    let mut sketch3 = HllSketch::new(8, HllType::Hll8);
    for i in 7_000..10_000 {
        sketch3.update(i);
    }
    union.update(&sketch3);
    assert_eq!(union.lg_config_k(), 8, "Gadget should downsize to lg_k=8");

    let result = union.get_result(HllType::Hll8);
    let estimate = result.estimate();

    // Should estimate ~10,000 unique values (0-9,999)
    // Lower precision means higher error tolerance
    assert!(
        estimate > 8_000.0 && estimate < 12_000.0,
        "Expected estimate around 10000, got {}",
        estimate
    );

    // Test downsampling: union at lower precision than sketch
    let mut union2 = HllUnion::new(10);
    let mut sketch_high_precision = HllSketch::new(12, HllType::Hll8);
    for i in 0..5_000 {
        sketch_high_precision.update(i);
    }

    union2.update(&sketch_high_precision);
    let result2 = union2.get_result(HllType::Hll8);
    assert_eq!(result2.lg_config_k(), 10, "Result should be at lg_k=10");

    let estimate2 = result2.estimate();
    assert!(
        estimate2 > 4_000.0 && estimate2 < 6_000.0,
        "Downsampling should still estimate ~5000, got {}",
        estimate2
    );
}

#[test]
fn test_union_bounds() {
    let mut union = HllUnion::new(12);

    // Empty union
    assert_eq!(union.estimate(), 0.0);
    let empty_lower = union.lower_bound(NumStdDev::Two);
    let empty_upper = union.upper_bound(NumStdDev::Two);
    assert!(empty_lower >= 0.0, "Lower bound should be non-negative");
    assert!(empty_upper >= 0.0, "Upper bound should be non-negative");
    assert!(empty_lower <= empty_upper);

    // Add sketches
    let mut sketch1 = HllSketch::new(12, HllType::Hll8);
    for i in 0..500 {
        sketch1.update(i);
    }

    let mut sketch2 = HllSketch::new(12, HllType::Hll8);
    for i in 400..900 {
        sketch2.update(i);
    }

    union.update(&sketch1);
    union.update(&sketch2);

    let estimate = union.estimate();
    let upper1 = union.upper_bound(NumStdDev::One);
    let lower1 = union.lower_bound(NumStdDev::One);
    let upper2 = union.upper_bound(NumStdDev::Two);
    let lower2 = union.lower_bound(NumStdDev::Two);
    let upper3 = union.upper_bound(NumStdDev::Three);
    let lower3 = union.lower_bound(NumStdDev::Three);

    // Basic sanity checks
    assert!(lower1 <= estimate);
    assert!(estimate <= upper1);

    // Bounds should widen with more standard deviations
    assert!(lower2 <= lower1);
    assert!(upper1 <= upper2);
    assert!(lower3 <= lower2);
    assert!(upper2 <= upper3);

    // Bounds should be reasonable
    assert!(lower3 > estimate * 0.5);
    assert!(upper3 < estimate * 1.5);

    // Test that smaller lg_k has wider bounds (higher RSE)
    let mut union_small = HllUnion::new(8);
    let mut union_large = HllUnion::new(14);

    let mut sketch_small = HllSketch::new(8, HllType::Hll8);
    let mut sketch_large = HllSketch::new(14, HllType::Hll8);

    for i in 0..1000 {
        sketch_small.update(i);
        sketch_large.update(i);
    }

    union_small.update(&sketch_small);
    union_large.update(&sketch_large);

    let est_small = union_small.estimate();
    let est_large = union_large.estimate();

    let width_small = (union_small.upper_bound(NumStdDev::Two)
        - union_small.lower_bound(NumStdDev::Two))
        / est_small;
    let width_large = (union_large.upper_bound(NumStdDev::Two)
        - union_large.lower_bound(NumStdDev::Two))
        / est_large;

    assert!(
        width_small > width_large,
        "Smaller union should have wider confidence interval: {} vs {}",
        width_small,
        width_large
    );
}

#[test]
fn test_union_reset() {
    let mut union = HllUnion::new(12);

    let mut sketch = HllSketch::new(12, HllType::Hll8);
    for i in 0..1000 {
        sketch.update(i);
    }

    union.update(&sketch);
    assert!(!union.is_empty());
    assert!(union.estimate() > 900.0);

    // Reset should clear all state
    union.reset();
    assert!(union.is_empty());
    assert_eq!(union.estimate(), 0.0);
    assert_eq!(union.lg_config_k(), 12, "lg_max_k should be preserved");

    // Reuse after reset - multiple iterations
    for iteration in 0..3 {
        let mut sketch = HllSketch::new(12, HllType::Hll8);
        for i in (iteration * 100)..((iteration + 1) * 100) {
            sketch.update(i);
        }

        union.update(&sketch);
        assert!(!union.is_empty());

        union.reset();
        assert!(union.is_empty());
    }
}

#[test]
fn test_union_commutativity() {
    // Verify A∪B = B∪A
    let mut sketch_a = HllSketch::new(12, HllType::Hll8);
    let mut sketch_b = HllSketch::new(12, HllType::Hll8);

    for i in 0..1000 {
        sketch_a.update(i);
    }
    for i in 500..1500 {
        sketch_b.update(i);
    }

    // Union in order A, B
    let mut union1 = HllUnion::new(12);
    union1.update(&sketch_a);
    union1.update(&sketch_b);

    // Union in order B, A
    let mut union2 = HllUnion::new(12);
    union2.update(&sketch_b);
    union2.update(&sketch_a);

    let est1 = union1.estimate();
    let est2 = union2.estimate();

    let relative_diff = (est1 - est2).abs() / est1.max(est2);
    assert!(
        relative_diff < 0.001,
        "Union not commutative: {} vs {} (diff: {:.4}%)",
        est1,
        est2,
        relative_diff * 100.0
    );
}

#[test]
fn test_union_associativity() {
    // Verify (A∪B)∪C = A∪(B∪C)
    let mut sketch_a = HllSketch::new(12, HllType::Hll8);
    let mut sketch_b = HllSketch::new(12, HllType::Hll8);
    let mut sketch_c = HllSketch::new(12, HllType::Hll8);

    for i in 0..1000 {
        sketch_a.update(i);
    }
    for i in 500..1500 {
        sketch_b.update(i);
    }
    for i in 1000..2000 {
        sketch_c.update(i);
    }

    // Compute (A∪B)∪C
    let mut union1 = HllUnion::new(12);
    union1.update(&sketch_a);
    union1.update(&sketch_b);
    let ab_sketch = union1.get_result(HllType::Hll8);

    let mut union2 = HllUnion::new(12);
    union2.update(&ab_sketch);
    union2.update(&sketch_c);
    let est1 = union2.estimate();

    // Compute A∪(B∪C)
    let mut union3 = HllUnion::new(12);
    union3.update(&sketch_b);
    union3.update(&sketch_c);
    let bc_sketch = union3.get_result(HllType::Hll8);

    let mut union4 = HllUnion::new(12);
    union4.update(&sketch_a);
    union4.update(&bc_sketch);
    let est2 = union4.estimate();

    let relative_diff = (est1 - est2).abs() / est1.max(est2);
    assert!(
        relative_diff < 0.01,
        "Union not associative: {} vs {} (diff: {:.4}%)",
        est1,
        est2,
        relative_diff * 100.0
    );
}

#[test]
fn test_union_idempotency() {
    // Verify A∪A = A
    let mut sketch = HllSketch::new(12, HllType::Hll8);
    for i in 0..1000 {
        sketch.update(i);
    }

    let mut union = HllUnion::new(12);
    union.update(&sketch);
    let est1 = union.estimate();

    // Union with itself
    union.update(&sketch);
    let est2 = union.estimate();

    let relative_diff = (est1 - est2).abs() / est1;
    assert!(
        relative_diff < 0.01,
        "Union not idempotent: {} vs {} (diff: {:.4}%)",
        est1,
        est2,
        relative_diff * 100.0
    );
}

#[test]
fn test_union_large_cardinality() {
    let mut union = HllUnion::new(14);

    // Create three large sketches with overlap
    let mut sketch1 = HllSketch::new(14, HllType::Hll8);
    for i in 0..100_000 {
        sketch1.update(i);
    }

    let mut sketch2 = HllSketch::new(14, HllType::Hll8);
    for i in 50_000..150_000 {
        sketch2.update(i);
    }

    let mut sketch3 = HllSketch::new(14, HllType::Hll8);
    for i in 100_000..200_000 {
        sketch3.update(i);
    }

    union.update(&sketch1);
    union.update(&sketch2);
    union.update(&sketch3);

    let estimate = union.estimate();
    let relative_error = (estimate - 200_000.0).abs() / 200_000.0;

    // For lg_k=14, relative error should be ~1.04%
    assert!(
        relative_error < 0.05,
        "Relative error should be < 5%, got {:.2}%",
        relative_error * 100.0
    );
}

#[test]
#[should_panic(expected = "lg_max_k must be in [4, 21]")]
fn test_union_invalid_lg_k_low() {
    HllUnion::new(3);
}

#[test]
#[should_panic(expected = "lg_max_k must be in [4, 21]")]
fn test_union_invalid_lg_k_high() {
    HllUnion::new(22);
}

#[test]
fn test_union_validation() {
    // Test valid boundaries
    let union_min = HllUnion::new(4);
    let union_max = HllUnion::new(21);

    assert_eq!(union_min.lg_max_k(), 4);
    assert_eq!(union_max.lg_max_k(), 21);
    assert!(union_min.is_empty());
    assert!(union_max.is_empty());

    // Test lg_max_k is preserved
    let mut union = HllUnion::new(15);
    let mut sketch = HllSketch::new(12, HllType::Hll8);
    for i in 0..1000 {
        sketch.update(i);
    }

    union.update(&sketch);
    assert_eq!(union.lg_max_k(), 15, "lg_max_k should not change");

    union.reset();
    assert_eq!(union.lg_max_k(), 15, "lg_max_k should persist after reset");
}
