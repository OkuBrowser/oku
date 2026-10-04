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

//! Count-Min sketch implementation for frequency estimation.
//!
//! The Count-Min sketch provides approximate frequency counts for streaming data
//! with configurable relative error and confidence bounds.
//!
//! # Usage
//!
//! ```rust
//! # use datasketches::countmin::CountMinSketch;
//! let mut sketch = CountMinSketch::new(5, 256);
//! sketch.update("apple");
//! sketch.update_with_weight("banana", 3);
//! assert!(sketch.estimate("banana") >= 3);
//! ```
//!
//! # Configuration Helpers
//!
//! ```rust
//! # use datasketches::countmin::CountMinSketch;
//! let buckets = CountMinSketch::suggest_num_buckets(0.01);
//! let hashes = CountMinSketch::suggest_num_hashes(0.99);
//! let _sketch = CountMinSketch::new(hashes, buckets);
//! ```

mod serialization;

mod sketch;
pub use self::sketch::CountMinSketch;
