use crate::{ActoId, PanicOrAbort};
use std::mem::discriminant;

/// Actor input as received with [`ActoCell::recv`].
#[derive(Debug)]
pub enum ActoInput<M, S> {
    /// All previously generated [`ActoRef`] handles were dropped, leaving only
    /// the one within [`ActoCell`]; the actor may wish to terminate unless it has
    /// other sources of input.
    ///
    /// Obtaining a new handle with [`ActoCell::me`] and dropping it will again
    /// generate this input.
    NoMoreSenders,
    /// A supervised actor with the given [`ActoId`] has terminated.
    ///
    /// The result contains either the value returned by the actor or the value
    /// with which the actor’s task panicked (if the underlying runtime handles this).
    ///
    /// ## Important notice
    ///
    /// Supervision notifications are delivered as soon as possible after the
    /// supervised actor’s task has finished. Messages sent by that actor — possibly
    /// to the supervisor — may still be in flight at this point.
    Supervision {
        id: ActoId,
        name: String,
        result: Result<S, PanicOrAbort>,
    },
    /// A message has been received via our [`ActoRef`] handle.
    Message(M),
}

impl<M, S> ActoInput<M, S> {
    pub fn is_sender_gone(&self) -> bool {
        matches!(self, ActoInput::NoMoreSenders)
    }

    pub fn is_supervision(&self) -> bool {
        matches!(self, ActoInput::Supervision { .. })
    }

    pub fn is_message(&self) -> bool {
        matches!(self, ActoInput::Message(_))
    }

    /// Obtain input message or supervision unless this is [`ActoInput::NoMoreSenders`].
    ///
    /// ```rust
    /// use acto::{ActoCell, ActoRuntime};
    ///
    /// async fn actor(mut cell: ActoCell<String, impl ActoRuntime>) {
    ///     while let Some(input) = cell.recv().await.has_senders() {
    ///         // do something with it
    ///     }
    ///     // actor automatically stops when all senders are gone
    /// }
    /// ```
    pub fn has_senders(self) -> Option<ActoMsgSuper<M, S>> {
        match self {
            ActoInput::NoMoreSenders => None,
            ActoInput::Supervision { id, name, result } => {
                Some(ActoMsgSuper::Supervision { id, name, result })
            }
            ActoInput::Message(msg) => Some(ActoMsgSuper::Message(msg)),
        }
    }
}

impl<M: PartialEq, S> PartialEq for ActoInput<M, S> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Supervision { id: left, .. }, Self::Supervision { id: right, .. }) => {
                left == right
            }
            (Self::Message(l0), Self::Message(r0)) => l0 == r0,
            _ => discriminant(self) == discriminant(other),
        }
    }
}

/// A filtered variant of [`ActoInput`] that omits `NoMoreSenders`.
///
/// see [`ActoInput::has_senders`]
pub enum ActoMsgSuper<M, S> {
    Supervision {
        id: ActoId,
        name: String,
        result: Result<S, PanicOrAbort>,
    },
    /// A message has been received via our [`ActoRef`] handle.
    Message(M),
}
