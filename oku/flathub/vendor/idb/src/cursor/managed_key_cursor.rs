use js_sys::Object;
use wasm_bindgen::JsValue;

use crate::{CursorDirection, Error, KeyCursor};

/// A key cursor that is managed by the library (for ease of use).
#[cfg(feature = "futures")]
#[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedKeyCursor {
    inner: Option<KeyCursor>,
}

impl ManagedKeyCursor {
    /// Returns the [`ObjectStore`](crate::ObjectStore) or [`Index`](crate::Index) the cursor was opened from.
    // TODO: make return type as enum: (IDBObjectStore or IDBIndex)
    pub fn source(&self) -> Option<Object> {
        self.inner.as_ref().map(|cursor| cursor.source())
    }

    /// Returns the direction of the cursor.
    pub fn direction(&self) -> Result<Option<CursorDirection>, Error> {
        self.inner
            .as_ref()
            .map(|cursor| cursor.direction())
            .transpose()
    }

    /// Returns the key of the cursor. Returns an [`Error`] if the cursor is advancing or is finished.
    pub fn key(&self) -> Result<Option<JsValue>, Error> {
        self.inner.as_ref().map(|cursor| cursor.key()).transpose()
    }

    /// Returns the effective key of the cursor. Returns an [`Error`] if the cursor is advancing or is finished.
    pub fn primary_key(&self) -> Result<Option<JsValue>, Error> {
        self.inner
            .as_ref()
            .map(|cursor| cursor.primary_key())
            .transpose()
    }

    /// Advances the cursor through the next count records in range.
    pub async fn advance(&mut self, count: u32) -> Result<(), Error> {
        let new_inner = {
            let inner = self.inner.as_ref().ok_or(Error::CursorFinished)?;
            inner.advance(count)?.await?
        };

        self.inner = new_inner;

        Ok(())
    }

    /// Advances the cursor to the next record in range matching or after key (if provided).
    pub async fn next(&mut self, key: Option<&JsValue>) -> Result<(), Error> {
        let new_inner = {
            let inner = self.inner.as_ref().ok_or(Error::CursorFinished)?;
            inner.next(key)?.await?
        };

        self.inner = new_inner;

        Ok(())
    }

    /// Advances the cursor to the next record in range matching or after key and primary key. Returns an [`Error`] if
    /// the source is not an [`Index`](crate::Index).
    pub async fn next_primary_key(
        &mut self,
        key: &JsValue,
        primary_key: &JsValue,
    ) -> Result<(), Error> {
        let new_inner = {
            let inner = self.inner.as_ref().ok_or(Error::CursorFinished)?;
            inner.next_primary_key(key, primary_key)?.await?
        };

        self.inner = new_inner;

        Ok(())
    }

    /// Updated the record pointed at by the cursor with a new value.
    pub async fn update(&self, value: &JsValue) -> Result<JsValue, Error> {
        self.inner
            .as_ref()
            .ok_or(Error::CursorFinished)?
            .update(value)?
            .await
    }

    /// Delete the record pointed at by the cursor with a new value.
    pub async fn delete(&self) -> Result<(), Error> {
        self.inner
            .as_ref()
            .ok_or(Error::CursorFinished)?
            .delete()?
            .await
    }
}

impl From<KeyCursor> for ManagedKeyCursor {
    fn from(inner: KeyCursor) -> Self {
        Self { inner: Some(inner) }
    }
}
