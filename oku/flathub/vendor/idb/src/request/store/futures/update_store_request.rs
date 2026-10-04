use crate::Error;

impl_store_request_future!(
    UpdateStoreRequestFuture,
    crate::request::UpdateStoreRequest,
    wasm_bindgen::JsValue,
    "Future returned by [`UpdateStoreRequest::into_future`](crate::request::UpdateStoreRequest::into_future)."
);
