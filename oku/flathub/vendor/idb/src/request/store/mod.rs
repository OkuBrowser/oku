#[macro_use]
mod macros;

#[cfg(feature = "futures")]
mod futures;

#[cfg(feature = "futures")]
pub use self::futures::{
    AddStoreRequestFuture, ClearStoreRequestFuture, CountStoreRequestFuture,
    DeleteStoreRequestFuture, GetAllKeysStoreRequestFuture, GetAllStoreRequestFuture,
    GetKeyStoreRequestFuture, GetStoreRequestFuture, OpenCursorStoreRequestFuture,
    OpenKeyCursorStoreRequestFuture, PutStoreRequestFuture, UpdateStoreRequestFuture,
};

use js_sys::Object;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{DomException, Event, EventTarget, IdbRequest};

use crate::{request::RequestReadyState, Error, Request, Transaction};

/// Request returned when performing operations on an [`ObjectStore`](crate::ObjectStore).
#[derive(Debug)]
pub struct StoreRequest {
    inner: IdbRequest,
    success_callback: Option<Closure<dyn FnMut(Event)>>,
    error_callback: Option<Closure<dyn FnMut(Event)>>,
}

impl StoreRequest {
    /// Release memory management of the callbacks to JS GC.
    ///
    /// > Note: This may leak memory. Read more about it
    /// > [here](https://docs.rs/wasm-bindgen/latest/wasm_bindgen/closure/struct.Closure.html#method.into_js_value).
    pub fn forget_callbacks(&mut self) {
        let success_callback = self.success_callback.take();
        let error_callback = self.error_callback.take();

        if let Some(callback) = success_callback {
            callback.forget();
        }

        if let Some(callback) = error_callback {
            callback.forget();
        }
    }
}

impl Request for StoreRequest {
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
        F: FnOnce(Self::Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_onsuccess(Some(closure.as_ref().unchecked_ref()));
        self.success_callback = Some(closure);
    }

    fn on_error<F>(&mut self, callback: F)
    where
        F: FnOnce(Self::Event) + 'static,
    {
        let closure = Closure::once(callback);
        self.inner
            .set_onerror(Some(closure.as_ref().unchecked_ref()));
        self.error_callback = Some(closure);
    }
}

impl TryFrom<EventTarget> for StoreRequest {
    type Error = Error;

    fn try_from(target: EventTarget) -> Result<Self, Self::Error> {
        let target: JsValue = target.into();
        target
            .dyn_into::<IdbRequest>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbRequest", value))
    }
}

impl From<IdbRequest> for StoreRequest {
    fn from(inner: IdbRequest) -> Self {
        Self {
            inner,
            success_callback: None,
            error_callback: None,
        }
    }
}

impl From<StoreRequest> for IdbRequest {
    fn from(request: StoreRequest) -> Self {
        request.inner
    }
}

impl TryFrom<JsValue> for StoreRequest {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbRequest>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbRequest", value))
    }
}

impl From<StoreRequest> for JsValue {
    fn from(value: StoreRequest) -> Self {
        value.inner.into()
    }
}

impl_store_request!(
    PutStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::PutStoreRequestEvent,
    "Request returned when performing [`ObjectStore::put`](crate::ObjectStore::put)."
);
impl_store_request!(
    AddStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::AddStoreRequestEvent,
    "Request returned when performing [`ObjectStore::add`](crate::ObjectStore::add)."
);
impl_store_request!(
    DeleteStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::DeleteStoreRequestEvent,
    "Request returned when performing [`ObjectStore::delete`](crate::ObjectStore::delete), [`Cursor::delete`](crate::Cursor::delete) or [`KeyCursor::delete`](crate::KeyCursor::delete)."
);
impl_store_request!(
    ClearStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::ClearStoreRequestEvent,
    "Request returned when performing [`ObjectStore::clear`](crate::ObjectStore::clear)."
);
impl_store_request!(
    GetStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::GetStoreRequestEvent,
    "Request returned when performing [`ObjectStore::get`](crate::ObjectStore::get) or [`Index::get`](crate::Index::get)."
);
impl_store_request!(
    GetKeyStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::GetKeyStoreRequestEvent,
    "Request returned when performing [`ObjectStore::get_key`](crate::ObjectStore::get_key) or [`Index::get_key`](crate::Index::get_key)."
);
impl_store_request!(
    GetAllStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::GetAllStoreRequestEvent,
    "Request returned when performing [`ObjectStore::get_all`](crate::ObjectStore::get_all) or [`Index::get_all`](crate::Index::get_all)."
);
impl_store_request!(
    GetAllKeysStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::GetAllKeysStoreRequestEvent,
    "Request returned when performing [`ObjectStore::get_all_keys`](crate::ObjectStore::get_all_keys) or [`Index::get_all_keys`](crate::Index::get_all_keys)."
);
impl_store_request!(
    CountStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::CountStoreRequestEvent,
    "Request returned when performing [`ObjectStore::count`](crate::ObjectStore::count) or [`Index::count`](crate::Index::count)."
);
impl_store_request!(
    OpenCursorStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::OpenCursorStoreRequestEvent,
    "Request returned when performing [`ObjectStore::open_cursor`](crate::ObjectStore::open_cursor) or [`Index::open_cursor`](crate::Index::open_cursor)."
);
impl_store_request!(
    OpenKeyCursorStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::OpenKeyCursorStoreRequestEvent,
    "Request returned when performing [`ObjectStore::open_key_cursor`](crate::ObjectStore::open_key_cursor) or [`Index::open_key_cursor`](crate::Index::open_key_cursor)."
);
impl_store_request!(
    UpdateStoreRequest,
    crate::request::store::StoreRequest,
    crate::event::UpdateStoreRequestEvent,
    "Request returned when performing [`Cursor::update`](crate::Cursor::update) or [`KeyCursor::update`](crate::KeyCursor::update)."
);
