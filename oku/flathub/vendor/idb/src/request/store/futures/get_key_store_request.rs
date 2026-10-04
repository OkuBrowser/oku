use crate::Error;

impl_store_request_future!(
    GetKeyStoreRequestFuture,
    crate::request::GetKeyStoreRequest,
    Option<wasm_bindgen::JsValue>,
    "Future returned by [`GetKeyStoreRequest::into_future`](crate::request::GetKeyStoreRequest::into_future)."
);
