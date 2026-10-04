#[cfg(feature = "fs-store")]
pub mod oneshot {
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let (tx, rx) = tokio::sync::oneshot::channel::<T>();
        (Sender::Tokio(tx), Receiver::Tokio(rx))
    }

    #[derive(Debug)]
    pub enum Sender<T> {
        Tokio(tokio::sync::oneshot::Sender<T>),
    }

    impl<T> From<Sender<T>> for irpc::channel::oneshot::Sender<T> {
        fn from(sender: Sender<T>) -> Self {
            match sender {
                Sender::Tokio(tx) => tx.into(),
            }
        }
    }

    impl<T> Sender<T> {
        pub fn send(self, value: T) {
            match self {
                Self::Tokio(tx) => tx.send(value).ok(),
            };
        }
    }

    pub enum Receiver<T> {
        Tokio(tokio::sync::oneshot::Receiver<T>),
    }

    impl<T> Future for Receiver<T> {
        type Output = std::result::Result<T, tokio::sync::oneshot::error::RecvError>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.as_mut().get_mut() {
                Self::Tokio(rx) => {
                    if rx.is_terminated() {
                        // don't panic when polling a terminated receiver
                        Poll::Pending
                    } else {
                        Future::poll(Pin::new(rx), cx)
                    }
                }
            }
        }
    }
}
