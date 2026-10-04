//! Wraps a [`ProtocolHandler`] with an access check, refusing connections from
//! endpoints that the provided limiter function rejects.

use std::{future::Future, sync::Arc};

use iroh::{
    EndpointId,
    endpoint::{Accepting, Connection},
    protocol::{AcceptError, ProtocolHandler},
};
use n0_error::e;

/// Wraps an existing protocol, limiting its access,
/// based on the provided function.
///
/// Any refused connection will be closed with an error code of `0` and reason `not allowed`.
#[derive(derive_more::Debug, Clone)]
pub struct AccessLimit<P: ProtocolHandler + Clone> {
    proto: P,
    #[debug("limiter")]
    limiter: Arc<dyn Fn(EndpointId) -> bool + Send + Sync + 'static>,
}

impl<P: ProtocolHandler + Clone> AccessLimit<P> {
    /// Create a new `AccessLimit`.
    ///
    /// The function should return `true` for endpoints that are allowed to
    /// connect, and `false` otherwise.
    pub fn new<F>(proto: P, limiter: F) -> Self
    where
        F: Fn(EndpointId) -> bool + Send + Sync + 'static,
    {
        Self {
            proto,
            limiter: Arc::new(limiter),
        }
    }
}

impl<P: ProtocolHandler + Clone> ProtocolHandler for AccessLimit<P> {
    fn on_accepting(
        &self,
        accepting: Accepting,
    ) -> impl Future<Output = Result<Connection, AcceptError>> + Send {
        self.proto.on_accepting(accepting)
    }

    async fn accept(&self, conn: Connection) -> Result<(), AcceptError> {
        let remote = conn.remote_id();
        let is_allowed = (self.limiter)(remote);
        if !is_allowed {
            conn.close(0u32.into(), b"not allowed");
            return Err(e!(AcceptError::NotAllowed));
        }
        self.proto.accept(conn).await?;
        Ok(())
    }

    fn shutdown(&self) -> impl Future<Output = ()> + Send {
        self.proto.shutdown()
    }
}

#[cfg(test)]
mod tests {
    use iroh::{
        Endpoint, EndpointAddr,
        endpoint::{
            BeforeConnectOutcome, ConnectError, ConnectWithOptsError, EndpointHooks, presets,
        },
        protocol::Router,
    };
    use n0_error::{Result, StdResultExt};

    use super::*;

    #[derive(Debug, Clone)]
    struct Echo;

    const ECHO_ALPN: &[u8] = b"/iroh/echo/1";

    impl ProtocolHandler for Echo {
        async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
            let (mut send, mut recv) = connection.accept_bi().await?;
            let _bytes_sent = tokio::io::copy(&mut recv, &mut send).await?;
            send.finish()?;
            connection.closed().await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_limiter_router() -> Result {
        let e1 = Endpoint::bind(presets::Minimal).await?;
        // deny all access
        let proto = AccessLimit::new(Echo, |_endpoint_id| false);
        let r1 = Router::builder(e1.clone()).accept(ECHO_ALPN, proto).spawn();

        let addr1 = r1.endpoint().addr();
        let e2 = Endpoint::bind(presets::Minimal).await?;

        let conn = e2.connect(addr1, ECHO_ALPN).await?;

        let (_send, mut recv) = conn.open_bi().await.anyerr()?;
        let response = recv.read_to_end(1000).await.unwrap_err();
        assert!(format!("{response:#?}").contains("not allowed"));

        r1.shutdown().await.anyerr()?;
        e2.close().await;

        Ok(())
    }

    #[tokio::test]
    async fn test_limiter_hook() -> Result {
        #[derive(Debug, Default)]
        struct LimitHook;
        impl EndpointHooks for LimitHook {
            async fn before_connect<'a>(
                &'a self,
                _remote_addr: &'a EndpointAddr,
                alpn: &'a [u8],
            ) -> BeforeConnectOutcome {
                assert_eq!(alpn, ECHO_ALPN);

                // deny all access
                BeforeConnectOutcome::Reject
            }
        }

        let e1 = Endpoint::bind(presets::Minimal).await?;

        let r1 = Router::builder(e1.clone()).accept(ECHO_ALPN, Echo).spawn();

        let addr1 = r1.endpoint().addr();
        let e2 = Endpoint::builder(presets::Minimal)
            .hooks(LimitHook)
            .bind()
            .await?;

        let conn_err = e2.connect(addr1, ECHO_ALPN).await.unwrap_err();

        assert!(matches!(
            conn_err,
            ConnectError::Connect {
                source: ConnectWithOptsError::LocallyRejected { .. },
                ..
            }
        ));

        r1.shutdown().await.anyerr()?;
        e2.close().await;

        Ok(())
    }
}
