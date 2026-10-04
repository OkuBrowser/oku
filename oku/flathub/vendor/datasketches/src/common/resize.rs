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

/// For the Families that accept this configuration parameter, it controls the size multiple that
/// affects how fast the internal cache grows, when more space is required.
///
/// For Theta Sketches, the Resize Factor is a dynamic, speed performance vs. memory size tradeoff.
/// The sketches created on-heap and configured with a Resize Factor of > X1 start out with an
/// internal hash table size that is the smallest submultiple of the target Nominal Entries
/// and larger than the minimum required hash table size for that sketch.
///
/// When the sketch needs to be resized larger, then the Resize Factor is used as a multiplier of
/// the current sketch cache array size.
///
/// "X1" means no resizing is allowed and the sketch will be initialized at full size.
///
/// "X2" means the internal cache will start very small and double in size until the target size is
/// reached.
///
/// Similarly, "X4" is a factor of 4 and "X8" is a factor of 8.
///
/// # Examples
///
/// ```
/// # use datasketches::common::ResizeFactor;
/// let factor = ResizeFactor::X4;
/// assert_eq!(factor.value(), 4);
/// assert_eq!(factor.lg_value(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeFactor {
    /// Do not resize. Sketch will be configured to full size.
    X1,
    /// Resize by factor of 2
    X2,
    /// Resize by factor of 4
    X4,
    /// Resize by factor of 8
    X8,
}

impl ResizeFactor {
    /// Returns the Log-base 2 of the Resize Factor
    pub fn lg_value(self) -> u8 {
        match self {
            ResizeFactor::X1 => 0,
            ResizeFactor::X2 => 1,
            ResizeFactor::X4 => 2,
            ResizeFactor::X8 => 3,
        }
    }

    /// Returns the Resize Factor.
    pub fn value(self) -> usize {
        // 1 << lg_value
        match self {
            ResizeFactor::X1 => 1,
            ResizeFactor::X2 => 2,
            ResizeFactor::X4 => 4,
            ResizeFactor::X8 => 8,
        }
    }
}
