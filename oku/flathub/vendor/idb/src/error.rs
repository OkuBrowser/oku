use js_sys::Object;
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue};

/// Error type for [`idb-sys`](crate) crate.
#[derive(Debug, Error, PartialEq)]
pub enum Error {
    /// Failed to add a value
    #[error("failed to add a value: {}", js_object_display(.0))]
    AddFailed(JsValue),

    /// Failed to clear object store
    #[error("failed to clear object store: {}", js_object_display(.0))]
    ClearFailed(JsValue),

    /// Failed to get count of records
    #[error("failed to get count of records: {}", js_object_display(.0))]
    CountFailed(JsValue),

    /// Failed to advance cursor
    #[error("failed to advance cursor: {}", js_object_display(.0))]
    CursorAdvanceFailed(JsValue),

    /// Failed to continue cursor
    #[error("failed to continue cursor: {}", js_object_display(.0))]
    CursorContinueFailed(JsValue),

    /// Cursor is finished.
    #[error("cursor is finished")]
    CursorFinished,

    /// Failed to get cursor key
    #[error("failed to get cursor key: {}", js_object_display(.0))]
    CursorKeyNotFound(JsValue),

    /// Failed to get cursor primary key
    #[error("failed to get cursor primary key: {}", js_object_display(.0))]
    CursorPrimaryKeyNotFound(JsValue),

    /// Failed to get cursor value
    #[error("failed to get cursor value: {}", js_object_display(.0))]
    CursorValueNotFound(JsValue),

    /// Failed to delete a value
    #[error("failed to delete a value: {}", js_object_display(.0))]
    DeleteFailed(JsValue),

    /// DOM exception
    #[error("DOM exception: {}", js_object_display(.0))]
    DomException(web_sys::DomException),

    /// DOM exception not found
    #[error("DOM exception not found")]
    DomExceptionNotFound,

    /// Failed to get event target
    #[error("failed to get event target")]
    EventTargetNotFound,

    /// Failed to get all values
    #[error("failed to get all values: {}", js_object_display(.0))]
    GetAllFailed(JsValue),

    /// Failed to get all keys
    #[error("failed to get all keys: {}", js_object_display(.0))]
    GetAllKeysFailed(JsValue),

    /// Failed to get a value
    #[error("failed to get a value: {}", js_object_display(.0))]
    GetFailed(JsValue),

    /// Failed to get a key
    #[error("failed to get a key: {}", js_object_display(.0))]
    GetKeyFailed(JsValue),

    /// Failed to create new index
    #[error("failed to create new index: {}", js_object_display(.0))]
    IndexCreateFailed(JsValue),

    /// Failed to delete index
    #[error("failed to delete index: {}", js_object_display(.0))]
    IndexDeleteFailed(JsValue),

    /// Failed to delete indexed db
    #[error("failed to delete indexed db: {}", js_object_display(.0))]
    IndexedDbDeleteFailed(JsValue),

    /// Indexed db not found
    #[error("indexed db not found")]
    IndexedDbNotFound(JsValue),

    /// Failed to open indexed db
    #[error("failed to open indexed db: {}", js_object_display(.0))]
    IndexedDbOpenFailed(JsValue),

    /// Failed to get index
    #[error("failed to get index: {}", js_object_display(.0))]
    IndexNotFound(JsValue),

    /// Invalid cursor direction
    #[error("invalid cursor direction")]
    InvalidCursorDirection,

    /// Invalid key path of an object store
    #[error("invalid key path of an object store")]
    InvalidKeyPath,

    /// Invalid request ready state
    #[error("invalid request ready state")]
    InvalidReqeustReadyState,

    /// Invalid storage type
    #[error("invalid storage type")]
    InvalidStorageType,

    /// Invalid transaction mode
    #[error("invalid transaction mode")]
    InvalidTransactionMode,

    /// Failed to get key path of an object store
    #[error("failed to get key path of an object store: {}", js_object_display(.0))]
    KeyPathNotFound(JsValue),

    /// Failed to get key range bound
    #[error("failed to get key range bound: {}", js_object_display(.0))]
    KeyRangeBoundNotFound(JsValue),

    /// Failed to create key range
    #[error("failed to create key range: {}", js_object_display(.0))]
    KeyRangeCreateFailed(JsValue),

    /// Failed to check if a value is included in key range
    #[error("failed to check if a value is included in key range: {}", js_object_display(.0))]
    KeyRangeIncludesFailed(JsValue),

    /// Number conversion error
    #[error("number conversion error")]
    NumberConversionError,

    /// Failed to create new object store
    #[error("failed to create new object store: {}", js_object_display(.0))]
    ObjectStoreCreateFailed(JsValue),

    /// Failed to delete object store
    #[error("failed to delete object store: {}", js_object_display(.0))]
    ObjectStoreDeleteFailed(JsValue),

    /// Failed to get object store
    #[error("failed to get object store: {}", js_object_display(.0))]
    ObjectStoreNotFound(JsValue),

    /// Failed to open cursor
    #[error("failed to open cursor: {}", js_object_display(.0))]
    OpenCursorFailed(JsValue),

    /// Failed to open key cursor
    #[error("failed to open key cursor: {}", js_object_display(.0))]
    OpenKeyCursorFailed(JsValue),

    /// Failed to get request error
    #[error("failed to get request error: {}", js_object_display(.0))]
    RequestErrorNotFound(JsValue),

    /// Failed to get request result
    #[error("failed to get request source: {}", js_object_display(.0))]
    RequestResultNotFound(JsValue),

    /// Failed to get request result
    #[error("failed to get request source")]
    RequestSourceNotFound,

    /// Failed to abort transaction
    #[error("failed to abort transaction: {}", js_object_display(.0))]
    TransactionAbortError(JsValue),

    /// Failed to commit transaction
    #[error("failed to commit transaction: {}", js_object_display(.0))]
    TransactionCommitError(JsValue),

    /// Failed to get transaction mode
    #[error("failed to get transaction mode: {}", js_object_display(.0))]
    TransactionModeNotFound(JsValue),

    /// Failed to open new transaction
    #[error("failed to open new transaction: {}", js_object_display(.0))]
    TransactionOpenFailed(JsValue),

    /// Unexpected JS type
    #[error("unexpected JS type. expected: {}, found: {}", .0, js_object_display(.1))]
    UnexpectedJsType(&'static str, JsValue),

    /// Failed to update a value
    #[error("failed to update a value: {}", js_object_display(.0))]
    UpdateFailed(JsValue),

    /// Failed to receive object on oneshot channel
    #[cfg(feature = "futures")]
    #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
    #[error("failed to receive object on oneshot channel")]
    OneshotChannelReceiveError,

    /// No transaction associated with database request
    #[cfg(feature = "builder")]
    #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "builder")))]
    #[error("no transaction associated with database request")]
    TransactionNotFound,
}

fn js_object_display(option: &JsValue) -> String {
    if option.is_undefined() {
        "undefined".to_string()
    } else if option.is_null() {
        "null".to_string()
    } else {
        let object: &Object = option.unchecked_ref();
        ToString::to_string(&object.to_string())
    }
}
