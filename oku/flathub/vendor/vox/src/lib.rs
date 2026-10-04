#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
// #![feature(doc_auto_cfg)]
#![warn(missing_docs)]

/// Operations relevant to the build process.
pub mod builds;

/// Date and time representations.
pub mod date;

/// A template block for Markdown.
pub mod markdown_block;

/// A template block for math.
pub mod math_block;

/// Logic pertaining to individual pages.
pub mod page;

/// Errors originating during the build process.
pub mod error;

/// The interface to the Vox build system.
pub mod provider;

/// A provider of the Vox build system that reads & writes from the file system.
#[cfg(feature = "fs_provider")]
pub mod fs_provider;

/// A provider of the Vox build system that reads & writes from memory.
#[cfg(feature = "ram_provider")]
pub mod ram_provider;
