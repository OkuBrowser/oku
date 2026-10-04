use std::future::Future;

use tokio::{select, sync::mpsc};
pub(crate) mod entity_manager;

/// A wrapper for a tokio mpsc receiver that allows peeking at the next message.
#[derive(Debug)]
pub struct PeekableReceiver<T> {
    msg: Option<T>,
    recv: mpsc::Receiver<T>,
}

#[allow(dead_code)]
impl<T> PeekableReceiver<T> {
    pub fn new(recv: mpsc::Receiver<T>) -> Self {
        Self { msg: None, recv }
    }

    /// Receive the next message.
    ///
    /// Will block if there are no messages.
    /// Returns None only if there are no more messages (sender is dropped).
    ///
    /// Cancel safe because the only async operation is the recv() call, which is cancel safe.
    pub async fn recv(&mut self) -> Option<T> {
        if let Some(msg) = self.msg.take() {
            return Some(msg);
        }
        self.recv.recv().await
    }

    /// Receive the next message, but only if it passes the filter.
    ///
    /// Cancel safe because the only async operation is the [Self::recv] call, which is cancel safe.
    pub async fn extract<U>(
        &mut self,
        f: impl Fn(T) -> std::result::Result<U, T>,
        timeout: impl Future + Unpin,
    ) -> Option<U> {
        let msg = select! {
            x = self.recv() => x?,
            _ = timeout => return None,
        };
        match f(msg) {
            Ok(u) => Some(u),
            Err(msg) => {
                self.msg = Some(msg);
                None
            }
        }
    }

    /// Push back a message. This will only work if there is room for it.
    /// Otherwise, it will fail and return the message.
    pub fn push_back(&mut self, msg: T) -> std::result::Result<(), T> {
        if self.msg.is_none() {
            self.msg = Some(msg);
            Ok(())
        } else {
            Err(msg)
        }
    }
}
