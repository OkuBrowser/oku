mod index_params;

pub use self::index_params::IndexParams;

use wasm_bindgen::{JsCast, JsValue};
use web_sys::IdbIndex;

#[cfg(feature = "builder")]
use crate::builder::IndexBuilder;
use crate::{
    request::{
        CountStoreRequest, GetAllKeysStoreRequest, GetAllStoreRequest, GetKeyStoreRequest,
        GetStoreRequest, OpenCursorStoreRequest, OpenKeyCursorStoreRequest,
    },
    CursorDirection, Error, KeyPath, ObjectStore, Query,
};

/// Provides asynchronous access to an index in a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    inner: IdbIndex,
}

impl Index {
    /// Creates a new instance of [`IndexBuilder`].
    #[cfg(feature = "builder")]
    #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "builder")))]
    pub fn builder(name: String, key_path: KeyPath) -> IndexBuilder {
        IndexBuilder::new(name, key_path)
    }

    /// Returns the name of the index.
    pub fn name(&self) -> String {
        self.inner.name()
    }

    /// Updates the name of the index.
    pub fn set_name(&self, name: &str) {
        self.inner.set_name(name)
    }

    /// Returns the [`ObjectStore`] the index belongs to.
    pub fn object_store(&self) -> ObjectStore {
        self.inner.object_store().into()
    }

    /// Returns the key path of the index.
    pub fn key_path(&self) -> Result<Option<KeyPath>, Error> {
        let inner_key_path = self.inner.key_path().map_err(Error::KeyPathNotFound)?;

        if inner_key_path.is_null() {
            Ok(None)
        } else {
            Some(inner_key_path.try_into()).transpose()
        }
    }

    /// Returns true if the index’s `multi_entry` flag is true.
    pub fn multi_entry(&self) -> bool {
        self.inner.multi_entry()
    }

    /// Returns true if the index’s `unique` flag is true.
    pub fn unique(&self) -> bool {
        self.inner.unique()
    }

    /// Retrieves the value of the first record matching the given key or key range in query.
    pub fn get(&self, query: impl Into<Query>) -> Result<GetStoreRequest, Error> {
        self.inner
            .get(&query.into().into())
            .map(Into::into)
            .map_err(Error::GetFailed)
    }

    /// Retrieves the key of the first record matching the given key or key range in query.
    pub fn get_key(&self, query: impl Into<Query>) -> Result<GetKeyStoreRequest, Error> {
        self.inner
            .get_key(&query.into().into())
            .map(Into::into)
            .map_err(Error::GetKeyFailed)
    }

    /// Retrieves the values of the records matching the given key or key range in query (up to limit if given).
    pub fn get_all(
        &self,
        query: Option<Query>,
        limit: Option<u32>,
    ) -> Result<GetAllStoreRequest, Error> {
        match (query, limit) {
            (Some(query), Some(limit)) => self
                .inner
                .get_all_with_key_and_limit(&query.into(), limit)
                .map(Into::into)
                .map_err(Error::GetAllKeysFailed),
            (Some(query), None) => self
                .inner
                .get_all_with_key(&query.into())
                .map(Into::into)
                .map_err(Error::GetAllKeysFailed),
            (None, Some(limit)) => self
                .inner
                .get_all_with_key_and_limit(&JsValue::null(), limit)
                .map(Into::into)
                .map_err(Error::GetAllKeysFailed),
            (None, None) => self
                .inner
                .get_all()
                .map(Into::into)
                .map_err(Error::GetAllKeysFailed),
        }
    }

    /// Retrieves the keys of records matching the given key or key range in query (up to limit if given).
    pub fn get_all_keys(
        &self,
        query: Option<Query>,
        limit: Option<u32>,
    ) -> Result<GetAllKeysStoreRequest, Error> {
        match (query, limit) {
            (Some(query), Some(limit)) => self
                .inner
                .get_all_keys_with_key_and_limit(&query.into(), limit)
                .map(Into::into)
                .map_err(Error::GetAllKeysFailed),
            (Some(query), None) => self
                .inner
                .get_all_keys_with_key(&query.into())
                .map(Into::into)
                .map_err(Error::GetAllKeysFailed),
            (None, Some(limit)) => self
                .inner
                .get_all_keys_with_key_and_limit(&JsValue::null(), limit)
                .map(Into::into)
                .map_err(Error::GetAllKeysFailed),
            (None, None) => self
                .inner
                .get_all_keys()
                .map(Into::into)
                .map_err(Error::GetAllKeysFailed),
        }
    }

    /// Retrieves the number of records matching the given key or key range in query.
    pub fn count(&self, query: Option<Query>) -> Result<CountStoreRequest, Error> {
        match query {
            None => self
                .inner
                .count()
                .map(Into::into)
                .map_err(Error::CountFailed),
            Some(query) => self
                .inner
                .count_with_key(&query.into())
                .map(Into::into)
                .map_err(Error::CountFailed),
        }
    }

    /// Opens a [`Cursor`](crate::Cursor) over the records matching query, ordered by direction. If query is `None`, all
    /// records in index are matched.
    pub fn open_cursor(
        &self,
        query: Option<Query>,
        cursor_direction: Option<CursorDirection>,
    ) -> Result<OpenCursorStoreRequest, Error> {
        match (query, cursor_direction) {
            (Some(query), Some(cursor_direction)) => self
                .inner
                .open_cursor_with_range_and_direction(&query.into(), cursor_direction.into())
                .map(Into::into)
                .map_err(Error::OpenCursorFailed),
            (Some(query), None) => self
                .inner
                .open_cursor_with_range(&query.into())
                .map(Into::into)
                .map_err(Error::OpenCursorFailed),
            (None, Some(cursor_direction)) => self
                .inner
                .open_cursor_with_range_and_direction(&JsValue::null(), cursor_direction.into())
                .map(Into::into)
                .map_err(Error::OpenCursorFailed),
            (None, None) => self
                .inner
                .open_cursor()
                .map(Into::into)
                .map_err(Error::OpenCursorFailed),
        }
    }

    /// Opens a [`KeyCursor`](crate::KeyCursor) over the records matching query, ordered by direction. If query is
    /// `None`, all records in index are matched.
    pub fn open_key_cursor(
        &self,
        query: Option<Query>,
        cursor_direction: Option<CursorDirection>,
    ) -> Result<OpenKeyCursorStoreRequest, Error> {
        match (query, cursor_direction) {
            (Some(query), Some(cursor_direction)) => self
                .inner
                .open_key_cursor_with_range_and_direction(&query.into(), cursor_direction.into())
                .map(Into::into)
                .map_err(Error::OpenCursorFailed),
            (Some(query), None) => self
                .inner
                .open_key_cursor_with_range(&query.into())
                .map(Into::into)
                .map_err(Error::OpenCursorFailed),
            (None, Some(cursor_direction)) => self
                .inner
                .open_key_cursor_with_range_and_direction(&JsValue::null(), cursor_direction.into())
                .map(Into::into)
                .map_err(Error::OpenCursorFailed),
            (None, None) => self
                .inner
                .open_key_cursor()
                .map(Into::into)
                .map_err(Error::OpenCursorFailed),
        }
    }
}

impl From<IdbIndex> for Index {
    fn from(inner: IdbIndex) -> Self {
        Self { inner }
    }
}

impl From<Index> for IdbIndex {
    fn from(index: Index) -> Self {
        index.inner
    }
}

impl TryFrom<JsValue> for Index {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbIndex>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbIndex", value))
    }
}

impl From<Index> for JsValue {
    fn from(value: Index) -> Self {
        value.inner.into()
    }
}
