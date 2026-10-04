use wasm_bindgen::JsValue;

use crate::{Error, Event};

#[macro_use]
mod macros;

/// Trait for defining events triggered when operations are performed on an object store.
pub trait StoreEvent: Event {
    /// The type of the result of the operation.
    type Output;

    /// Returns the result of the operation.
    fn result(&self) -> Result<Self::Output, Error>;

    /// Returns the error of the operation.
    fn error(&self) -> Error;
}

impl_store_event!(
    PutStoreRequestEvent,
    web_sys::Event,
    crate::request::PutStoreRequest,
    wasm_bindgen::JsValue,
    crate::mappers::NullCheckMapper,
    "Event for [`PutStoreRequest`](crate::request::PutStoreRequest) handlers."
);
impl_store_event!(
    AddStoreRequestEvent,
    web_sys::Event,
    crate::request::AddStoreRequest,
    wasm_bindgen::JsValue,
    crate::mappers::NullCheckMapper,
    "Event for [`AddStoreRequest`](crate::request::AddStoreRequest) handlers."
);
impl_store_event!(
    DeleteStoreRequestEvent,
    web_sys::Event,
    crate::request::DeleteStoreRequest,
    (),
    crate::mappers::IgnoreMapper,
    "Event for [`DeleteStoreRequest`](crate::request::DeleteStoreRequest) handlers."
);
impl_store_event!(
    ClearStoreRequestEvent,
    web_sys::Event,
    crate::request::ClearStoreRequest,
    (),
    crate::mappers::IgnoreMapper,
    "Event for [`ClearStoreRequest`](crate::request::ClearStoreRequest) handlers."
);
impl_store_event!(
    GetStoreRequestEvent,
    web_sys::Event,
    crate::request::GetStoreRequest,
    Option<JsValue>,
    crate::mappers::OptionMapper,
    "Event for [`GetStoreRequest`](crate::request::GetStoreRequest) handlers."
);
impl_store_event!(
    GetKeyStoreRequestEvent,
    web_sys::Event,
    crate::request::GetKeyStoreRequest,
    Option<JsValue>,
    crate::mappers::OptionMapper,
    "Event for [`GetKeyStoreRequest`](crate::request::GetKeyStoreRequest) handlers."
);
impl_store_event!(
    GetAllStoreRequestEvent,
    web_sys::Event,
    crate::request::GetAllStoreRequest,
    Vec<JsValue>,
    crate::mappers::VecMapper,
    "Event for [`GetAllStoreRequest`](crate::request::GetAllStoreRequest) handlers."
);
impl_store_event!(
    GetAllKeysStoreRequestEvent,
    web_sys::Event,
    crate::request::GetAllKeysStoreRequest,
    Vec<JsValue>,
    crate::mappers::VecMapper,
    "Event for [`GetAllKeysStoreRequest`](crate::request::GetAllKeysStoreRequest) handlers."
);
impl_store_event!(
    CountStoreRequestEvent,
    web_sys::Event,
    crate::request::CountStoreRequest,
    u32,
    crate::mappers::U32Mapper,
    "Event for [`CountStoreRequest`](crate::request::CountStoreRequest) handlers."
);
impl_store_event!(
    OpenCursorStoreRequestEvent,
    web_sys::Event,
    crate::request::OpenCursorStoreRequest,
    Option<crate::Cursor>,
    crate::mappers::CursorMapper,
    "Event for [`OpenCursorStoreRequest`](crate::request::OpenCursorStoreRequest) handlers."
);
impl_store_event!(
    OpenKeyCursorStoreRequestEvent,
    web_sys::Event,
    crate::request::OpenKeyCursorStoreRequest,
    Option<crate::KeyCursor>,
    crate::mappers::KeyCursorMapper,
    "Event for [`OpenKeyCursorStoreRequest`](crate::request::OpenKeyCursorStoreRequest) handlers."
);
impl_store_event!(
    UpdateStoreRequestEvent,
    web_sys::Event,
    crate::request::UpdateStoreRequest,
    JsValue,
    crate::mappers::NullCheckMapper,
    "Event for [`UpdateStoreRequest`](crate::request::UpdateStoreRequest) handlers."
);
