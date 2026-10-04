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

//! Cubic interpolation utilities for cardinality estimation
//!
//! Implements Lagrange cubic interpolation over lookup tables to provide
//! smooth, accurate cardinality estimates from discrete observations.

/// Interpolate Y value from X using pre-computed X/Y tables
pub fn using_x_and_y_tables(x_arr: &[f64], y_arr: &[f64], x: f64) -> f64 {
    debug_assert!(x_arr.len() >= 4 && x_arr.len() == y_arr.len());

    let last_idx = x_arr.len() - 1;
    debug_assert!(x >= x_arr[0] && x < x_arr[last_idx]);

    if x == x_arr[last_idx] {
        return y_arr[last_idx]; // corner case
    }

    let offset = find_straddle(x_arr, x);
    debug_assert!(offset < last_idx);

    // Select 4-point window based on position in array
    if offset == 0 {
        return interpolate_using_x_and_y_tables(x_arr, y_arr, offset, x);
    }

    if offset == last_idx - 1 {
        return interpolate_using_x_and_y_tables(x_arr, y_arr, offset - 2, x);
    }

    interpolate_using_x_and_y_tables(x_arr, y_arr, offset - 1, x)
}

/// Helper to perform cubic interpolation at offset using X/Y tables
fn interpolate_using_x_and_y_tables(x_arr: &[f64], y_arr: &[f64], offset: usize, x: f64) -> f64 {
    cubic_interpolate(
        x_arr[offset],
        y_arr[offset],
        x_arr[offset + 1],
        y_arr[offset + 1],
        x_arr[offset + 2],
        y_arr[offset + 2],
        x_arr[offset + 3],
        y_arr[offset + 3],
        x,
    )
}

/// Interpolate Y value from X using X array and uniform Y stride
pub fn using_x_arr_and_y_stride(x_arr: &[f64], y_stride: f64, x: f64) -> f64 {
    let len = x_arr.len();
    debug_assert!(len >= 4);

    let last_idx = len - 1;
    debug_assert!(x >= x_arr[0] && x <= x_arr[last_idx]);

    if x == x_arr[last_idx] {
        // corner case
        return y_stride * (last_idx as f64);
    }

    let offset = find_straddle(x_arr, x);
    let len_m2 = len - 2;
    debug_assert!(offset <= len_m2);

    if offset == 0 {
        // corner case
        return interpolate_using_x_arr_and_y_stride(x_arr, y_stride, offset, x);
    } else if offset == len_m2 {
        // corner case: offset - 2
        return interpolate_using_x_arr_and_y_stride(x_arr, y_stride, offset - 2, x);
    }

    // main case: offset - 1
    interpolate_using_x_arr_and_y_stride(x_arr, y_stride, offset - 1, x)
}

fn interpolate_using_x_arr_and_y_stride(
    x_arr: &[f64],
    y_stride: f64,
    offset: usize,
    x: f64,
) -> f64 {
    cubic_interpolate(
        x_arr[offset],
        y_stride * offset as f64,
        x_arr[offset + 1],
        y_stride * (offset + 1) as f64,
        x_arr[offset + 2],
        y_stride * (offset + 2) as f64,
        x_arr[offset + 3],
        y_stride * (offset + 3) as f64,
        x,
    )
}

/// Cubic interpolation using the Lagrange interpolation formula.
fn cubic_interpolate(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x: f64,
) -> f64 {
    let l0_numerator = (x - x1) * (x - x2) * (x - x3);
    let l1_numerator = (x - x0) * (x - x2) * (x - x3);
    let l2_numerator = (x - x0) * (x - x1) * (x - x3);
    let l3_numerator = (x - x0) * (x - x1) * (x - x2);

    let l0_denominator = (x0 - x1) * (x0 - x2) * (x0 - x3);
    let l1_denominator = (x1 - x0) * (x1 - x2) * (x1 - x3);
    let l2_denominator = (x2 - x0) * (x2 - x1) * (x2 - x3);
    let l3_denominator = (x3 - x0) * (x3 - x1) * (x3 - x2);

    let term0 = y0 * l0_numerator / l0_denominator;
    let term1 = y1 * l1_numerator / l1_denominator;
    let term2 = y2 * l2_numerator / l2_denominator;
    let term3 = y3 * l3_numerator / l3_denominator;

    term0 + term1 + term2 + term3
}

/// Find index `i` such that x_arr[i] <= x < x_arr[i+1].
fn find_straddle(x_arr: &[f64], x: f64) -> usize {
    debug_assert!(x_arr.len() >= 2);
    let last_idx = x_arr.len() - 1;
    debug_assert!(x >= x_arr[0] && x <= x_arr[last_idx]);

    recursive_find_straddle(x_arr, 0, last_idx, x)
}

/// Recursive helper for `find_straddle`.
fn recursive_find_straddle(x_arr: &[f64], left: usize, right: usize, x: f64) -> usize {
    debug_assert!(left < right);
    debug_assert!(x_arr[left] <= x && x < x_arr[right]); // invariant

    if left + 1 == right {
        return left;
    }

    let middle = left + (right - left) / 2;

    if x_arr[middle] <= x {
        recursive_find_straddle(x_arr, middle, right, x)
    } else {
        recursive_find_straddle(x_arr, left, middle, x)
    }
}
