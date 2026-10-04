//! Runtime-agnostic actor library for Rust
//!
//! Currently supports [`tokio`](https://docs.rs/tokio) for execution.
//! Please refer to [`AcTokio`] for example usage.
//!
//! Actors combine well with sharing immutable snapshots, like [`futures-signals`](https://docs.rs/futures-signals).

// while acto works fine with single-threaded runtimes, it must guarantee spawning on multi-threaded ones as well
#![deny(clippy::future_not_send)]

#[cfg(feature = "tokio")]
mod tokio;

#[cfg(feature = "tokio")]
pub use crate::tokio::{AcTokio, AcTokioRuntime, TokioJoinHandle};

mod abort;
mod acto_cell;
mod acto_handle;
mod acto_ref;
mod messages;
mod runtime;
mod snd_rcv;
mod supervision_ref;
pub mod variable;

pub use abort::{ActoAborted, PanicInfo, PanicOrAbort};
pub use acto_cell::ActoCell;
pub use acto_handle::{ActoHandle, MappedActoHandle};
pub use acto_ref::{ActoId, ActoRef};
pub use messages::{ActoInput, ActoMsgSuper};
pub use runtime::{ActoRuntime, MailboxSize};
pub use snd_rcv::{Receiver, Sender};
pub use supervision_ref::SupervisionRef;

#[cfg(test)]
mod tests;
