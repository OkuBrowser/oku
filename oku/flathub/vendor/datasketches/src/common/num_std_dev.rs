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

//! Standard deviation enums for confidence bounds
//!
//! This module provides types for specifying confidence levels when computing
//! upper and lower bounds for sketch estimates.

#[allow(clippy::excessive_precision)]
const DELTA_OF_NUM_STD_DEVS: [f64; 4] = [
    0.5000000000000000000, // = 0.5 (1 + erf(0))
    0.1586553191586026479, // = 0.5 (1 + erf((-1/sqrt(2))))
    0.0227502618904135701, // = 0.5 (1 + erf((-2/sqrt(2))))
    0.0013498126861731796, // = 0.5 (1 + erf((-3/sqrt(2))))
];

/// Number of standard deviations for confidence bounds
///
/// This enum specifies the number of standard deviations to use when computing
/// upper and lower bounds for cardinality estimates. Higher values provide wider
/// confidence intervals with greater certainty that the true cardinality falls
/// within the bounds.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumStdDev {
    /// One standard deviation (\~68% confidence interval)
    One = 1,
    /// Two standard deviations (\~95% confidence interval)
    Two = 2,
    /// Three standard deviations (\~99.7% confidence interval)
    Three = 3,
}

impl NumStdDev {
    /// Returns the tail probability (delta) for this confidence level
    pub const fn tail_probability(&self) -> f64 {
        DELTA_OF_NUM_STD_DEVS[*self as usize]
    }
}
