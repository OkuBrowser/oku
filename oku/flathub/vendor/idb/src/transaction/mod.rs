#[cfg(feature = "futures")]
mod transaction_future;
mod transaction_mode;

#[cfg(feature = "futures")]
pub use self::transaction_future::{TransactionFuture, TransactionResult};
pub use self::transaction_mode::TransactionMode;

use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{DomException, Event, EventTarget, IdbTransaction};

use crate::{utils::dom_string_list_to_vec, Database, Error, ObjectStore};

/// Provides a static, asynchronous transaction on a database. All reading and writing of data is done within
/// transactions.
#[derive(Debug)]
pub struct Transaction {
    inner: IdbTransaction,
    abort_callback: Option<Closure<dyn FnMut(Event)>>,
    complete_callback: Option<Closure<dyn FnMut(Event)>>,
    error_callback: Option<Closure<dyn FnMut(Event)>>,
}

impl Transaction {
    /// Returns a list of the names of object stores in the transaction’s scope. For an upgrade transaction this is all
    /// object stores in the database.
    pub fn store_names(&self) -> Vec<String> {
        dom_string_list_to_vec(&self.inner.object_store_names())
    }

    /// Returns the mode the transaction was created with ("readonly" or "readwrite"), or "versionchange" for an upgrade
    /// transaction.
    pub fn mode(&self) -> Result<TransactionMode, Error> {
        self.inner
            .mode()
            .map_err(Error::TransactionModeNotFound)?
            .try_into()
    }

    /// Returns the transaction’s connection.
    pub fn database(&self) -> Database {
        self.inner.db().into()
    }

    /// If the transaction was aborted, returns the error (a `DOMException`) providing the reason.
    pub fn error(&self) -> Option<DomException> {
        self.inner.error()
    }

    /// Returns an [`ObjectStore`] in the transaction's scope.
    pub fn object_store(&self, name: &str) -> Result<ObjectStore, Error> {
        self.inner
            .object_store(name)
            .map(Into::into)
            .map_err(Error::ObjectStoreNotFound)
    }

    /// Attempts to commit the transaction. All pending requests will be allowed to complete, but no new requests will
    /// be accepted. This can be used to force a transaction to quickly finish, without waiting for pending requests to
    /// fire success events before attempting to commit normally.
    pub fn commit(self) -> Result<Self, Error> {
        // This API was removed from idb 2.0 spec but will be added back in idb 3.0 spec
        #[allow(deprecated)]
        self.inner.commit().map_err(Error::TransactionCommitError)?;
        Ok(self)
    }

    /// Aborts the transaction. All pending requests will fail and all changes made to the database will be reverted.
    pub fn abort(self) -> Result<Self, Error> {
        self.inner.abort().map_err(Error::TransactionAbortError)?;
        Ok(self)
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

    /// Adds an event handler for `complete` event.
    pub fn on_complete<F>(&mut self, callback: F)
    where
        F: FnOnce(Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_oncomplete(Some(closure.as_ref().unchecked_ref()));
        self.complete_callback = Some(closure);
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

    /// Release memory management of the callbacks to JS GC.
    ///
    /// > Note: This may leak memory. Read more about it
    /// > [here](https://docs.rs/wasm-bindgen/latest/wasm_bindgen/closure/struct.Closure.html#method.into_js_value).
    pub fn forget_callbacks(&mut self) {
        let abort_callback = self.abort_callback.take();
        let complete_callback = self.complete_callback.take();
        let error_callback = self.error_callback.take();

        if let Some(callback) = abort_callback {
            callback.forget();
        }

        if let Some(callback) = complete_callback {
            callback.forget();
        }

        if let Some(callback) = error_callback {
            callback.forget();
        }
    }
}

impl TryFrom<EventTarget> for Transaction {
    type Error = Error;

    fn try_from(target: EventTarget) -> Result<Self, Self::Error> {
        let target: JsValue = target.into();
        target
            .dyn_into::<IdbTransaction>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbTransaction", value))
    }
}

impl From<IdbTransaction> for Transaction {
    fn from(inner: IdbTransaction) -> Self {
        Self {
            inner,
            abort_callback: None,
            complete_callback: None,
            error_callback: None,
        }
    }
}

impl From<Transaction> for IdbTransaction {
    fn from(transaction: Transaction) -> Self {
        transaction.inner
    }
}

impl TryFrom<JsValue> for Transaction {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbTransaction>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbTransaction", value))
    }
}

impl From<Transaction> for JsValue {
    fn from(value: Transaction) -> Self {
        value.inner.into()
    }
}
