use crate::behaviour::peer_store::store::Store;
use crate::error::{Error, Protocol};
use crate::task::ConnexaTask;
use crate::types::StreamCommand;
use libp2p::swarm::NetworkBehaviour;
use std::fmt::Debug;

impl<X, C: NetworkBehaviour, S, T, K> ConnexaTask<X, C, S, T, K>
where
    X: Default + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub fn process_stream_command(&mut self, command: StreamCommand) {
        let swarm = self.swarm.as_mut().unwrap();
        match command {
            StreamCommand::NewStream { protocol, resp } => {
                let Some(stream) = swarm.behaviour_mut().stream.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Stream,
                    }));
                    return;
                };

                let _ = resp.send(
                    stream
                        .new_control()
                        .accept(protocol)
                        .map_err(|e| std::io::Error::other(e).into()),
                );
            }
            StreamCommand::ControlHandle { resp } => {
                let Some(stream) = swarm.behaviour_mut().stream.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Stream,
                    }));
                    return;
                };

                let _ = resp.send(Ok(stream.new_control()));
            }
        }
    }
}
