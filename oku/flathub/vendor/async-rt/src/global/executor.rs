use crate::{Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, JoinHandle};

/// Executor that selects an available runtime backend at compile time.
///
/// * On non-Wasm targets with the `tokio`, `smol`, or `compio` feature enabled, it uses
///   `TokioExecutor`, `SmolExecutor`, or `CompioExecutor`.
/// * On non-Wasm targets with the `threadpool` feature enabled and the `tokio`, `smol` or `compio`
///   feature disabled, it uses `ThreadPoolExecutor`.
/// * On non-Wasm targets with only the `lite` runtime feature enabled, it uses
///   `LiteExecutor`.
/// * On Wasm targets, it uses `WasmExecutor`, backed by
///   `wasm-bindgen-futures`.
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub type DefaultExecutor = crate::rt::tokio::TokioExecutor;

#[cfg(all(
    feature = "compio",
    not(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))
))]
pub type DefaultExecutor = crate::rt::compio::CompioExecutor;

#[cfg(all(feature = "smol", not(any(feature = "tokio", target_arch = "wasm32"))))]
pub type DefaultExecutor = crate::rt::smol::SmolExecutor;

#[cfg(all(
    feature = "threadpool",
    not(any(
        feature = "tokio",
        feature = "smol",
        feature = "compio",
        target_arch = "wasm32"
    ))
))]
pub type DefaultExecutor = crate::rt::threadpool::ThreadPoolExecutor;

#[cfg(all(
    feature = "lite",
    not(any(
        feature = "tokio",
        feature = "smol",
        feature = "compio",
        feature = "threadpool"
    )),
    not(target_arch = "wasm32")
))]
pub type DefaultExecutor = crate::rt::lite::LiteExecutor;

#[cfg(target_arch = "wasm32")]
pub type DefaultExecutor = crate::rt::wasm::WasmExecutor;

#[cfg(all(
    not(feature = "threadpool"),
    not(feature = "tokio"),
    not(feature = "compio"),
    not(feature = "smol"),
    not(feature = "lite"),
    not(target_arch = "wasm32")
))]
pub type DefaultExecutor = crate::rt::dummy::DummyExecutor;

/// Built-in executor types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinExecutor {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    Tokio,
    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    Smol,
    #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
    Compio,
    #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
    ThreadPool,
    #[cfg(target_arch = "wasm32")]
    Wasm,
    #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
    Lite,
    Dummy,
}

impl BuiltinExecutor {
    pub(crate) fn is_exclusive(&self) -> bool {
        match self {
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => true,
            _ => false,
        }
    }
}

impl Default for BuiltinExecutor {
    fn default() -> Self {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        return Self::Tokio;

        #[cfg(all(feature = "smol", not(any(feature = "tokio", target_arch = "wasm32"))))]
        return Self::Smol;

        #[cfg(all(
            feature = "compio",
            not(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))
        ))]
        return Self::Compio;

        #[cfg(all(
            feature = "threadpool",
            not(any(
                feature = "tokio",
                feature = "smol",
                feature = "compio",
                target_arch = "wasm32"
            ))
        ))]
        return Self::ThreadPool;

        #[cfg(target_arch = "wasm32")]
        return Self::Wasm;

        #[cfg(all(
            feature = "lite",
            not(any(
                feature = "threadpool",
                feature = "tokio",
                feature = "compio",
                feature = "smol"
            )),
            not(target_arch = "wasm32")
        ))]
        return Self::Lite;

        #[cfg(all(
            not(feature = "threadpool"),
            not(feature = "tokio"),
            not(feature = "compio"),
            not(feature = "smol"),
            not(feature = "lite"),
            not(target_arch = "wasm32")
        ))]
        return Self::Dummy;
    }
}

impl Executor for BuiltinExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        match self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Tokio => Executor::runtime_type(&crate::rt::tokio::TokioExecutor),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            BuiltinExecutor::Smol => Executor::runtime_type(&crate::rt::smol::SmolExecutor),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Compio => Executor::runtime_type(&crate::rt::compio::CompioExecutor),
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            BuiltinExecutor::ThreadPool => {
                Executor::runtime_type(&crate::rt::threadpool::ThreadPoolExecutor)
            }
            #[cfg(target_arch = "wasm32")]
            BuiltinExecutor::Wasm => Executor::runtime_type(&crate::rt::wasm::WasmExecutor),
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            BuiltinExecutor::Lite => Executor::runtime_type(&crate::rt::lite::LiteExecutor),
            BuiltinExecutor::Dummy => Executor::runtime_type(&crate::rt::dummy::DummyExecutor),
        }
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Tokio => Executor::spawn(&crate::rt::tokio::TokioExecutor, future),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            BuiltinExecutor::Smol => Executor::spawn(&crate::rt::smol::SmolExecutor, future),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Compio => Executor::spawn(&crate::rt::compio::CompioExecutor, future),
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            BuiltinExecutor::ThreadPool => {
                Executor::spawn(&crate::rt::threadpool::ThreadPoolExecutor, future)
            }
            #[cfg(target_arch = "wasm32")]
            BuiltinExecutor::Wasm => Executor::spawn(&crate::rt::wasm::WasmExecutor, future),
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            BuiltinExecutor::Lite => Executor::spawn(&crate::rt::lite::LiteExecutor, future),
            BuiltinExecutor::Dummy => Executor::spawn(&crate::rt::dummy::DummyExecutor, future),
        }
    }
}

impl ExecutorBlocking for BuiltinExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        match self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Tokio => {
                ExecutorBlocking::spawn_blocking(&crate::rt::tokio::TokioExecutor, f)
            }
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            BuiltinExecutor::Smol => {
                ExecutorBlocking::spawn_blocking(&crate::rt::smol::SmolExecutor, f)
            }
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Compio => {
                ExecutorBlocking::spawn_blocking(&crate::rt::compio::CompioExecutor, f)
            }
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            BuiltinExecutor::ThreadPool => {
                ExecutorBlocking::spawn_blocking(&crate::rt::threadpool::ThreadPoolExecutor, f)
            }
            #[cfg(target_arch = "wasm32")]
            BuiltinExecutor::Wasm => {
                ExecutorBlocking::spawn_blocking(&crate::rt::wasm::WasmExecutor, f)
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            BuiltinExecutor::Lite => {
                ExecutorBlocking::spawn_blocking(&crate::rt::lite::LiteExecutor, f)
            }
            BuiltinExecutor::Dummy => {
                ExecutorBlocking::spawn_blocking(&crate::rt::dummy::DummyExecutor, f)
            }
        }
    }
}

impl ExecutorTimeout for BuiltinExecutor {}

impl ExecutorBlockOn for BuiltinExecutor {
    fn block_on<F: Future>(&self, future: F) -> F::Output {
        match self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Tokio => {
                ExecutorBlockOn::block_on(&crate::rt::tokio::TokioExecutor, future)
            }
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            BuiltinExecutor::Smol => {
                ExecutorBlockOn::block_on(&crate::rt::smol::SmolExecutor, future)
            }
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Compio => {
                ExecutorBlockOn::block_on(&crate::rt::compio::CompioExecutor, future)
            }
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            BuiltinExecutor::ThreadPool => {
                ExecutorBlockOn::block_on(&crate::rt::threadpool::ThreadPoolExecutor, future)
            }
            #[cfg(target_arch = "wasm32")]
            BuiltinExecutor::Wasm => {
                ExecutorBlockOn::block_on(&crate::rt::wasm::WasmExecutor, future)
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            BuiltinExecutor::Lite => {
                ExecutorBlockOn::block_on(&crate::rt::lite::LiteExecutor, future)
            }
            BuiltinExecutor::Dummy => {
                ExecutorBlockOn::block_on(&crate::rt::dummy::DummyExecutor, future)
            }
        }
    }
}
