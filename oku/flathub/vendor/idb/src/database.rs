use js_sys::Array;
use num_traits::ToPrimitive;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{Event, EventTarget, IdbDatabase};

#[cfg(feature = "builder")]
use crate::builder::DatabaseBuilder;
use crate::{
    utils::dom_string_list_to_vec, Error, ObjectStore, ObjectStoreParams, Transaction,
    TransactionMode,
};

/// [`Database`] provides a connection to a database; you can use an [`Database`] object to open a transaction on your
/// database then create, manipulate, and delete objects (data) in that database. The object provides the only way to
/// get and manage versions of the database.
#[derive(Debug)]
pub struct Database {
    inner: IdbDatabase,
    abort_callback: Option<Closure<dyn FnMut(Event)>>,
    close_callback: Option<Closure<dyn FnMut(Event)>>,
    error_callback: Option<Closure<dyn FnMut(Event)>>,
    version_change_callback: Option<Closure<dyn FnMut(Event)>>,
}

impl Database {
    /// Creates a new instance of [`DatabaseBuilder`].
    #[cfg(feature = "builder")]
    #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "builder")))]
    pub fn builder(name: &str) -> DatabaseBuilder {
        DatabaseBuilder::new(name)
    }

    /// Returns the name of the database.
    pub fn name(&self) -> String {
        self.inner.name()
    }

    /// Returns the version of the database.
    pub fn version(&self) -> Result<u32, Error> {
        self.inner
            .version()
            .to_u32()
            .ok_or(Error::NumberConversionError)
    }

    /// Returns a list of the names of [`ObjectStore`]s in the database.
    pub fn store_names(&self) -> Vec<String> {
        dom_string_list_to_vec(&self.inner.object_store_names())
    }

    /// Returns a new transaction with the given scope (which can be a single object store name or an array of names),
    /// mode ([`TransactionMode::ReadOnly`] or [`TransactionMode::ReadWrite`]).
    pub fn transaction<T>(
        &self,
        store_names: &[T],
        mode: TransactionMode,
    ) -> Result<Transaction, Error>
    where
        T: AsRef<str>,
    {
        let store_names: Array = store_names
            .iter()
            .map(|s| JsValue::from(s.as_ref()))
            .collect();

        self.inner
            .transaction_with_str_sequence_and_mode(&store_names, mode.into())
            .map(Into::into)
            .map_err(Error::TransactionOpenFailed)
    }

    /// Closes the connection once all running transactions have finished.
    pub fn close(&self) {
        self.inner.close()
    }

    /// Creates a new object store with the given name and options and returns a new [`ObjectStore`]. Returns an
    /// [`Error`] if not called within an upgrade transaction.
    pub fn create_object_store(
        &self,
        name: &str,
        params: ObjectStoreParams,
    ) -> Result<ObjectStore, Error> {
        self.inner
            .create_object_store_with_optional_parameters(name, &params.into())
            .map(Into::into)
            .map_err(Error::ObjectStoreCreateFailed)
    }

    /// Deletes the object store with the given name. Returns an [`Error`] if not called within an upgrade transaction.
    pub fn delete_object_store(&self, name: &str) -> Result<(), Error> {
        self.inner
            .delete_object_store(name)
            .map_err(Error::ObjectStoreDeleteFailed)
    }

    /// Adds an event handler for `abort` event.
    pub fn on_abort<F>(&mut self, callback: F)
    where
        F: FnOnce(Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_onabort(Some(closure.as_ref().unchecked_ref()));
        self.abort_callback = Some(closure);
    }

    /// Adds an event handler for `close` event.
    pub fn on_close<F>(&mut self, callback: F)
    where
        F: FnOnce(Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_onclose(Some(closure.as_ref().unchecked_ref()));
        self.close_callback = Some(closure);
    }

    /// Adds an event handler for `error` event.
    pub fn on_error<F>(&mut self, callback: F)
    where
        F: FnOnce(Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_onerror(Some(closure.as_ref().unchecked_ref()));
        self.error_callback = Some(closure);
    }

    /// Adds an event handler for `versionchange` event.
    pub fn on_version_change<F>(&mut self, callback: F)
    where
        F: FnOnce(Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_onversionchange(Some(closure.as_ref().unchecked_ref()));
        self.version_change_callback = Some(closure);
    }

    /// Release memory management of the callbacks to JS GC.
    ///
    /// > Note: This may leak memory. Read more about it
    /// > [here](https://docs.rs/wasm-bindgen/latest/wasm_bindgen/closure/struct.Closure.html#method.into_js_value).
    pub fn forget_callbacks(&mut self) {
        let abort_callback = self.abort_callback.take();
        let close_callback = self.close_callback.take();
        let error_callback = self.error_callback.take();
        let version_change_callback = self.version_change_callback.take();

        if let Some(callback) = abort_callback {
            callback.forget();
        }

        if let Some(callback) = close_callback {
            callback.forget();
        }

        if let Some(callback) = error_callback {
            callback.forget();
        }

        if let Some(callback) = version_change_callback {
            callback.forget();
        }
    }
}

impl TryFrom<EventTarget> for Database {
    type Error = Error;

    fn try_from(target: EventTarget) -> Result<Self, Self::Error> {
        let target: JsValue = target.into();
        target
            .dyn_into::<IdbDatabase>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbDatabase", value))
    }
}

impl From<IdbDatabase> for Database {
    fn from(inner: IdbDatabase) -> Self {
        Self {
            inner,
            abort_callback: None,
            close_callback: None,
            error_callback: None,
            version_change_callback: None,
        }
    }
}

impl From<Database> for IdbDatabase {
    fn from(database: Database) -> Self {
        database.inner
    }
}

impl TryFrom<JsValue> for Database {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbDatabase>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbDatabase", value))
    }
}

impl From<Database> for JsValue {
    fn from(value: Database) -> Self {
        value.inner.into()
    }
}
