use crate::Error;

impl_store_request_future!(
    PutStoreRequestFuture,
    crate::request::PutStoreRequest,
    wasm_bindgen::JsValue,
    "Future returned by [`PutStoreRequest::into_future`](crate::request::PutStoreRequest::into_future)."
);
