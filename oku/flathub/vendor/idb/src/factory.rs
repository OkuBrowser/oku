use js_sys::Reflect;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::IdbFactory;

use crate::{
    request::{DeleteDatabaseRequest, OpenDatabaseRequest},
    Error,
};

/// Lets applications asynchronously access the indexed databases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Factory {
    inner: IdbFactory,
}

impl Factory {
    /// Gets an instance of [Factory] from `global` scope.
    pub fn new() -> Result<Factory, Error> {
        let inner = Reflect::get(&js_sys::global(), &JsValue::from("indexedDB"))
            .map_err(Error::IndexedDbNotFound)?
            .dyn_into()
            .map_err(Error::IndexedDbNotFound)?;
        Ok(Self { inner })
    }

    /// Attempts to open a connection to the named database with the specified version. If the database already exists
    /// with a lower version and there are open connections that don’t close in response to a `versionchange` event, the
    /// request will be blocked until they all close, then an upgrade will occur. If the database already exists with a
    /// higher version the request will fail.
    pub fn open(&self, name: &str, version: Option<u32>) -> Result<OpenDatabaseRequest, Error> {
        match version {
            Some(version) => self.inner.open_with_u32(name, version),
            None => self.inner.open(name),
        }
        .map(Into::into)
        .map_err(Error::IndexedDbOpenFailed)
    }

    /// Attempts to delete the named database. If the database already exists and there are open connections that don’t
    /// close in response to a `versionchange` event, the request will be blocked until they all close.
    pub fn delete(&self, name: &str) -> Result<DeleteDatabaseRequest, Error> {
        self.inner
            .delete_database(name)
            .map(Into::into)
            .map_err(Error::IndexedDbDeleteFailed)
    }
}

impl From<IdbFactory> for Factory {
    fn from(inner: IdbFactory) -> Self {
        Self { inner }
    }
}

impl From<Factory> for IdbFactory {
    fn from(factory: Factory) -> Self {
        factory.inner
    }
}

impl TryFrom<JsValue> for Factory {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbFactory>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbFactory", value))
    }
}

impl From<Factory> for JsValue {
    fn from(value: Factory) -> Self {
        value.inner.into()
    }
}
