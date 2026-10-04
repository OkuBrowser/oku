use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JoinError {
    /// The task was cancelled without an explicit abort request through its
    /// [`JoinHandle`](crate::JoinHandle), such as during runtime shutdown.
    #[error("The task was cancelled")]
    Cancelled,
    /// The task was cancelled after [`JoinHandle::abort`](crate::JoinHandle::abort)
    /// was requested.
    #[error("The task was aborted")]
    Aborted,
    /// The task panicked.
    #[error("The task panicked")]
    Panicked,
    /// The task that was polled was empty or contained no pending future.
    #[error("The task was empty")]
    Empty,
    /// Unknown error.
    #[error("An unknown error occurred")]
    Unknown,
}

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
impl From<tokio::task::JoinError> for JoinError {
    fn from(err: tokio::task::JoinError) -> Self {
        if err.is_cancelled() {
            return JoinError::Cancelled;
        }

        if err.is_panic() {
            return JoinError::Panicked;
        }

        JoinError::Unknown
    }
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
impl From<compio::runtime::JoinError> for JoinError {
    fn from(err: compio::runtime::JoinError) -> Self {
        match err {
            compio::runtime::JoinError::Cancelled => JoinError::Cancelled,
            compio::runtime::JoinError::Panicked(_) => JoinError::Panicked,
        }
    }
}

/// Error indicating a task did not complete before its timeout elapsed.
///
/// Returned as the inner error of a timeout task's output, for example from
/// [`ExecutorTimeout::spawn_timeout`](crate::ExecutorTimeout::spawn_timeout). A timeout does not
/// produce [`JoinError::Aborted`] or [`JoinError::Cancelled`] as those come from aborting or dropping
/// the task's handle.
#[derive(Debug, Error)]
#[error("The task timed out")]
pub struct TimeoutError;
