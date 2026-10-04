use crate::Error;

impl_store_request_future!(
    ClearStoreRequestFuture,
    crate::request::ClearStoreRequest,
    (),
    "Future returned by [`ClearStoreRequest::into_future`](crate::request::ClearStoreRequest::into_future)."
);
