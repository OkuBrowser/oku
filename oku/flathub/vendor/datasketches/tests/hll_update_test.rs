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

use datasketches::common::NumStdDev;
use datasketches::hll::HllSketch;
use datasketches::hll::HllType;

#[test]
fn test_basic_update() {
    let mut sketch = HllSketch::new(12, HllType::Hll8);

    // Initially empty
    assert_eq!(sketch.estimate(), 0.0);

    // Update with some values
    for i in 0..100 {
        sketch.update(i);
    }

    let estimate = sketch.estimate();
    assert!(estimate > 0.0, "Estimate should be positive after updates");
    assert!(
        (estimate - 100.0).abs() < 20.0,
        "Estimate should be reasonably close to 100, got {}",
        estimate
    );
}

#[test]
fn test_list_to_set_promotion() {
    // Use lg_k=12, which has promotion threshold ~512 for List→Set
    let mut sketch = HllSketch::new(12, HllType::Hll8);

    // Add enough unique values to trigger promotion
    for i in 0..600 {
        sketch.update(i);
    }

    let estimate = sketch.estimate();
    assert!(
        (estimate - 600.0).abs() < 100.0,
        "Estimate should be close to 600 after promotion, got {}",
        estimate
    );
}

#[test]
fn test_set_to_hll_promotion() {
    // Use lg_k=10 (K=1024), set promotes at 75% = 768
    let mut sketch = HllSketch::new(10, HllType::Hll8);

    // Add enough values to trigger List→Set→HLL promotions
    for i in 0..1000 {
        sketch.update(i);
    }

    let estimate = sketch.estimate();
    assert!(
        (estimate - 1000.0).abs() < 150.0,
        "Estimate should be close to 1000 after full promotion, got {}",
        estimate
    );
}

#[test]
fn test_duplicate_handling() {
    let mut sketch = HllSketch::new(12, HllType::Hll8);

    // Add same values multiple times
    for _ in 0..10 {
        for i in 0..100 {
            sketch.update(i);
        }
    }

    // Estimate should reflect ~100 unique values, not 1000
    let estimate = sketch.estimate();
    assert!(
        (estimate - 100.0).abs() < 20.0,
        "Duplicates should not inflate estimate, got {}",
        estimate
    );
}

#[test]
fn test_different_types() {
    let mut sketch = HllSketch::new(10, HllType::Hll8);

    // Mix different types
    sketch.update(42i32);
    sketch.update("hello");
    sketch.update(100u64);
    sketch.update(true);
    sketch.update(vec![1, 2, 3]);

    let estimate = sketch.estimate();
    assert!(estimate >= 5.0, "Should have at least 5 distinct values");
}

#[test]
fn test_hll4_type() {
    let mut sketch = HllSketch::new(12, HllType::Hll4);

    for i in 0..1000 {
        sketch.update(i);
    }

    let estimate = sketch.estimate();
    assert!(
        (estimate - 1000.0).abs() < 200.0,
        "HLL4 estimate should be reasonable, got {}",
        estimate
    );
}

#[test]
fn test_hll6_type() {
    let mut sketch = HllSketch::new(12, HllType::Hll6);

    for i in 0..1000 {
        sketch.update(i);
    }

    let estimate = sketch.estimate();
    assert!(
        (estimate - 1000.0).abs() < 200.0,
        "HLL6 estimate should be reasonable, got {}",
        estimate
    );
}

#[test]
fn test_serialization_roundtrip_after_updates() {
    let mut sketch1 = HllSketch::new(12, HllType::Hll8);

    // Add values and promote through all modes
    for i in 0..2000 {
        sketch1.update(i);
    }

    let estimate1 = sketch1.estimate();

    // Serialize and deserialize
    let bytes = sketch1.serialize();
    let sketch2 = HllSketch::deserialize(&bytes).unwrap();

    let estimate2 = sketch2.estimate();

    // Estimates should match after round-trip (allow some numerical error)
    let relative_error = (estimate1 - estimate2).abs() / estimate1;
    assert!(
        relative_error < 0.05,
        "Estimates should match after serialization (< 5% error), got {} vs {} ({:.2}% error)",
        estimate1,
        estimate2,
        relative_error * 100.0
    );
}

#[test]
fn test_large_cardinality() {
    let mut sketch = HllSketch::new(14, HllType::Hll8);

    // Add 100K unique values
    for i in 0..100_000 {
        sketch.update(i);
    }

    let estimate = sketch.estimate();
    let relative_error = (estimate - 100_000.0).abs() / 100_000.0;

    // For lg_k=14, relative error should be ~1.04%
    assert!(
        relative_error < 0.05,
        "Relative error should be < 5% for large cardinality, got {:.2}%",
        relative_error * 100.0
    );
}

#[test]
fn test_equals_method() {
    let mut sketch1 = HllSketch::new(10, HllType::Hll8);
    let mut sketch2 = HllSketch::new(10, HllType::Hll8);

    // Both start equal (empty)
    assert!(sketch1.eq(&sketch2));

    // Add same values to both
    for i in 0..100 {
        sketch1.update(i);
        sketch2.update(i);
    }

    // Should still be equal
    assert!(sketch1.eq(&sketch2));

    // Add different value to sketch2
    sketch2.update(999);

    // Now they're different
    assert!(!sketch1.eq(&sketch2));
}

#[test]
#[should_panic(expected = "lg_config_k must be in [4, 21]")]
fn test_invalid_lg_k_low() {
    HllSketch::new(3, HllType::Hll8);
}

#[test]
#[should_panic(expected = "lg_config_k must be in [4, 21]")]
fn test_invalid_lg_k_high() {
    HllSketch::new(22, HllType::Hll8);
}

#[test]
fn test_bounds_basic() {
    let mut sketch = HllSketch::new(12, HllType::Hll8);

    // Add 1000 unique values
    for i in 0..1000 {
        sketch.update(i);
    }

    let estimate = sketch.estimate();
    let upper1 = sketch.upper_bound(NumStdDev::One);
    let lower1 = sketch.lower_bound(NumStdDev::One);
    let upper2 = sketch.upper_bound(NumStdDev::Two);
    let lower2 = sketch.lower_bound(NumStdDev::Two);
    let upper3 = sketch.upper_bound(NumStdDev::Three);
    let lower3 = sketch.lower_bound(NumStdDev::Three);

    // Basic sanity checks
    assert!(lower1 <= estimate, "Lower bound should be <= estimate");
    assert!(estimate <= upper1, "Estimate should be <= upper bound");

    // Bounds should widen with more standard deviations
    assert!(lower2 <= lower1, "2-sigma lower should be <= 1-sigma lower");
    assert!(upper1 <= upper2, "1-sigma upper should be <= 2-sigma upper");
    assert!(lower3 <= lower2, "3-sigma lower should be <= 2-sigma lower");
    assert!(upper2 <= upper3, "2-sigma upper should be <= 3-sigma upper");

    // Bounds should be reasonable (within 50% for 3-sigma)
    assert!(
        lower3 > estimate * 0.5,
        "3-sigma lower bound seems too low: {} vs estimate {}",
        lower3,
        estimate
    );
    assert!(
        upper3 < estimate * 1.5,
        "3-sigma upper bound seems too high: {} vs estimate {}",
        upper3,
        estimate
    );
}

#[test]
fn test_bounds_all_modes() {
    // Test List mode (small cardinality)
    let mut sketch = HllSketch::new(12, HllType::Hll8);
    for i in 0..10 {
        sketch.update(i);
    }
    let estimate = sketch.estimate();
    let upper = sketch.upper_bound(NumStdDev::Two);
    let lower = sketch.lower_bound(NumStdDev::Two);
    assert!(
        lower <= estimate && estimate <= upper,
        "Bounds don't contain estimate in LIST mode"
    );

    // Test Set mode (medium cardinality)
    for i in 10..100 {
        sketch.update(i);
    }
    let estimate = sketch.estimate();
    let upper = sketch.upper_bound(NumStdDev::Two);
    let lower = sketch.lower_bound(NumStdDev::Two);
    assert!(
        lower <= estimate && estimate <= upper,
        "Bounds don't contain estimate in SET mode"
    );

    // Test HLL mode (large cardinality)
    for i in 100..5000 {
        sketch.update(i);
    }
    let estimate = sketch.estimate();
    let upper = sketch.upper_bound(NumStdDev::Two);
    let lower = sketch.lower_bound(NumStdDev::Two);
    assert!(
        lower <= estimate && estimate <= upper,
        "Bounds don't contain estimate in HLL mode"
    );
}

#[test]
fn test_bounds_different_lg_k() {
    // Smaller lg_k should have wider bounds (higher RSE)
    let mut sketch_small = HllSketch::new(8, HllType::Hll8); // lg_k=8, k=256
    let mut sketch_large = HllSketch::new(14, HllType::Hll8); // lg_k=14, k=16384

    for i in 0..1000 {
        sketch_small.update(i);
        sketch_large.update(i);
    }

    let est_small = sketch_small.estimate();
    let est_large = sketch_large.estimate();

    let upper_small = sketch_small.upper_bound(NumStdDev::Two);
    let lower_small = sketch_small.lower_bound(NumStdDev::Two);
    let upper_large = sketch_large.upper_bound(NumStdDev::Two);
    let lower_large = sketch_large.lower_bound(NumStdDev::Two);

    // Calculate relative width of confidence intervals
    let width_small = (upper_small - lower_small) / est_small;
    let width_large = (upper_large - lower_large) / est_large;

    // Smaller sketch should have wider relative confidence interval
    assert!(
        width_small > width_large,
        "Smaller sketch should have wider confidence interval: {} vs {}",
        width_small,
        width_large
    );
}

#[test]
fn test_bounds_empty_sketch() {
    let sketch = HllSketch::new(12, HllType::Hll8);

    let estimate = sketch.estimate();
    let upper = sketch.upper_bound(NumStdDev::Two);
    let lower = sketch.lower_bound(NumStdDev::Two);

    assert_eq!(estimate, 0.0, "Empty sketch should have 0 estimate");
    assert!(lower >= 0.0, "Lower bound should be non-negative");
    assert!(upper >= 0.0, "Upper bound should be non-negative");
    assert!(lower <= upper, "Lower bound should be <= upper bound");
}
