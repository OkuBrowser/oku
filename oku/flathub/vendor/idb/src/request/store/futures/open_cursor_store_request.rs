use crate::Error;

impl_store_request_future!(
    OpenCursorStoreRequestFuture,
    crate::request::OpenCursorStoreRequest,
    Option<crate::Cursor>,
    "Future returned by [`OpenCursorStoreRequest::into_future`](crate::request::OpenCursorStoreRequest::into_future)."
);
