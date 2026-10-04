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

use crate::codec::SketchBytes;
use crate::codec::SketchSlice;
use crate::error::Error;

/// Family ID for frequency sketches.
pub const FAMILY_ID: u8 = 10;
/// Serialization version.
pub const SERIAL_VERSION: u8 = 1;

/// Preamble longs for empty sketch.
pub const PREAMBLE_LONGS_EMPTY: u8 = 1;
/// Preamble longs for non-empty sketch.
pub const PREAMBLE_LONGS_NONEMPTY: u8 = 4;

/// Empty flag mask (both bits for compatibility).
pub const EMPTY_FLAG_MASK: u8 = 5;

pub(crate) fn count_string_items_bytes(items: &[String]) -> usize {
    items.iter().map(|item| 4 + item.len()).sum()
}

pub(crate) fn serialize_string_items(bytes: &mut SketchBytes, items: &[String]) {
    for item in items {
        let bs = item.as_bytes();
        bytes.write_u32_le(bs.len() as u32);
        bytes.write(bs);
    }
}

pub(crate) fn deserialize_string_items(
    mut cursor: SketchSlice<'_>,
    num_items: usize,
) -> Result<Vec<String>, Error> {
    let mut items = Vec::with_capacity(num_items);
    for i in 0..num_items {
        let len = cursor.read_u32_le().map_err(|_| {
            Error::insufficient_data(format!(
                "expected {num_items} string items, failed to read len at index {i}"
            ))
        })?;

        let mut slice = vec![0; len as usize];
        cursor.read_exact(&mut slice).map_err(|_| {
            Error::insufficient_data(format!(
                "expected {num_items} string items, failed to read slice at index {i}"
            ))
        })?;

        let value = String::from_utf8(slice)
            .map_err(|_| Error::deserial(format!("invalid UTF-8 string payload at index {i}")))?;
        items.push(value);
    }
    Ok(items)
}

pub(crate) fn count_i64_items_bytes(items: &[i64]) -> usize {
    items.len() * 8
}

pub(crate) fn serialize_i64_items(bytes: &mut SketchBytes, items: &[i64]) {
    for item in items.iter().copied() {
        bytes.write_i64_le(item);
    }
}

pub(crate) fn deserialize_i64_items(
    mut cursor: SketchSlice<'_>,
    num_items: usize,
) -> Result<Vec<i64>, Error> {
    let mut items = Vec::with_capacity(num_items);
    for i in 0..num_items {
        let value = cursor.read_i64_le().map_err(|_| {
            Error::insufficient_data(format!(
                "expected {num_items} i64 items, failed at index {i}"
            ))
        })?;
        items.push(value);
    }
    Ok(items)
}
