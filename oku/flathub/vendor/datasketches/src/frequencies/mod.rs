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

//! Frequency sketches for finding heavy hitters in data streams.
//!
//! This module implements the Frequent Items sketch from Apache DataSketches. It tracks
//! approximate frequencies in a stream and can report heavy hitters with explicit
//! error guarantees (no false negatives or no false positives).
//!
//! For background, see the Java documentation:
//! <https://apache.github.io/datasketches-java/9.0.0/org/apache/datasketches/frequencies/FrequentItemsSketch.html>
//!
//! # Usage
//!
//! ```rust
//! # use datasketches::frequencies::ErrorType;
//! # use datasketches::frequencies::FrequentItemsSketch;
//! let mut sketch = FrequentItemsSketch::<i64>::new(64);
//! sketch.update_with_count(1, 3);
//! sketch.update(2);
//! let rows = sketch.frequent_items(ErrorType::NoFalseNegatives);
//! assert!(rows.iter().any(|row| *row.item() == 1));
//! ```
//!
//! # Serialization
//!
//! ```rust
//! # use datasketches::frequencies::FrequentItemsSketch;
//! let mut sketch = FrequentItemsSketch::<i64>::new(64);
//! sketch.update_with_count(42, 2);
//!
//! let bytes = sketch.serialize();
//! let decoded = FrequentItemsSketch::<i64>::deserialize(&bytes).unwrap();
//! assert!(decoded.estimate(&42) >= 2);
//! ```

mod reverse_purge_item_hash_map;
mod serialization;
mod sketch;

pub use self::sketch::ErrorType;
pub use self::sketch::FrequentItemsSketch;
pub use self::sketch::Row;
