use js_sys::Object;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::IdbCursorWithValue;

#[cfg(feature = "futures")]
use crate::ManagedCursor;
use crate::{
    request::{DeleteStoreRequest, OpenCursorStoreRequest, UpdateStoreRequest},
    CursorDirection, Error,
};

/// Represents a cursor for traversing or iterating over multiple records in a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    inner: IdbCursorWithValue,
}

impl Cursor {
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

    /// Returns the cursor's current value. Returns an [`Error`] if the cursor is advancing or is finished.
    pub fn value(&self) -> Result<JsValue, Error> {
        self.inner.value().map_err(Error::CursorValueNotFound)
    }

    /// Returns the [`OpenCursorStoreRequest`] that was used to obtain this cursor.
    pub fn request(&self) -> OpenCursorStoreRequest {
        // This API was removed from idb 2.0 spec but will be added back in idb 3.0 spec
        #[allow(deprecated)]
        self.inner.request().into()
    }

    /// Advances the cursor through the next count records in range.
    pub fn advance(&self, count: u32) -> Result<OpenCursorStoreRequest, Error> {
        self.inner
            .advance(count)
            .map_err(Error::CursorAdvanceFailed)?;

        Ok(self.request())
    }

    /// Advances the cursor to the next record in range matching or after key (if provided).
    pub fn next(&self, key: Option<&JsValue>) -> Result<OpenCursorStoreRequest, Error> {
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
    ) -> Result<OpenCursorStoreRequest, Error> {
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

    /// Returns a managed cursor.
    #[cfg(feature = "futures")]
    #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
    pub fn into_managed(self) -> ManagedCursor {
        self.into()
    }
}

impl From<IdbCursorWithValue> for Cursor {
    fn from(inner: IdbCursorWithValue) -> Self {
        Self { inner }
    }
}

impl From<Cursor> for IdbCursorWithValue {
    fn from(cursor: Cursor) -> Self {
        cursor.inner
    }
}

impl TryFrom<JsValue> for Cursor {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbCursorWithValue>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbCursorWithValue", value))
    }
}

impl From<Cursor> for JsValue {
    fn from(cursor: Cursor) -> Self {
        cursor.inner.into()
    }
}
