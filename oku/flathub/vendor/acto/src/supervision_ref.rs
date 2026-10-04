use crate::{ActoHandle, ActoRef, MappedActoHandle};

/// A package of an actor’s [`ActoRef`] and [`ActoHandle`].
///
/// This is the result of [`ActoCell::spawn`] and can be passed to [`ActoCell::supervise`].
pub struct SupervisionRef<M, H> {
    pub me: ActoRef<M>,
    pub handle: H,
}

impl<M: Send + 'static, H: ActoHandle> SupervisionRef<M, H> {
    /// Derive a new reference by embedding the supervisor-required type `M2` into the message schema.
    ///
    /// ```rust
    /// # use acto::{ActoCell, ActoInput, ActoRef, ActoRuntime, SupervisionRef};
    /// struct Shutdown;
    ///
    /// enum ActorCommand {
    ///     Shutdown(Shutdown),
    ///     DoStuff(String),
    /// }
    ///
    /// async fn top_level(mut cell: ActoCell<(), impl ActoRuntime>) {
    ///     let supervisor = cell.spawn_supervised("super",
    ///         |mut cell: ActoCell<SupervisionRef<Shutdown, _>, _>| async move {
    ///             while let ActoInput::Message(actor) = cell.recv().await {
    ///                 let actor_ref = cell.supervise(actor);
    ///                 // use reference to shut it down at a later time
    ///             }
    ///             // if any of them fail, shut all of them down
    ///         }
    ///     );
    ///     let actor = cell.spawn("actor", |mut cell: ActoCell<_, _>| async move {
    ///         while let ActoInput::Message(msg) = cell.recv().await {
    ///             if let ActorCommand::Shutdown(_) = msg {
    ///                 break;
    ///             }
    ///         }
    ///     });
    ///     let actor_ref = actor.me.clone();
    ///     supervisor.send(actor.contramap(ActorCommand::Shutdown));
    ///     // do stuff with actor_ref
    /// }
    /// ```
    pub fn contramap<M2: Send + 'static>(
        self,
        f: impl Fn(M2) -> M + Send + Sync + 'static,
    ) -> SupervisionRef<M2, H> {
        let Self { me, handle } = self;
        let me = me.contramap(f);
        SupervisionRef { me, handle }
    }

    /// Map the return type of the contained [`ActoHandle`] to match the intended supervisor.
    ///
    /// ```rust
    /// # use acto::{ActoCell, ActoInput, ActoRef, ActoRuntime, SupervisionRef, AcTokio, ActoHandle};
    /// async fn top_level(mut cell: ActoCell<(), impl ActoRuntime, String>) -> ActoInput<(), String> {
    ///     let actor = cell.spawn("actor", |mut cell: ActoCell<(), _>| async move {
    ///         // some async computation that leads to the result
    ///         42
    ///     });
    ///     // cannot supervise without transforming result to a String
    ///     let ar = cell.supervise(actor.map_handle(|number| number.to_string()));
    ///     // now do something with the actor reference
    /// #   cell.recv().await
    /// }
    /// # let sys = AcTokio::new("doc", 1).unwrap();
    /// # let ah = sys.spawn_actor("top", top_level);
    /// # let ah = ah.handle;
    /// # let ActoInput::Supervision { name, result, ..} = sys.with_rt(|rt| rt.block_on(ah.join())).unwrap().unwrap() else { panic!("wat") };
    /// # assert!(name.starts_with("actor(doc/"));
    /// # assert_eq!(result.unwrap(), "42");
    /// ```
    pub fn map_handle<S, F>(self, f: F) -> SupervisionRef<M, MappedActoHandle<H, F, S>>
    where
        F: FnOnce(H::Output) -> S + Send + Sync + Unpin + 'static,
        S: Send + 'static,
    {
        SupervisionRef {
            me: self.me,
            handle: MappedActoHandle::new(self.handle, f),
        }
    }
}
