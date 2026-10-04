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

#![allow(dead_code)]

use std::io;
use std::io::Cursor;
use std::io::Read;

pub(crate) struct SketchBytes {
    bytes: Vec<u8>,
}

impl SketchBytes {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn write(&mut self, buf: &[u8]) {
        self.bytes.extend_from_slice(buf);
    }

    pub fn write_u8(&mut self, n: u8) {
        self.bytes.push(n);
    }

    pub fn write_i8(&mut self, n: i8) {
        self.bytes.push(n as u8);
    }

    pub fn write_u16_le(&mut self, n: u16) {
        self.write(&n.to_le_bytes());
    }

    pub fn write_u16_be(&mut self, n: u16) {
        self.write(&n.to_be_bytes());
    }

    pub fn write_i16_le(&mut self, n: i16) {
        self.write(&n.to_le_bytes());
    }

    pub fn write_i16_be(&mut self, n: i16) {
        self.write(&n.to_be_bytes());
    }

    pub fn write_u32_le(&mut self, n: u32) {
        self.write(&n.to_le_bytes());
    }

    pub fn write_u32_be(&mut self, n: u32) {
        self.write(&n.to_be_bytes());
    }

    pub fn write_i32_le(&mut self, n: i32) {
        self.write(&n.to_le_bytes());
    }

    pub fn write_i32_be(&mut self, n: i32) {
        self.write(&n.to_be_bytes());
    }

    pub fn write_u64_le(&mut self, n: u64) {
        self.write(&n.to_le_bytes());
    }

    pub fn write_u64_be(&mut self, n: u64) {
        self.write(&n.to_be_bytes());
    }

    pub fn write_i64_le(&mut self, n: i64) {
        self.write(&n.to_le_bytes());
    }

    pub fn write_i64_be(&mut self, n: i64) {
        self.write(&n.to_be_bytes());
    }

    pub fn write_f32_le(&mut self, n: f32) {
        self.write(&n.to_le_bytes());
    }

    pub fn write_f32_be(&mut self, n: f32) {
        self.write(&n.to_be_bytes());
    }

    pub fn write_f64_le(&mut self, n: f64) {
        self.write(&n.to_le_bytes());
    }

    pub fn write_f64_be(&mut self, n: f64) {
        self.write(&n.to_be_bytes());
    }
}

pub(crate) struct SketchSlice<'a> {
    slice: Cursor<&'a [u8]>,
}

impl SketchSlice<'_> {
    pub fn new(slice: &[u8]) -> SketchSlice<'_> {
        SketchSlice {
            slice: Cursor::new(slice),
        }
    }

    pub fn advance(&mut self, n: u64) {
        let pos = self.slice.position();
        self.slice.set_position(pos + n);
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.slice.read_exact(buf)
    }

    pub fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn read_i8(&mut self) -> io::Result<i8> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0] as i8)
    }

    pub fn read_u16_le(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn read_u16_be(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    pub fn read_i16_le(&mut self) -> io::Result<i16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    pub fn read_i16_be(&mut self) -> io::Result<i16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_be_bytes(buf))
    }

    pub fn read_u32_le(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn read_u32_be(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    pub fn read_i32_le(&mut self) -> io::Result<i32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    pub fn read_i32_be(&mut self) -> io::Result<i32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_be_bytes(buf))
    }

    pub fn read_u64_le(&mut self) -> io::Result<u64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    pub fn read_u64_be(&mut self) -> io::Result<u64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }

    pub fn read_i64_le(&mut self) -> io::Result<i64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }

    pub fn read_i64_be(&mut self) -> io::Result<i64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_be_bytes(buf))
    }

    pub fn read_f32_le(&mut self) -> io::Result<f32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    pub fn read_f32_be(&mut self) -> io::Result<f32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_be_bytes(buf))
    }

    pub fn read_f64_le(&mut self) -> io::Result<f64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }

    pub fn read_f64_be(&mut self) -> io::Result<f64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(f64::from_be_bytes(buf))
    }
}
