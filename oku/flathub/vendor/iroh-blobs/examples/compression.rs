/// Example how to use compression with iroh-blobs
///
/// We create a derived protocol that compresses both requests and responses using lz4
/// or any other compression algorithm supported by async-compression.
mod common;
use std::{fmt::Debug, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use common::setup_logging;
use iroh::{endpoint::presets, protocol::ProtocolHandler};
use iroh_blobs::{
    api::Store,
    get::StreamPair,
    provider::{
        self,
        events::{ClientConnected, EventSender, HasErrorCode},
        handle_stream,
    },
    store::mem::MemStore,
    ticket::BlobTicket,
};
use tracing::debug;

use crate::common::get_or_generate_secret_key;

#[derive(Debug, Parser)]
#[command(version, about)]
pub enum Args {
    /// Limit requests by endpoint id
    Provide {
        /// Path for files to add.
        path: PathBuf,
    },
    /// Get a blob. Just for completeness sake.
    Get {
        /// Ticket for the blob to download
        ticket: BlobTicket,
        /// Path to save the blob to
        #[clap(long)]
        target: Option<PathBuf>,
    },
}

trait Compression: Clone + Send + Sync + Debug + 'static {
    const ALPN: &'static [u8];
    fn recv_stream(
        &self,
        stream: iroh::endpoint::RecvStream,
    ) -> impl iroh_blobs::util::RecvStream + Sync + 'static;
    fn send_stream(
        &self,
        stream: iroh::endpoint::SendStream,
    ) -> impl iroh_blobs::util::SendStream + Sync + 'static;
}

mod lz4 {
    use std::io;

    use async_compression::tokio::{bufread::Lz4Decoder, write::Lz4Encoder};
    use iroh::endpoint::VarInt;
    use iroh_blobs::util::{
        AsyncReadRecvStream, AsyncReadRecvStreamExtra, AsyncWriteSendStream,
        AsyncWriteSendStreamExtra,
    };
    use tokio::io::{AsyncRead, AsyncWrite, BufReader};

    struct SendStream(Lz4Encoder<iroh::endpoint::SendStream>);

    impl SendStream {
        pub fn new(inner: iroh::endpoint::SendStream) -> AsyncWriteSendStream<Self> {
            AsyncWriteSendStream::new(Self(Lz4Encoder::new(inner)))
        }
    }

    impl AsyncWriteSendStreamExtra for SendStream {
        fn inner(&mut self) -> &mut (impl AsyncWrite + Unpin + Send) {
            &mut self.0
        }

        fn reset(&mut self, code: VarInt) -> io::Result<()> {
            Ok(self.0.get_mut().reset(code)?)
        }

        async fn stopped(&mut self) -> io::Result<Option<VarInt>> {
            Ok(self.0.get_mut().stopped().await?)
        }

        fn id(&self) -> u64 {
            self.0.get_ref().id().index()
        }
    }

    struct RecvStream(Lz4Decoder<BufReader<iroh::endpoint::RecvStream>>);

    impl RecvStream {
        pub fn new(inner: iroh::endpoint::RecvStream) -> AsyncReadRecvStream<Self> {
            AsyncReadRecvStream::new(Self(Lz4Decoder::new(BufReader::new(inner))))
        }
    }

    impl AsyncReadRecvStreamExtra for RecvStream {
        fn inner(&mut self) -> &mut (impl AsyncRead + Unpin + Send) {
            &mut self.0
        }

        fn stop(&mut self, code: VarInt) -> io::Result<()> {
            Ok(self.0.get_mut().get_mut().stop(code)?)
        }

        fn id(&self) -> u64 {
            self.0.get_ref().get_ref().id().index()
        }
    }

    #[derive(Debug, Clone)]
    pub struct Compression;

    impl super::Compression for Compression {
        const ALPN: &[u8] = concat_const::concat_bytes!(b"lz4/", iroh_blobs::ALPN);
        fn recv_stream(
            &self,
            stream: iroh::endpoint::RecvStream,
        ) -> impl iroh_blobs::util::RecvStream + Sync + 'static {
            RecvStream::new(stream)
        }
        fn send_stream(
            &self,
            stream: iroh::endpoint::SendStream,
        ) -> impl iroh_blobs::util::SendStream + Sync + 'static {
            SendStream::new(stream)
        }
    }
}

#[derive(Debug, Clone)]
struct CompressedBlobsProtocol<C: Compression> {
    store: Store,
    events: EventSender,
    compression: C,
}

impl<C: Compression> CompressedBlobsProtocol<C> {
    fn new(store: &Store, events: EventSender, compression: C) -> Self {
        Self {
            store: store.clone(),
            events,
            compression,
        }
    }
}

impl<C: Compression> ProtocolHandler for CompressedBlobsProtocol<C> {
    async fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> std::result::Result<(), iroh::protocol::AcceptError> {
        let connection_id = connection.stable_id() as u64;
        if let Err(cause) = self
            .events
            .client_connected(|| ClientConnected {
                connection_id,
                endpoint_id: Some(connection.remote_id()),
            })
            .await
        {
            connection.close(cause.code(), cause.reason());
            debug!("closing connection: {cause}");
            return Ok(());
        }
        while let Ok((send, recv)) = connection.accept_bi().await {
            let send = self.compression.send_stream(send);
            let recv = self.compression.recv_stream(recv);
            let store = self.store.clone();
            let pair = provider::StreamPair::new(connection_id, recv, send, self.events.clone());
            tokio::spawn(handle_stream(pair, store));
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();
    let args = Args::parse();
    let secret = get_or_generate_secret_key()?;
    let endpoint = iroh::Endpoint::builder(presets::N0)
        .secret_key(secret)
        .bind()
        .await?;
    let compression = lz4::Compression;
    match args {
        Args::Provide { path } => {
            let store = MemStore::new();
            let tag = store.add_path(path).await?;
            let blobs = CompressedBlobsProtocol::new(&store, EventSender::DEFAULT, compression);
            let router = iroh::protocol::Router::builder(endpoint.clone())
                .accept(lz4::Compression::ALPN, blobs)
                .spawn();
            let ticket = BlobTicket::new(endpoint.id().into(), tag.hash, tag.format);
            println!("Serving blob with hash {}", tag.hash);
            println!("Ticket: {ticket}");
            println!("Node is running. Press Ctrl-C to exit.");
            tokio::signal::ctrl_c().await?;
            println!("Shutting down.");
            router.shutdown().await?;
        }
        Args::Get { ticket, target } => {
            let store = MemStore::new();
            let conn = endpoint
                .connect(ticket.addr().clone(), lz4::Compression::ALPN)
                .await?;
            let connection_id = conn.stable_id() as u64;
            let (send, recv) = conn.open_bi().await?;
            let send = compression.send_stream(send);
            let recv = compression.recv_stream(recv);
            let sp = StreamPair::new(connection_id, recv, send);
            let _stats = store.remote().fetch(sp, ticket.hash_and_format()).await?;
            if let Some(target) = target {
                let size = store.export(ticket.hash(), &target).await?;
                println!("Wrote {} bytes to {}", size, target.display());
            } else {
                println!("Hash: {}", ticket.hash());
            }
        }
    }
    endpoint.close().await;
    Ok(())
}
