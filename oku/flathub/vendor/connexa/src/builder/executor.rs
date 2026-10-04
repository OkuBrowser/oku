// This will internally use `async-rt` for the executor, which
// would be selected at compile-time based on target.
pub struct ConnexaExecutor;

impl libp2p::swarm::Executor for ConnexaExecutor {
    fn exec(&self, future: std::pin::Pin<Box<dyn Future<Output = ()> + 'static + Send>>) {
        async_rt::task::dispatch(future);
    }
}
