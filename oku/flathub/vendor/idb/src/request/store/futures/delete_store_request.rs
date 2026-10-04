use crate::Error;

impl_store_request_future!(
    DeleteStoreRequestFuture,
    crate::request::DeleteStoreRequest,
    (),
    "Future returned by [`DeleteStoreRequest::into_future`](crate::request::DeleteStoreRequest::into_future)."
);
