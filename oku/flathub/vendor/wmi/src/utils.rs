use serde::{de, ser};
use std::fmt::{Debug, Display};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WMIError {
    /// You can find a useful resource for decoding error codes [here](https://docs.microsoft.com/en-us/windows/win32/wmisdk/wmi-error-constants)
    /// (or a github version [here](https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/WmiSdk/wmi-error-constants.md))
    #[error("HRESULT Call failed with: {hres:#X}")]
    HResultError { hres: i32 },
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[cfg(feature = "chrono")]
    #[error(transparent)]
    ParseDatetimeError(#[from] chrono::format::ParseError),
    #[cfg(feature = "chrono")]
    #[error("Cannot parse a non unique local timestamp")]
    ParseDatetimeLocalError,
    #[cfg(feature = "time")]
    #[error(transparent)]
    ParseOffsetDatetimeError(#[from] time::Error),
    #[error("Converting from variant type {0:#X} is not implemented yet")]
    ConvertError(u16),
    #[error("{0}")]
    ConvertVariantError(String),
    #[error("Invalid bool value: {0:#X}")]
    ConvertBoolError(i16),
    #[error(transparent)]
    ConvertStringError(#[from] std::string::FromUtf16Error),
    #[error("Expected {0:?} to be at least 21 chars")]
    ConvertDatetimeError(String),
    #[error("Expected {0:?} to be at 25 chars")]
    ConvertDurationError(String),
    #[error("Length {0} was too long to convert")]
    ConvertLengthError(u64),
    #[error("{0}")]
    SerdeError(String),
    #[error(transparent)]
    DeserializeValueError(#[from] de::value::Error),
    #[error("No results returned")]
    ResultEmpty,
    #[error("Null pointer was sent as part of query result")]
    NullPointerResult,
    #[error("Unimplemeted array item in query")]
    UnimplementedArrayItem,
    #[error("Invalid variant {0} during deserialization")]
    InvalidDeserializationVariantError(String),
}

impl From<windows::core::Error> for WMIError {
    fn from(value: windows::core::Error) -> Self {
        Self::HResultError {
            hres: value.code().0,
        }
    }
}

impl de::Error for WMIError {
    #[cold]
    fn custom<T: Display>(msg: T) -> WMIError {
        Self::SerdeError(format!("{}", msg))
    }
}

impl ser::Error for WMIError {
    #[cold]
    fn custom<T: Display>(msg: T) -> WMIError {
        Self::SerdeError(format!("{}", msg))
    }
}

/// Alias type for `Result<T, WMIError>`
pub type WMIResult<T> = Result<T, WMIError>;
