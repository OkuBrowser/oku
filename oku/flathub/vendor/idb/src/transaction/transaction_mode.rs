use wasm_bindgen::JsValue;
use web_sys::IdbTransactionMode;

use crate::Error;

/// Specifies the transaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionMode {
    /// The transaction is only allowed to read data.
    ReadOnly,
    /// The transaction is allowed to read, modify and delete data from existing object stores.
    ReadWrite,
    /// The transaction is allowed to read, modify and delete data from existing object stores, and can also create and
    /// remove object stores and indexes. This type of transaction can’t be manually created, but instead is created
    /// automatically when an `upgradeneeded` event is fired.
    VersionChange,
}

impl TryFrom<IdbTransactionMode> for TransactionMode {
    type Error = Error;

    fn try_from(value: IdbTransactionMode) -> Result<Self, Self::Error> {
        match value {
            IdbTransactionMode::Readonly => Ok(TransactionMode::ReadOnly),
            IdbTransactionMode::Readwrite => Ok(TransactionMode::ReadWrite),
            IdbTransactionMode::Versionchange => Ok(TransactionMode::VersionChange),
            _ => Err(Error::InvalidTransactionMode),
        }
    }
}

impl From<TransactionMode> for IdbTransactionMode {
    fn from(value: TransactionMode) -> Self {
        match value {
            TransactionMode::ReadOnly => IdbTransactionMode::Readonly,
            TransactionMode::ReadWrite => IdbTransactionMode::Readwrite,
            TransactionMode::VersionChange => IdbTransactionMode::Versionchange,
        }
    }
}

impl TryFrom<JsValue> for TransactionMode {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        IdbTransactionMode::from_js_value(&value)
            .ok_or(Error::InvalidCursorDirection)?
            .try_into()
    }
}

impl From<TransactionMode> for JsValue {
    fn from(direction: TransactionMode) -> Self {
        let inner: IdbTransactionMode = direction.into();
        inner.into()
    }
}
