use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::sync::oneshot;
use web_sys::Event;

use crate::{Error, Transaction};

/// An enum that represents the result state of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionResult {
    /// The transaction was committed.
    Committed,
    /// The transaction was aborted.
    Aborted,
}

impl TransactionResult {
    /// Returns `true` if the transaction was committed.
    pub fn is_committed(&self) -> bool {
        matches!(self, Self::Committed)
    }

    /// Returns `true` if the transaction was aborted.
    pub fn is_aborted(&self) -> bool {
        matches!(self, Self::Aborted)
    }
}

/// Future that resolved when transaction is completed. Either successfully or with an error.
pub struct TransactionFuture {
    _inner: Transaction,
    abort_receiver: oneshot::Receiver<()>,
    success_receiver: oneshot::Receiver<()>,
    error_receiver: oneshot::Receiver<Error>,
}

impl IntoFuture for Transaction {
    type Output = <Self::IntoFuture as Future>::Output;

    type IntoFuture = TransactionFuture;

    fn into_future(mut self) -> Self::IntoFuture {
        let (abort_sender, abort_receiver) = oneshot::channel();
        let (error_sender, error_receiver) = oneshot::channel();
        let (success_sender, success_receiver) = oneshot::channel();

        self.on_abort(move |_| {
            let _ = abort_sender.send(());
        });

        self.on_complete(move |_| {
            let _ = success_sender.send(());
        });

        self.on_error(move |event| {
            let result = error_callback(event);
            let _ = error_sender.send(result);
        });

        Self::IntoFuture {
            _inner: self,
            abort_receiver,
            success_receiver,
            error_receiver,
        }
    }
}

impl Future for TransactionFuture {
    type Output = Result<TransactionResult, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if let Poll::Ready(err) = Pin::new(&mut this.error_receiver).poll(cx) {
            return match err {
                Ok(err) => Poll::Ready(Err(err)),
                Err(_) => Poll::Ready(Err(Error::OneshotChannelReceiveError)),
            };
        }

        if let Poll::Ready(result) = Pin::new(&mut this.success_receiver).poll(cx) {
            return match result {
                Ok(_) => Poll::Ready(Ok(TransactionResult::Committed)),
                Err(_) => Poll::Ready(Err(Error::OneshotChannelReceiveError)),
            };
        }

        if let Poll::Ready(result) = Pin::new(&mut this.abort_receiver).poll(cx) {
            return match result {
                Ok(_) => Poll::Ready(Ok(TransactionResult::Aborted)),
                Err(_) => Poll::Ready(Err(Error::OneshotChannelReceiveError)),
            };
        }

        Poll::Pending
    }
}

fn error_callback(event: Event) -> Error {
    error_callback_inner(event).unwrap_err()
}

fn error_callback_inner(event: Event) -> Result<(), Error> {
    let transaction = Transaction::try_from(event.target().ok_or(Error::EventTargetNotFound)?)?;
    let error = transaction.error();

    match error {
        None => Err(Error::DomExceptionNotFound),
        Some(error) => Err(Error::DomException(error)),
    }
}
