//! Error handling types and utilities for FUSE operations.
//!
//! This module provides error types and utilities for handling errors in FUSE filesystem
//! operations. It includes definitions for POSIX errors, error kinds, and result types.
//!
//! # Key Types
//!
//! - [`PosixError`]: Represents a POSIX error with an error code and message.
//! - [`ErrorKind`]: Represents various kinds of POSIX errors.
//! - [`FuseResult`]: A type alias for `Result<T, PosixError>`.
//!
//! # Functions
//!
//! - [`PosixError::new`]: Creates a new PosixError with a given code and message.
//! - [`PosixError::last_error`]: Creates a PosixError from the last system error.
//! - [`ErrorKind::to_error`]: Converts an ErrorKind to a PosixError with a custom message.
//!

use crate::unix_fs::get_errno;
use std::any::Any;

use std::fmt::{Debug, Display};

pub type FuseResult<T> = Result<T, PosixError>;

/// Represents a POSIX error with an error code and message.
#[derive(Clone, PartialEq, Eq)]
pub struct PosixError {
    code: i32,
    pub msg: String,
}

impl PosixError {
    // Creates a new PosixError with the given code and message.
    ///
    /// # Arguments
    /// * `code` - Any type that can be converted into an i32.(The user should ensure it corresponds to a valid errno)
    /// * `msg` - Any type that can be converted to a String
    pub fn new<T, U>(code: T, msg: U) -> Self
    where
        T: Into<i32>,
        U: ToString,
    {
        Self {
            code: code.into(),
            msg: msg.to_string(),
        }
    }

    /// Creates a PosixError from the last system error.
    ///
    /// # Arguments
    /// * `msg` - Any type that can be converted to a String
    ///
    /// # Safety
    /// This function is unsafe because it accesses the errno location.
    /// It should only be used if you know that the errno location is always valid
    pub fn last_error<U>(msg: U) -> Self
    where
        U: ToString,
    {
        Self {
            code: get_errno(),
            msg: msg.to_string(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        ErrorKind::from(self.code)
    }

    pub fn io_error(&self) -> std::io::Error {
        std::io::Error::from_raw_os_error(self.code)
    }
}

impl<E> From<E> for PosixError
where
    E: std::error::Error + 'static,
{
    fn from(e: E) -> Self {
        if let Some(io_error) = (&e as &dyn Any).downcast_ref::<std::io::Error>() {
            PosixError::new(
                io_error.raw_os_error().unwrap_or(libc::EIO),
                io_error.to_string(),
            )
        } else {
            PosixError::new(libc::EIO, e.to_string())
        }
    }
}

impl Debug for PosixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PosixError")
            .field("code", &self.code)
            .field("kind", &ErrorKind::from(self.code))
            .field("msg", &self.msg)
            .finish()
    }
}

impl Display for PosixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = ErrorKind::from(self.code);
        match self.msg.as_str() {
            "" => write!(f, "{:?} (code {})", kind, self.code),
            _ => write!(f, "{:?} (code {}): {}", kind, self.code, self.msg),
        }
    }
}

/// Represents various kinds of POSIX errors.
///
/// This enum is not exhaustive and may be extended in the future to include
/// additional error kinds as needed. The `Unknown` variant is used for
/// error codes that are not explicitly listed.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ErrorKind {
    PermissionDenied,
    FileNotFound,
    NoSuchProcess,
    InterruptedSystemCall,
    InputOutputError,
    NoSuchDeviceOrAddress,
    ArgumentListTooLong,
    ExecFormatError,
    BadFileDescriptor,
    NoChildProcesses,
    ResourceDeadlockAvoided,
    OutOfMemory,
    PermissionDeniedAccess,
    BadAddress,
    BlockDeviceRequired,
    DeviceOrResourceBusy,
    FileExists,
    InvalidCrossDeviceLink,
    NoSuchDevice,
    NotADirectory,
    IsADirectory,
    InvalidArgument,
    ValueTooLarge,
    TooManyOpenFiles,
    TooManyFilesInSystem,
    InappropriateIoctlForDevice,
    TextFileBusy,
    FileTooLarge,
    NoSpaceLeftOnDevice,
    IllegalSeek,
    ReadOnlyFileSystem,
    TooManyLinks,
    BrokenPipe,
    DomainError,
    ResultTooLarge,
    ResourceUnavailableTryAgain,
    OperationInProgress,
    OperationAlreadyInProgress,
    NotASocket,
    MessageSize,
    ProtocolWrongType,
    ProtocolNotAvailable,
    ProtocolNotSupported,
    SocketTypeNotSupported,
    ProtocolFamilyNotSupported,
    AddressFamilyNotSupported,
    AddressInUse,
    AddressNotAvailable,
    NetworkDown,
    NetworkUnreachable,
    NetworkReset,
    ConnectionAborted,
    ConnectionReset,
    NoBufferSpaceAvailable,
    AlreadyConnected,
    NotConnected,
    DestinationAddressRequired,
    Shutdown,
    TooManyReferences,
    TimedOut,
    ConnectionRefused,
    TooManySymbolicLinks,
    FileNameTooLong,
    HostIsDown,
    NoRouteToHost,
    DirectoryNotEmpty,
    TooManyUsers,
    QuotaExceeded,
    StaleFileHandle,
    ObjectIsRemote,
    NoLocksAvailable,
    FunctionNotImplemented,
    NotSupported,
    IllegalByteSequence,
    BadMessage,
    IdentifierRemoved,
    MultihopAttempted,
    LinkHasBeenSevered,
    NoMessage,
    Unknown(i32),
}

impl ErrorKind {
    /// Equivalent to `PosixError::new(kind, msg)`.
    pub fn to_error<T>(self, msg: T) -> PosixError
    where
        T: ToString,
    {
        PosixError::new(i32::from(self), msg)
    }
}

impl From<i32> for ErrorKind {
    fn from(code: i32) -> Self {
        match code {
            libc::EPERM => Self::PermissionDenied,
            libc::ENOENT => Self::FileNotFound,
            libc::ESRCH => Self::NoSuchProcess,
            libc::EINTR => Self::InterruptedSystemCall,
            libc::EIO => Self::InputOutputError,
            libc::ENXIO => Self::NoSuchDeviceOrAddress,
            libc::E2BIG => Self::ArgumentListTooLong,
            libc::ENOEXEC => Self::ExecFormatError,
            libc::EBADF => Self::BadFileDescriptor,
            libc::ECHILD => Self::NoChildProcesses,
            libc::EDEADLK => Self::ResourceDeadlockAvoided,
            libc::ENOMEM => Self::OutOfMemory,
            libc::EACCES => Self::PermissionDeniedAccess,
            libc::EFAULT => Self::BadAddress,
            libc::ENOTBLK => Self::BlockDeviceRequired,
            libc::EBUSY => Self::DeviceOrResourceBusy,
            libc::EEXIST => Self::FileExists,
            libc::EXDEV => Self::InvalidCrossDeviceLink,
            libc::ENODEV => Self::NoSuchDevice,
            libc::ENOTDIR => Self::NotADirectory,
            libc::EISDIR => Self::IsADirectory,
            libc::EINVAL => Self::InvalidArgument,
            libc::EOVERFLOW => Self::ValueTooLarge,
            libc::EMFILE => Self::TooManyOpenFiles,
            libc::ENFILE => Self::TooManyFilesInSystem,
            libc::ENOTTY => Self::InappropriateIoctlForDevice,
            libc::ETXTBSY => Self::TextFileBusy,
            libc::EFBIG => Self::FileTooLarge,
            libc::ENOSPC => Self::NoSpaceLeftOnDevice,
            libc::ESPIPE => Self::IllegalSeek,
            libc::EROFS => Self::ReadOnlyFileSystem,
            libc::EMLINK => Self::TooManyLinks,
            libc::EPIPE => Self::BrokenPipe,
            libc::EDOM => Self::DomainError,
            libc::ERANGE => Self::ResultTooLarge,
            libc::EAGAIN => Self::ResourceUnavailableTryAgain,
            libc::EINPROGRESS => Self::OperationInProgress,
            libc::EALREADY => Self::OperationAlreadyInProgress,
            libc::ENOTSOCK => Self::NotASocket,
            libc::EMSGSIZE => Self::MessageSize,
            libc::EPROTOTYPE => Self::ProtocolWrongType,
            libc::ENOPROTOOPT => Self::ProtocolNotAvailable,
            libc::EPROTONOSUPPORT => Self::ProtocolNotSupported,
            libc::ESOCKTNOSUPPORT => Self::SocketTypeNotSupported,
            libc::EPFNOSUPPORT => Self::ProtocolFamilyNotSupported,
            libc::EAFNOSUPPORT => Self::AddressFamilyNotSupported,
            libc::EADDRINUSE => Self::AddressInUse,
            libc::EADDRNOTAVAIL => Self::AddressNotAvailable,
            libc::ENETDOWN => Self::NetworkDown,
            libc::ENETUNREACH => Self::NetworkUnreachable,
            libc::ENETRESET => Self::NetworkReset,
            libc::ECONNABORTED => Self::ConnectionAborted,
            libc::ECONNRESET => Self::ConnectionReset,
            libc::ENOBUFS => Self::NoBufferSpaceAvailable,
            libc::EISCONN => Self::AlreadyConnected,
            libc::ENOTCONN => Self::NotConnected,
            libc::EDESTADDRREQ => Self::DestinationAddressRequired,
            libc::ESHUTDOWN => Self::Shutdown,
            libc::ETOOMANYREFS => Self::TooManyReferences,
            libc::ETIMEDOUT => Self::TimedOut,
            libc::ECONNREFUSED => Self::ConnectionRefused,
            libc::ELOOP => Self::TooManySymbolicLinks,
            libc::ENAMETOOLONG => Self::FileNameTooLong,
            libc::EHOSTDOWN => Self::HostIsDown,
            libc::EHOSTUNREACH => Self::NoRouteToHost,
            libc::ENOTEMPTY => Self::DirectoryNotEmpty,
            libc::EUSERS => Self::TooManyUsers,
            libc::EDQUOT => Self::QuotaExceeded,
            libc::ESTALE => Self::StaleFileHandle,
            libc::EREMOTE => Self::ObjectIsRemote,
            libc::ENOLCK => Self::NoLocksAvailable,
            libc::ENOSYS => Self::FunctionNotImplemented,
            libc::ENOTSUP => Self::NotSupported,
            libc::EILSEQ => Self::IllegalByteSequence,
            libc::EBADMSG => Self::BadMessage,
            libc::EIDRM => Self::IdentifierRemoved,
            libc::EMULTIHOP => Self::MultihopAttempted,
            libc::ENOLINK => Self::LinkHasBeenSevered,
            libc::ENOMSG => Self::NoMessage,
            _ => Self::Unknown(code),
        }
    }
}

impl From<ErrorKind> for i32 {
    fn from(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::PermissionDenied => libc::EPERM,
            ErrorKind::FileNotFound => libc::ENOENT,
            ErrorKind::NoSuchProcess => libc::ESRCH,
            ErrorKind::InterruptedSystemCall => libc::EINTR,
            ErrorKind::InputOutputError => libc::EIO,
            ErrorKind::NoSuchDeviceOrAddress => libc::ENXIO,
            ErrorKind::ArgumentListTooLong => libc::E2BIG,
            ErrorKind::ExecFormatError => libc::ENOEXEC,
            ErrorKind::BadFileDescriptor => libc::EBADF,
            ErrorKind::NoChildProcesses => libc::ECHILD,
            ErrorKind::ResourceDeadlockAvoided => libc::EDEADLK,
            ErrorKind::OutOfMemory => libc::ENOMEM,
            ErrorKind::PermissionDeniedAccess => libc::EACCES,
            ErrorKind::BadAddress => libc::EFAULT,
            ErrorKind::BlockDeviceRequired => libc::ENOTBLK,
            ErrorKind::DeviceOrResourceBusy => libc::EBUSY,
            ErrorKind::FileExists => libc::EEXIST,
            ErrorKind::InvalidCrossDeviceLink => libc::EXDEV,
            ErrorKind::NoSuchDevice => libc::ENODEV,
            ErrorKind::NotADirectory => libc::ENOTDIR,
            ErrorKind::IsADirectory => libc::EISDIR,
            ErrorKind::InvalidArgument => libc::EINVAL,
            ErrorKind::ValueTooLarge => libc::EOVERFLOW,
            ErrorKind::TooManyOpenFiles => libc::EMFILE,
            ErrorKind::TooManyFilesInSystem => libc::ENFILE,
            ErrorKind::InappropriateIoctlForDevice => libc::ENOTTY,
            ErrorKind::TextFileBusy => libc::ETXTBSY,
            ErrorKind::FileTooLarge => libc::EFBIG,
            ErrorKind::NoSpaceLeftOnDevice => libc::ENOSPC,
            ErrorKind::IllegalSeek => libc::ESPIPE,
            ErrorKind::ReadOnlyFileSystem => libc::EROFS,
            ErrorKind::TooManyLinks => libc::EMLINK,
            ErrorKind::BrokenPipe => libc::EPIPE,
            ErrorKind::DomainError => libc::EDOM,
            ErrorKind::ResultTooLarge => libc::ERANGE,
            ErrorKind::ResourceUnavailableTryAgain => libc::EAGAIN,
            ErrorKind::OperationInProgress => libc::EINPROGRESS,
            ErrorKind::OperationAlreadyInProgress => libc::EALREADY,
            ErrorKind::NotASocket => libc::ENOTSOCK,
            ErrorKind::MessageSize => libc::EMSGSIZE,
            ErrorKind::ProtocolWrongType => libc::EPROTOTYPE,
            ErrorKind::ProtocolNotAvailable => libc::ENOPROTOOPT,
            ErrorKind::ProtocolNotSupported => libc::EPROTONOSUPPORT,
            ErrorKind::SocketTypeNotSupported => libc::ESOCKTNOSUPPORT,
            ErrorKind::ProtocolFamilyNotSupported => libc::EPFNOSUPPORT,
            ErrorKind::AddressFamilyNotSupported => libc::EAFNOSUPPORT,
            ErrorKind::AddressInUse => libc::EADDRINUSE,
            ErrorKind::AddressNotAvailable => libc::EADDRNOTAVAIL,
            ErrorKind::NetworkDown => libc::ENETDOWN,
            ErrorKind::NetworkUnreachable => libc::ENETUNREACH,
            ErrorKind::NetworkReset => libc::ENETRESET,
            ErrorKind::ConnectionAborted => libc::ECONNABORTED,
            ErrorKind::ConnectionReset => libc::ECONNRESET,
            ErrorKind::NoBufferSpaceAvailable => libc::ENOBUFS,
            ErrorKind::AlreadyConnected => libc::EISCONN,
            ErrorKind::NotConnected => libc::ENOTCONN,
            ErrorKind::DestinationAddressRequired => libc::EDESTADDRREQ,
            ErrorKind::Shutdown => libc::ESHUTDOWN,
            ErrorKind::TooManyReferences => libc::ETOOMANYREFS,
            ErrorKind::TimedOut => libc::ETIMEDOUT,
            ErrorKind::ConnectionRefused => libc::ECONNREFUSED,
            ErrorKind::TooManySymbolicLinks => libc::ELOOP,
            ErrorKind::FileNameTooLong => libc::ENAMETOOLONG,
            ErrorKind::HostIsDown => libc::EHOSTDOWN,
            ErrorKind::NoRouteToHost => libc::EHOSTUNREACH,
            ErrorKind::DirectoryNotEmpty => libc::ENOTEMPTY,
            ErrorKind::TooManyUsers => libc::EUSERS,
            ErrorKind::QuotaExceeded => libc::EDQUOT,
            ErrorKind::StaleFileHandle => libc::ESTALE,
            ErrorKind::ObjectIsRemote => libc::EREMOTE,
            ErrorKind::NoLocksAvailable => libc::ENOLCK,
            ErrorKind::FunctionNotImplemented => libc::ENOSYS,
            ErrorKind::NotSupported => libc::ENOTSUP,
            ErrorKind::IllegalByteSequence => libc::EILSEQ,
            ErrorKind::BadMessage => libc::EBADMSG,
            ErrorKind::IdentifierRemoved => libc::EIDRM,
            ErrorKind::MultihopAttempted => libc::EMULTIHOP,
            ErrorKind::LinkHasBeenSevered => libc::ENOLINK,
            ErrorKind::NoMessage => libc::ENOMSG,
            ErrorKind::Unknown(code) => code,
        }
    }
}

impl From<ErrorKind> for fuser::Errno {
    fn from(kind: ErrorKind) -> Self {
        fuser::Errno::from_i32(i32::from(kind))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_kind_roundtrip() {
        // List of all ErrorKind variants except Unknown
        let error_kinds = vec![
            ErrorKind::PermissionDenied,
            ErrorKind::FileNotFound,
            ErrorKind::NoSuchProcess,
            ErrorKind::InterruptedSystemCall,
            ErrorKind::InputOutputError,
            ErrorKind::NoSuchDeviceOrAddress,
            ErrorKind::ArgumentListTooLong,
            ErrorKind::ExecFormatError,
            ErrorKind::BadFileDescriptor,
            ErrorKind::NoChildProcesses,
            ErrorKind::ResourceDeadlockAvoided,
            ErrorKind::OutOfMemory,
            ErrorKind::PermissionDeniedAccess,
            ErrorKind::BadAddress,
            ErrorKind::BlockDeviceRequired,
            ErrorKind::DeviceOrResourceBusy,
            ErrorKind::FileExists,
            ErrorKind::InvalidCrossDeviceLink,
            ErrorKind::NoSuchDevice,
            ErrorKind::NotADirectory,
            ErrorKind::IsADirectory,
            ErrorKind::InvalidArgument,
            ErrorKind::ValueTooLarge,
            ErrorKind::TooManyOpenFiles,
            ErrorKind::TooManyFilesInSystem,
            ErrorKind::InappropriateIoctlForDevice,
            ErrorKind::TextFileBusy,
            ErrorKind::FileTooLarge,
            ErrorKind::NoSpaceLeftOnDevice,
            ErrorKind::IllegalSeek,
            ErrorKind::ReadOnlyFileSystem,
            ErrorKind::TooManyLinks,
            ErrorKind::BrokenPipe,
            ErrorKind::DomainError,
            ErrorKind::ResultTooLarge,
            ErrorKind::ResourceUnavailableTryAgain,
            ErrorKind::OperationInProgress,
            ErrorKind::OperationAlreadyInProgress,
            ErrorKind::NotASocket,
            ErrorKind::MessageSize,
            ErrorKind::ProtocolWrongType,
            ErrorKind::ProtocolNotAvailable,
            ErrorKind::ProtocolNotSupported,
            ErrorKind::SocketTypeNotSupported,
            ErrorKind::ProtocolFamilyNotSupported,
            ErrorKind::AddressFamilyNotSupported,
            ErrorKind::AddressInUse,
            ErrorKind::AddressNotAvailable,
            ErrorKind::NetworkDown,
            ErrorKind::NetworkUnreachable,
            ErrorKind::NetworkReset,
            ErrorKind::ConnectionAborted,
            ErrorKind::ConnectionReset,
            ErrorKind::NoBufferSpaceAvailable,
            ErrorKind::AlreadyConnected,
            ErrorKind::NotConnected,
            ErrorKind::DestinationAddressRequired,
            ErrorKind::Shutdown,
            ErrorKind::TooManyReferences,
            ErrorKind::TimedOut,
            ErrorKind::ConnectionRefused,
            ErrorKind::TooManySymbolicLinks,
            ErrorKind::FileNameTooLong,
            ErrorKind::HostIsDown,
            ErrorKind::NoRouteToHost,
            ErrorKind::DirectoryNotEmpty,
            ErrorKind::TooManyUsers,
            ErrorKind::QuotaExceeded,
            ErrorKind::StaleFileHandle,
            ErrorKind::ObjectIsRemote,
            ErrorKind::NoLocksAvailable,
            ErrorKind::FunctionNotImplemented,
            ErrorKind::NotSupported,
            ErrorKind::IllegalByteSequence,
            ErrorKind::BadMessage,
            ErrorKind::IdentifierRemoved,
            ErrorKind::MultihopAttempted,
            ErrorKind::LinkHasBeenSevered,
            ErrorKind::NoMessage,
        ];

        for kind in error_kinds {
            let code: i32 = kind.into(); // Convert ErrorKind -> i32
            let converted_kind = ErrorKind::from(code); // Convert i32 -> ErrorKind

            assert_eq!(
                kind, converted_kind,
                "Failed for ErrorKind::{:?} with code {}",
                kind, code
            );
        }
    }
}
