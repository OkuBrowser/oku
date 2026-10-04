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

use crate::hll::HllType;
use crate::hll::array4::Array4;
use crate::hll::array6::Array6;
use crate::hll::array8::Array8;
use crate::hll::hash_set::HashSet;
use crate::hll::list::List;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    List { list: List, hll_type: HllType },
    Set { set: HashSet, hll_type: HllType },
    Array4(Array4),
    Array6(Array6),
    Array8(Array8),
}
