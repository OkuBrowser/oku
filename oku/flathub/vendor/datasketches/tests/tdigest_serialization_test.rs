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

mod common;

use std::fs;
use std::path::PathBuf;

use common::serialization_test_data;
use common::test_data;
use datasketches::tdigest::TDigestMut;
use googletest::assert_that;
use googletest::prelude::eq;
use googletest::prelude::near;

fn test_sketch_file(path: PathBuf, n: u64, with_buffer: bool, is_f32: bool) {
    let bytes = fs::read(&path).unwrap();
    let td = TDigestMut::deserialize(&bytes, is_f32).unwrap();
    let td = td.freeze();

    let path = path.display();
    if n == 0 {
        assert!(td.is_empty(), "filepath: {path}");
        assert_eq!(td.total_weight(), 0, "filepath: {path}");
    } else {
        assert!(!td.is_empty(), "filepath: {path}");
        assert_eq!(td.total_weight(), n, "filepath: {path}");
        assert_eq!(td.min_value(), Some(1.0), "filepath: {path}");
        assert_eq!(td.max_value(), Some(n as f64), "filepath: {path}");
        assert_eq!(td.rank(0.0), Some(0.0), "filepath: {path}");
        assert_eq!(td.rank((n + 1) as f64), Some(1.0), "filepath: {path}");
        if n == 1 {
            assert_eq!(td.rank(n as f64), Some(0.5), "filepath: {path}");
        } else {
            assert_that!(
                td.rank(n as f64 / 2.).unwrap(),
                near(0.5, 0.05),
                "filepath: {path}",
            );
        }
    }

    if !with_buffer && !is_f32 {
        let mut td = td.unfreeze();
        let roundtrip_bytes = td.serialize();
        assert_eq!(bytes, roundtrip_bytes, "filepath: {path}");
    }
}

#[test]
fn test_deserialize_from_cpp_snapshots() {
    let ns = [0, 1, 10, 100, 1000, 10_000, 100_000, 1_000_000];
    for n in ns {
        let filename = format!("tdigest_double_n{}_cpp.sk", n);
        let path = serialization_test_data("cpp_generated_files", &filename);
        test_sketch_file(path, n, false, false);
    }
    for n in ns {
        let filename = format!("tdigest_double_buf_n{}_cpp.sk", n);
        let path = serialization_test_data("cpp_generated_files", &filename);
        test_sketch_file(path, n, true, false);
    }
    for n in ns {
        let filename = format!("tdigest_float_n{}_cpp.sk", n);
        let path = serialization_test_data("cpp_generated_files", &filename);
        test_sketch_file(path, n, false, true);
    }
    for n in ns {
        let filename = format!("tdigest_float_buf_n{}_cpp.sk", n);
        let path = serialization_test_data("cpp_generated_files", &filename);
        test_sketch_file(path, n, true, true);
    }
}

#[test]
fn test_deserialize_from_reference_implementation() {
    for filename in [
        "tdigest_ref_k100_n10000_double.sk",
        "tdigest_ref_k100_n10000_float.sk",
    ] {
        let path = test_data(filename);
        let bytes = fs::read(&path).unwrap();
        let td = TDigestMut::deserialize(&bytes, false).unwrap();
        let td = td.freeze();

        let n = 10000;
        let path = path.display();
        assert_eq!(td.k(), 100, "filepath: {path}");
        assert_eq!(td.total_weight(), n, "filepath: {path}");
        assert_eq!(td.min_value(), Some(0.0), "filepath: {path}");
        assert_eq!(td.max_value(), Some((n - 1) as f64), "filepath: {path}");
        assert_that!(td.rank(0.0).unwrap(), near(0.0, 0.0001), "filepath: {path}");
        assert_that!(
            td.rank(n as f64 / 4.).unwrap(),
            near(0.25, 0.0001),
            "filepath: {path}"
        );
        assert_that!(
            td.rank(n as f64 / 2.).unwrap(),
            near(0.5, 0.0001),
            "filepath: {path}"
        );
        assert_that!(
            td.rank((n * 3) as f64 / 4.).unwrap(),
            near(0.75, 0.0001),
            "filepath: {path}"
        );
        assert_that!(td.rank(n as f64).unwrap(), eq(1.0), "filepath: {path}");
    }
}

#[test]
fn test_deserialize_from_java_snapshots() {
    let ns = [0, 1, 10, 100, 1000, 10_000, 100_000, 1_000_000];
    for n in ns {
        let filename = format!("tdigest_double_n{}_java.sk", n);
        let path = serialization_test_data("java_generated_files", &filename);
        test_sketch_file(path, n, false, false);
    }
}

#[test]
fn test_empty() {
    let mut td = TDigestMut::new(100);
    assert!(td.is_empty());

    let bytes = td.serialize();
    assert_eq!(bytes.len(), 8);
    let td = td.freeze();

    let deserialized_td = TDigestMut::deserialize(&bytes, false).unwrap();
    let deserialized_td = deserialized_td.freeze();
    assert_eq!(td.k(), deserialized_td.k());
    assert_eq!(td.total_weight(), deserialized_td.total_weight());
    assert!(td.is_empty());
    assert!(deserialized_td.is_empty());
}

#[test]
fn test_single_value() {
    let mut td = TDigestMut::default();
    td.update(123.0);

    let bytes = td.serialize();
    assert_eq!(bytes.len(), 16);

    let deserialized_td = TDigestMut::deserialize(&bytes, false).unwrap();
    let deserialized_td = deserialized_td.freeze();
    assert_eq!(deserialized_td.k(), 200);
    assert_eq!(deserialized_td.total_weight(), 1);
    assert!(!deserialized_td.is_empty());
    assert_eq!(deserialized_td.min_value(), Some(123.0));
    assert_eq!(deserialized_td.max_value(), Some(123.0));
}

#[test]
fn test_many_values() {
    let mut td = TDigestMut::new(100);
    for i in 0..1000 {
        td.update(i as f64);
    }

    let bytes = td.serialize();
    assert_eq!(bytes.len(), 1584);
    let td = td.freeze();

    let deserialized_td = TDigestMut::deserialize(&bytes, false).unwrap();
    let deserialized_td = deserialized_td.freeze();
    assert_eq!(td.k(), deserialized_td.k());
    assert_eq!(td.total_weight(), deserialized_td.total_weight());
    assert_eq!(td.is_empty(), deserialized_td.is_empty());
    assert_eq!(td.min_value(), deserialized_td.min_value());
    assert_eq!(td.max_value(), deserialized_td.max_value());
    assert_eq!(td.rank(500.0), deserialized_td.rank(500.0));
    assert_eq!(td.quantile(0.5), deserialized_td.quantile(0.5));
}
