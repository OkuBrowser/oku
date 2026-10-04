use crate::Error;

impl_store_request_future!(
    CountStoreRequestFuture,
    crate::request::CountStoreRequest,
    u32,
    "Future returned by [`CountStoreRequest::into_future`](crate::request::CountStoreRequest::into_future)."
);
