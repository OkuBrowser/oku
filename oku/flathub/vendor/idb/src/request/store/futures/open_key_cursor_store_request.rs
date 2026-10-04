use crate::Error;

impl_store_request_future!(
    OpenKeyCursorStoreRequestFuture,
    crate::request::OpenKeyCursorStoreRequest,
    Option<crate::KeyCursor>,
    "Future returned by [`OpenKeyCursorStoreRequest::into_future`](crate::request::OpenKeyCursorStoreRequest::into_future)."
);
