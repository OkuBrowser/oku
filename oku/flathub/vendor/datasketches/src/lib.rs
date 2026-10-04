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

//! # Apache® DataSketches™ Core Rust Library Component
//!
//! The Sketching Core Library provides a range of stochastic streaming algorithms and closely
//! related Rust technologies that are particularly useful when integrating this technology into
//! systems that must deal with massive data.
//!
//! This library is divided into modules that constitute distinct groups of functionality.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

// See https://github.com/apache/datasketches-rust/issues/28 for more information.
#[cfg(target_endian = "big")]
compile_error!("datasketches does not support big-endian targets");

pub mod bloom;
pub mod common;
pub mod countmin;
pub mod error;
pub mod frequencies;
pub mod hll;
pub mod tdigest;
pub mod theta;

mod codec;
mod hash;
