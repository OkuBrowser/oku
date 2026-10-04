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

//! Error types for datasketches operations

use std::fmt;

/// ErrorKind is all kinds of Error of datasketches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The argument provided is invalid.
    InvalidArgument,
    /// The sketch data deserializing is malformed.
    InvalidData,
}

impl ErrorKind {
    /// Convert this error kind instance into static str.
    pub const fn into_static(self) -> &'static str {
        match self {
            ErrorKind::InvalidArgument => "InvalidArgument",
            ErrorKind::InvalidData => "InvalidData",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_static())
    }
}

/// Error is the error struct returned by all datasketches functions.
///
/// # Examples
///
/// ```
/// # use datasketches::error::Error;
/// # use datasketches::error::ErrorKind;
/// let err = Error::new(ErrorKind::InvalidArgument, "bad input");
/// assert_eq!(err.kind(), ErrorKind::InvalidArgument);
/// assert_eq!(err.message(), "bad input");
/// ```
pub struct Error {
    kind: ErrorKind,
    message: String,
    context: Vec<(&'static str, String)>,
}

impl Error {
    /// Create a new Error with error kind and message.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: vec![],
        }
    }

    /// Add more context in error.
    pub fn with_context(mut self, key: &'static str, value: impl ToString) -> Self {
        self.context.push((key, value.to_string()));
        self
    }

    /// Return error's kind.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Return error's message.
    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

// Convenience constructors for deserialization errors
impl Error {
    pub(crate) fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidArgument, msg)
    }

    pub(crate) fn deserial(msg: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidData, msg)
    }

    pub(crate) fn insufficient_data(msg: impl fmt::Display) -> Self {
        Self::deserial(format!("insufficient data: {msg}"))
    }

    pub(crate) fn insufficient_data_of(context: &'static str, msg: impl fmt::Display) -> Self {
        Self::deserial(format!("insufficient data ({context}): {msg}"))
    }

    pub(crate) fn invalid_family(expected: u8, actual: u8, name: &'static str) -> Self {
        Self::deserial(format!(
            "invalid family: expected {expected} ({name}), got {actual}"
        ))
    }

    pub(crate) fn unsupported_serial_version(expected: u8, actual: u8) -> Self {
        Self::deserial(format!(
            "unsupported serial version: expected {expected}, got {actual}"
        ))
    }

    pub(crate) fn invalid_preamble_longs(expected: u8, actual: u8) -> Self {
        Self::deserial(format!(
            "invalid preamble longs: expected {expected}, got {actual}"
        ))
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // If alternate has been specified, we will print like Debug.
        if f.alternate() {
            let mut de = f.debug_struct("Error");
            de.field("kind", &self.kind);
            de.field("message", &self.message);
            de.field("context", &self.context);
            return de.finish();
        }

        write!(f, "{}", self.kind)?;
        if !self.message.is_empty() {
            write!(f, " => {}", self.message)?;
        }
        writeln!(f)?;

        if !self.context.is_empty() {
            writeln!(f)?;
            writeln!(f, "Context:")?;
            for (k, v) in self.context.iter() {
                writeln!(f, "   {k}: {v}")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;

        if !self.context.is_empty() {
            write!(f, ", context: {{ ")?;
            write!(
                f,
                "{}",
                self.context
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
            write!(f, " }}")?;
        }

        if !self.message.is_empty() {
            write!(f, " => {}", self.message)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {}
