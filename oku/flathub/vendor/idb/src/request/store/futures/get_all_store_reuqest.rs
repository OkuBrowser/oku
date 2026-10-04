use crate::Error;

impl_store_request_future!(
    GetAllStoreRequestFuture,
    crate::request::GetAllStoreRequest,
    Vec<wasm_bindgen::JsValue>,
    "Future returned by [`GetAllStoreRequest::into_future`](crate::request::GetAllStoreRequest::into_future)."
);
