macro_rules! impl_store_request_future {
    ($type: ident, $request: ty, $return_type: ty, $doc: expr) => {
        #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
        #[doc = $doc]
        pub struct $type {
            _inner: $request,
            success_receiver: tokio::sync::oneshot::Receiver<Result<$return_type, Error>>,
            error_receiver: tokio::sync::oneshot::Receiver<Error>,
        }

        #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
        impl std::future::IntoFuture for $request {
            type Output = <Self::IntoFuture as std::future::Future>::Output;

            type IntoFuture = $type;

            fn into_future(mut self) -> Self::IntoFuture {
                let (error_sender, error_receiver) = tokio::sync::oneshot::channel();
                let (success_sender, success_receiver) = tokio::sync::oneshot::channel();

                crate::Request::on_error(&mut self, move |event| {
                    let result = crate::StoreEvent::error(&event);
                    let _ = error_sender.send(result);
                });

                crate::Request::on_success(&mut self, move |event| {
                    let result = crate::StoreEvent::result(&event);
                    let _ = success_sender.send(result);
                });

                $type {
                    _inner: self,
                    success_receiver,
                    error_receiver,
                }
            }
        }

        impl std::future::Future for $type {
            type Output = Result<$return_type, crate::Error>;

            fn poll(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                let this = self.get_mut();

                if let std::task::Poll::Ready(err) =
                    std::pin::Pin::new(&mut this.error_receiver).poll(cx)
                {
                    return match err {
                        Ok(err) => std::task::Poll::Ready(Err(err)),
                        Err(_) => std::task::Poll::Ready(Err(Error::OneshotChannelReceiveError)),
                    };
                }

                if let std::task::Poll::Ready(result) =
                    std::pin::Pin::new(&mut this.success_receiver).poll(cx)
                {
                    return match result {
                        Ok(result) => std::task::Poll::Ready(result),
                        Err(_) => std::task::Poll::Ready(Err(Error::OneshotChannelReceiveError)),
                    };
                }

                std::task::Poll::Pending
            }
        }
    };
}
