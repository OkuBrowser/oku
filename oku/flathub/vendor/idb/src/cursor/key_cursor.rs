use js_sys::Object;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::IdbCursor;

use crate::{
    request::{DeleteStoreRequest, OpenKeyCursorStoreRequest, UpdateStoreRequest},
    CursorDirection, Error,
};

#[cfg(feature = "futures")]
use crate::ManagedKeyCursor;

/// Represents a key cursor for traversing or iterating over multiple records (only keys) in a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCursor {
    inner: IdbCursor,
}

impl KeyCursor {
    /// Returns the [`ObjectStore`](crate::ObjectStore) or [`Index`](crate::Index) the cursor was opened from.
    // TODO: make return type as enum: (IDBObjectStore or IDBIndex)
    pub fn source(&self) -> Object {
        self.inner.source()
    }

    /// Returns the direction of the cursor.
    pub fn direction(&self) -> Result<CursorDirection, Error> {
        self.inner.direction().try_into()
    }

    /// Returns the key of the cursor. Returns an [`Error`] if the cursor is advancing or is finished.
    pub fn key(&self) -> Result<JsValue, Error> {
        self.inner.key().map_err(Error::CursorKeyNotFound)
    }

    /// Returns the effective key of the cursor. Returns an [`Error`] if the cursor is advancing or is finished.
    pub fn primary_key(&self) -> Result<JsValue, Error> {
        self.inner
            .primary_key()
            .map_err(Error::CursorPrimaryKeyNotFound)
    }

    /// Returns the [`OpenKeyCursorStoreRequest`] that was used to obtain this cursor.
    pub fn request(&self) -> OpenKeyCursorStoreRequest {
        // This API was removed from idb 2.0 spec but will be added back in idb 3.0 spec
        #[allow(deprecated)]
        self.inner.request().into()
    }

    /// Advances the cursor through the next count records in range.
    pub fn advance(&self, count: u32) -> Result<OpenKeyCursorStoreRequest, Error> {
        self.inner
            .advance(count)
            .map_err(Error::CursorAdvanceFailed)?;

        Ok(self.request())
    }

    /// Advances the cursor to the next record in range matching or after key (if provided).
    pub fn next(&self, key: Option<&JsValue>) -> Result<OpenKeyCursorStoreRequest, Error> {
        match key {
            None => self.inner.continue_().map_err(Error::CursorContinueFailed),
            Some(key) => self
                .inner
                .continue_with_key(key)
                .map_err(Error::CursorContinueFailed),
        }?;

        Ok(self.request())
    }

    /// Advances the cursor to the next record in range matching or after key and primary key. Returns an [`Error`] if
    /// the source is not an [`Index`](crate::Index).
    pub fn next_primary_key(
        &self,
        key: &JsValue,
        primary_key: &JsValue,
    ) -> Result<OpenKeyCursorStoreRequest, Error> {
        self.inner
            .continue_primary_key(key, primary_key)
            .map_err(Error::CursorContinueFailed)?;

        Ok(self.request())
    }

    /// Updated the record pointed at by the cursor with a new value.
    pub fn update(&self, value: &JsValue) -> Result<UpdateStoreRequest, Error> {
        self.inner
            .update(value)
            .map(Into::into)
            .map_err(Error::UpdateFailed)
    }

    /// Delete the record pointed at by the cursor with a new value.
    pub fn delete(&self) -> Result<DeleteStoreRequest, Error> {
        self.inner
            .delete()
            .map(Into::into)
            .map_err(Error::DeleteFailed)
    }

    /// Returns a managed version of this cursor.
    #[cfg(feature = "futures")]
    #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
    pub fn into_managed(self) -> ManagedKeyCursor {
        self.into()
    }
}

impl From<IdbCursor> for KeyCursor {
    fn from(inner: IdbCursor) -> Self {
        Self { inner }
    }
}

impl From<KeyCursor> for IdbCursor {
    fn from(cursor: KeyCursor) -> Self {
        cursor.inner
    }
}

impl TryFrom<JsValue> for KeyCursor {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbCursor>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbCursor", value))
    }
}

impl From<KeyCursor> for JsValue {
    fn from(cursor: KeyCursor) -> Self {
        cursor.inner.into()
    }
}
