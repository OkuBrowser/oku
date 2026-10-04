use crate::Error;

impl_store_request_future!(
    GetStoreRequestFuture,
    crate::request::GetStoreRequest,
    Option<wasm_bindgen::JsValue>,
    "Future returned by [`GetStoreRequest::into_future`](crate::request::GetStoreRequest::into_future)."
);
