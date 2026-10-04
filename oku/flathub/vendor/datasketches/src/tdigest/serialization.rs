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

pub(super) const PREAMBLE_LONGS_EMPTY_OR_SINGLE: u8 = 1;
pub(super) const PREAMBLE_LONGS_MULTIPLE: u8 = 2;
pub(super) const SERIAL_VERSION: u8 = 1;
pub(super) const TDIGEST_FAMILY_ID: u8 = 20;
pub(super) const FLAGS_IS_EMPTY: u8 = 1 << 0;
pub(super) const FLAGS_IS_SINGLE_VALUE: u8 = 1 << 1;
pub(super) const FLAGS_REVERSE_MERGE: u8 = 1 << 2;
/// the format of the reference implementation is using double (f64) precision
pub(super) const COMPAT_DOUBLE: u32 = 1;
/// the format of the reference implementation is using float (f32) precision
pub(super) const COMPAT_FLOAT: u32 = 2;
