#[macro_use]
mod macros;

#[cfg(feature = "futures")]
mod futures;

use js_sys::Object;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{DomException, Event, EventTarget, IdbOpenDbRequest, IdbVersionChangeEvent};

use crate::{
    event::{DeleteDatabaseRequestEvent, OpenDatabaseRequestEvent, VersionChangeEvent},
    request::RequestReadyState,
    Database, Error, Request, Transaction,
};

#[cfg(feature = "futures")]
pub use self::futures::{DeleteDatabaseRequestFuture, OpenDatabaseRequestFuture};

/// Request returned by [`Factory`](crate::Factory) when opening or deleting a database.
#[derive(Debug)]
struct DatabaseRequest {
    inner: IdbOpenDbRequest,
    success_callback: Option<Closure<dyn FnMut(Event)>>,
    error_callback: Option<Closure<dyn FnMut(Event)>>,
    blocked_callback: Option<Closure<dyn FnMut(IdbVersionChangeEvent)>>,
    upgrade_needed_callback: Option<Closure<dyn FnMut(IdbVersionChangeEvent)>>,
}

impl DatabaseRequest {
    /// Returns the database associated with this request
    pub fn database(&self) -> Result<Database, Error> {
        self.result().map(TryInto::try_into)?
    }

    /// Adds an event handler for `blocked` event.
    pub fn on_blocked<F>(&mut self, callback: F)
    where
        F: FnOnce(VersionChangeEvent) + 'static,
    {
        let f = move |event: IdbVersionChangeEvent| {
            let event = VersionChangeEvent::from(event);
            callback(event);
        };

        let closure = Closure::once(f);

        self.inner
            .set_onblocked(Some(closure.as_ref().unchecked_ref()));
        self.blocked_callback = Some(closure);
    }

    /// Adds an event handler for `upgradeneeded` event.
    pub fn on_upgrade_needed<F>(&mut self, callback: F)
    where
        F: FnOnce(VersionChangeEvent) + 'static,
    {
        let f = move |event: IdbVersionChangeEvent| {
            let event = VersionChangeEvent::from(event);
            callback(event);
        };

        let closure = Closure::once(f);

        self.inner
            .set_onupgradeneeded(Some(closure.as_ref().unchecked_ref()));
        self.upgrade_needed_callback = Some(closure);
    }

    /// Release memory management of the callbacks to JS GC.
    ///
    /// > Note: This may leak memory. Read more about it
    /// > [here](https://docs.rs/wasm-bindgen/latest/wasm_bindgen/closure/struct.Closure.html#method.into_js_value).
    pub fn forget_callbacks(&mut self) {
        let success_callback = self.success_callback.take();
        let error_callback = self.error_callback.take();
        let blocked_callback = self.blocked_callback.take();
        let upgrade_needed_callback = self.upgrade_needed_callback.take();

        if let Some(callback) = success_callback {
            callback.forget();
        }

        if let Some(callback) = error_callback {
            callback.forget();
        }

        if let Some(callback) = blocked_callback {
            callback.forget();
        }

        if let Some(callback) = upgrade_needed_callback {
            callback.forget();
        }
    }
}

impl Request for DatabaseRequest {
    type Event = Event;

    fn result(&self) -> Result<JsValue, Error> {
        self.inner.result().map_err(Error::RequestResultNotFound)
    }

    fn error(&self) -> Result<Option<DomException>, Error> {
        self.inner.error().map_err(Error::RequestErrorNotFound)
    }

    fn source(&self) -> Result<Object, Error> {
        self.inner.source().ok_or(Error::RequestSourceNotFound)
    }

    fn transaction(&self) -> Option<Transaction> {
        self.inner.transaction().map(Into::into)
    }

    fn ready_state(&self) -> Result<RequestReadyState, Error> {
        self.inner.ready_state().try_into()
    }

    fn on_success<F>(&mut self, callback: F)
    where
        F: FnOnce(Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_onsuccess(Some(closure.as_ref().unchecked_ref()));
        self.success_callback = Some(closure);
    }

    fn on_error<F>(&mut self, callback: F)
    where
        F: FnOnce(Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_onerror(Some(closure.as_ref().unchecked_ref()));
        self.error_callback = Some(closure);
    }
}

impl TryFrom<EventTarget> for DatabaseRequest {
    type Error = Error;

    fn try_from(target: EventTarget) -> Result<Self, Self::Error> {
        let target: JsValue = target.into();
        target
            .dyn_into::<IdbOpenDbRequest>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbOpenDbRequest", value))
    }
}

impl From<IdbOpenDbRequest> for DatabaseRequest {
    fn from(inner: IdbOpenDbRequest) -> Self {
        Self {
            inner,
            success_callback: None,
            error_callback: None,
            blocked_callback: None,
            upgrade_needed_callback: None,
        }
    }
}

impl From<DatabaseRequest> for IdbOpenDbRequest {
    fn from(request: DatabaseRequest) -> Self {
        request.inner
    }
}

impl TryFrom<JsValue> for DatabaseRequest {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbOpenDbRequest>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbOpenDbRequest", value))
    }
}

impl From<DatabaseRequest> for JsValue {
    fn from(value: DatabaseRequest) -> Self {
        value.inner.into()
    }
}

impl_database_request!(
    OpenDatabaseRequest,
    OpenDatabaseRequestEvent,
    "Request returned by [`Factory`](crate::Factory) when opening a database."
);

impl_database_request!(
    DeleteDatabaseRequest,
    DeleteDatabaseRequestEvent,
    "Request returned by [`Factory`](crate::Factory) when deleting a database."
);
