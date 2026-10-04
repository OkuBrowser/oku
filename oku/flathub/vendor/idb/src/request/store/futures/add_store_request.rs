use crate::Error;

impl_store_request_future!(
    AddStoreRequestFuture,
    crate::request::AddStoreRequest,
    wasm_bindgen::JsValue,
    "Future returned by [`AddStoreRequest::into_future`](crate::request::AddStoreRequest::into_future)."
);
