//! This module contains all the `Future` types for handling database requests (using `async`/`await` syntax).
pub use super::{
    database::{DeleteDatabaseRequestFuture, OpenDatabaseRequestFuture},
    store::{
        AddStoreRequestFuture, ClearStoreRequestFuture, CountStoreRequestFuture,
        DeleteStoreRequestFuture, GetAllKeysStoreRequestFuture, GetAllStoreRequestFuture,
        GetKeyStoreRequestFuture, GetStoreRequestFuture, OpenCursorStoreRequestFuture,
        OpenKeyCursorStoreRequestFuture, PutStoreRequestFuture, UpdateStoreRequestFuture,
    },
};
