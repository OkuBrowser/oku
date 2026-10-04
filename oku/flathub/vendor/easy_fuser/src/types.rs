//! Types and structures for FUSE filesystem operations.
//!
//! This module provides various type definitions and structures that are used
//! throughout the `easy_fuser` crate to represent FUSE-related concepts and data.
//!
//! # Modules
//!
//! - \[arguments\]: Defines argument types and structures for FUSE operations.
//! - \[errors\]: Contains error types and handling for FUSE operations.
//! - \[file_descriptor\]: Provides types related to file descriptors.
//! - \[file_id_type\]: Defines traits for file identification.
//! - \[flags\]: Contains flag definitions for various FUSE operations.
//! - \[inode\]: Defines the `Inode` type for representing filesystem objects.
//!
//! # Re-exports
//!
//! This module re-exports key types from its submodules for easier access, as well as
//! some types from the `fuser` crate that are commonly used in FUSE operations.

pub mod arguments;
pub mod errors;
pub mod file_handle;
mod file_id_type;
pub mod flags;
mod inode;

pub use self::{arguments::*, errors::*, file_handle::*, file_id_type::*, flags::*, inode::*};

pub use fuser::{FileType as FileKind, KernelConfig, TimeOrNow};
