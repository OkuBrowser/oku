//! File handle management for FUSE filesystems.
//!
//! This module provides abstractions for working with file handles in FUSE (Filesystem in Userspace) implementations.
//! It offers two main types: `OwnedFileHandle` and `BorrowedFileHandle`, which provide safe wrappers around raw file handles.
//!
//! # Key Features
//! - Safe abstractions over raw file handles (u64) and file descriptors (i32).
//! - Conversion methods between file handles and file descriptors.
//! - Ownership and borrowing semantics for file handles.
//!
//! # Types
//! - [`OwnedFileHandle`]: Represents ownership of a file handle.
//! - [`BorrowedFileHandle`]: A borrowed representation of a file handle, tied to a specific lifetime.
//!
//! # Safety Considerations
//! This module includes several unsafe operations and makes certain assumptions about the validity of file handles and descriptors.
//! Users should be cautious when working with raw file descriptors and ensure that all safety requirements are met.
//!
//! Because FileHandle doesn't necessarly represent a concrete resource, no RAII is done when OwnedFileHandle is drop.
//! It is the role of the user to manipulate the resource by converting to and from OwnedFd.
//!
//! # Examples
//! ```rust
//! use std::os::fd::OwnedFd;
//! use easy_fuser::types::file_handle::{OwnedFileHandle, BorrowedFileHandle};
//!
//! // Creating an OwnedFileHandle from a raw value (unsafe)
//! let owned_handle = unsafe { OwnedFileHandle::from_raw(42) };
//!
//! // Borrowing the file handle
//! let borrowed_handle = owned_handle.borrow();
//!
//! // Converting to and from OwnedFd
//! let owned_fd = owned_handle.into_owned_fd();
//! let new_owned_handle = OwnedFileHandle::from_owned_fd(owned_fd).unwrap();
//! ```
//!
//! Note: The above example assumes the existence of a valid file handle or descriptor.
//! In real-world scenarios, ensure proper error handling and validity checks..

use std::marker::PhantomData;
pub use std::os::fd::*;

/// A wrapper around a raw file handle that represents ownership of a file handle.
/// It doesn't necessarily represent a valid file descriptor according to how fuse is implemented by the user.
/// But provide methods to work with file descriptors in a safe manner
///
/// ## Caveats
/// A file handle is represented as a u64 value, whereas a file descriptor is a i32 value.
#[derive(Debug)]
pub struct OwnedFileHandle(u64);

impl OwnedFileHandle {
    /// Creates an OwnedFileHandle from a raw u64 value.
    ///
    /// Unsafe because it assumes the provided value is a valid, open file handle.
    pub unsafe fn from_raw(handle: u64) -> Self {
        Self(handle)
    }

    /// Borrows the file handle, creating a BorrowedFileHandle with a lifetime tied to self.
    pub fn borrow(&self) -> BorrowedFileHandle<'_> {
        BorrowedFileHandle(self.0, PhantomData)
    }

    /// Borrows the file handle as a BorrowedFd.
    ///
    /// Note: This method performs an unchecked cast from `u64` to `i32`, which may lead to undefined behavior if the file handle value doesn't fit within an `i32`.
    pub fn borrow_as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.0 as i32) }
    }

    /// Attempts to convert an OwnedFd into an OwnedFileHandle.
    ///
    /// This method consumes the OwnedFd and returns an `Option<OwnedFileHandle>`.
    /// It returns None if the conversion from i32 to u64 fails, which can happen if the file descriptor is negative.
    pub fn from_owned_fd(fd: OwnedFd) -> Option<Self> {
        let raw_fd = fd.into_raw_fd().try_into().ok()?;
        Some(unsafe { Self::from_raw(raw_fd) })
    }

    /// Converts the OwnedFileHandle into an OwnedFd. Consumes self.
    ///
    /// Note: Assumes the internal u64 always represents a valid file descriptor.
    pub fn into_owned_fd(self) -> OwnedFd {
        // SAFETY: We're assuming that self.0 always contains a valid file descriptor.
        // This assumption makes this conversion safe.
        unsafe { OwnedFd::from_raw_fd(self.0 as i32) }
    }

    /// Returns the raw u64 value of the file handle.
    ///
    /// Note: This struct provides a safe abstraction over raw file handles,
    /// but some methods rely on assumptions about the validity of the internal u64 value.
    /// Use with caution when interfacing with raw file descriptors.
    pub fn as_raw(&self) -> u64 {
        self.0
    }
}

/// A borrowed representation of a file handle, tied to a specific lifetime 'a. It wraps a u64 value representing the file handle.
#[derive(Debug, Clone, Copy)]
pub struct BorrowedFileHandle<'a>(u64, PhantomData<&'a ()>);

impl<'a> BorrowedFileHandle<'a> {
    /// Retrieves the raw u64 value of the file handle.
    pub fn as_raw(&self) -> u64 {
        self.0
    }

    /// Creates an BorrowedFileHandle from a raw u64 value.
    ///
    /// Unsafe because it assumes the provided value is a valid, open file handle.
    pub unsafe fn from_raw(handle: u64) -> Self {
        Self(handle, PhantomData)
    }

    /// Converts the BorrowedFileHandle into a BorrowedFd.
    ///
    /// Note: This method performs an unchecked cast from `u64` to `i32`, which may lead to undefined behavior if the file handle value doesn't fit within an `i32`.
    pub fn as_borrowed_fd(self) -> BorrowedFd<'a> {
        unsafe { BorrowedFd::borrow_raw(self.0 as i32) }
    }

    /// Creates a BorrowedFileHandle from a OwnedFd. Don't consume the OwnedFd.
    ///
    /// Note: This method returns `None` if the conversion from `i32` to `u64` fails,
    /// which can happen with negative file descriptors.
    pub fn from_owned_fd(fd: OwnedFd) -> Option<Self> {
        let raw_fd = fd.as_raw_fd().try_into().ok()?;
        Some(BorrowedFileHandle(raw_fd, PhantomData))
    }

    /// Creates a BorrowedFileHandle from a BorrowedFd.
    ///
    /// Note: This method returns `None` if the conversion from `i32` to `u64` fails,
    /// which can happen with negative file descriptors.
    pub fn from_borrowed_fd(fd: BorrowedFd<'a>) -> Option<Self> {
        let raw_fd = fd.as_raw_fd().try_into().ok()?;
        Some(BorrowedFileHandle(raw_fd, PhantomData))
    }
}
