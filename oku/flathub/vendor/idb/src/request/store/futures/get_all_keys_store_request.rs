use crate::Error;

impl_store_request_future!(
    GetAllKeysStoreRequestFuture,
    crate::request::GetAllKeysStoreRequest,
    Vec<wasm_bindgen::JsValue>,
    "Future returned by [`GetAllKeysStoreRequest::into_future`](crate::request::GetAllKeysStoreRequest::into_future)."
);
