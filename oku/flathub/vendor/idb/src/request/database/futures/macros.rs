macro_rules! impl_database_request_future {
    ($type: ident, $base_request: ty, $return_type: ty, $doc: expr) => {
        #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
        #[doc = $doc]
        pub struct $type {
            inner: $base_request,
            success_receiver: tokio::sync::oneshot::Receiver<Result<$return_type, crate::Error>>,
            error_receiver: tokio::sync::oneshot::Receiver<crate::Error>,
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
                        Err(_) => {
                            std::task::Poll::Ready(Err(crate::Error::OneshotChannelReceiveError))
                        }
                    };
                }

                if let std::task::Poll::Ready(result) =
                    std::pin::Pin::new(&mut this.success_receiver).poll(cx)
                {
                    return match result {
                        Ok(result) => std::task::Poll::Ready(result),
                        Err(_) => {
                            std::task::Poll::Ready(Err(crate::Error::OneshotChannelReceiveError))
                        }
                    };
                }

                std::task::Poll::Pending
            }
        }

        impl Drop for $type {
            fn drop(&mut self) {
                self.inner.forget_callbacks();
            }
        }

        #[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
        impl std::future::IntoFuture for $base_request {
            type Output = <Self::IntoFuture as std::future::Future>::Output;

            type IntoFuture = $type;

            fn into_future(mut self) -> Self::IntoFuture {
                let (error_sender, error_receiver) = tokio::sync::oneshot::channel();
                let (success_sender, success_receiver) = tokio::sync::oneshot::channel();

                crate::Request::on_error(&mut self, move |event| {
                    let result = error_callback(event.into());
                    let _ = error_sender.send(result);
                });

                crate::Request::on_success(&mut self, move |event| {
                    let result = success_callback(event.into());
                    let _ = success_sender.send(result);
                });

                $type {
                    inner: self,
                    success_receiver,
                    error_receiver,
                }
            }
        }

        fn error_callback(event: web_sys::Event) -> crate::Error {
            error_callback_inner(event).unwrap_err()
        }

        fn error_callback_inner(event: web_sys::Event) -> Result<(), crate::Error> {
            let target = event
                .target()
                .ok_or_else(|| crate::Error::EventTargetNotFound)?;
            let request = <$base_request>::try_from(target)?;

            let error = crate::Request::error(&request)?;

            match error {
                None => Err(crate::Error::DomExceptionNotFound),
                Some(error) => Err(crate::Error::DomException(error)),
            }
        }
    };
}
