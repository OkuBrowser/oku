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

//! T-Digest implementation for estimating quantiles and ranks.
//!
//! The implementation in this library is based on the MergingDigest described in
//! [Computing Extremely Accurate Quantiles Using t-Digests][paper] by Ted Dunning and Otmar Ertl.
//!
//! The implementation in this library has a few differences from the reference implementation
//! associated with that paper:
//!
//! * Merge does not modify the input
//! * Deserialization similar to other sketches in this library, although reading the reference
//!   implementation format is supported
//!
//! Unlike all other algorithms in the library, t-digest is empirical and has no mathematical
//! basis for estimating its error and its results are dependent on the input data. However,
//! for many common data distributions, it can produce excellent results. t-digest also operates
//! only on numeric data and, unlike the quantiles family algorithms in the library which return
//! quantile approximations from the input domain, t-digest interpolates values and will hold and
//! return data points not seen in the input.
//!
//! The closest alternative to t-digest in this library is REQ sketch. It prioritizes one chosen
//! side of the rank domain: either low rank accuracy or high rank accuracy. t-digest (in this
//! implementation) prioritizes both ends of the rank domain and has lower accuracy towards the
//! middle of the rank domain (median).
//!
//! Measurements show that t-digest is slightly biased (tends to underestimate low ranks and
//! overestimate high ranks), while still doing very well close to the extremes. The effect seems
//! to be more pronounced with more input values.
//!
//! For more information on the performance characteristics, see the
//! [Datasketches page on t-digest](https://datasketches.apache.org/docs/tdigest/tdigest.html).
//!
//! [paper]: https://arxiv.org/abs/1902.04023
//!
//! # Usage
//!
//! ```rust
//! # use datasketches::tdigest::TDigestMut;
//! let mut sketch = TDigestMut::new(100);
//! sketch.update(1.0);
//! sketch.update(2.0);
//! let median = sketch.quantile(0.5).unwrap();
//! let frozen = sketch.freeze();
//! assert!(frozen.rank(2.0).is_some());
//! ```

mod serialization;

mod sketch;
pub use self::sketch::TDigest;
pub use self::sketch::TDigestMut;
