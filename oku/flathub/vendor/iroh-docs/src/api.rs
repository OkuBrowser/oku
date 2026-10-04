//! irpc-based RPC implementation for docs.

#![allow(missing_docs)]

use std::{
    future::Future,
    path::Path,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{ready, Poll},
};

use anyhow::Result;
use bytes::Bytes;
use iroh::EndpointAddr;
use iroh_blobs::{
    api::blobs::{AddPathOptions, AddProgressItem, ExportMode, ExportOptions, ExportProgress},
    Hash,
};
use n0_future::{FutureExt, Stream, StreamExt};

use self::{
    actor::RpcActor,
    protocol::{
        AddrInfoOptions, AuthorCreateRequest, AuthorDeleteRequest, AuthorExportRequest,
        AuthorGetDefaultRequest, AuthorImportRequest, AuthorListRequest, AuthorSetDefaultRequest,
        CloseRequest, CreateRequest, DelRequest, DocsProtocol, DropRequest,
        GetDownloadPolicyRequest, GetExactRequest, GetManyRequest, GetSyncPeersRequest,
        ImportRequest, LeaveRequest, ListRequest, OpenRequest, SetDownloadPolicyRequest,
        SetHashRequest, SetRequest, ShareMode, ShareRequest, StartSyncRequest, StatusRequest,
        SubscribeRequest,
    },
};
use crate::{
    actor::OpenState,
    engine::{Engine, LiveEvent},
    store::{DownloadPolicy, Query},
    Author, AuthorId, Capability, CapabilityKind, DocTicket, Entry, NamespaceId, PeerIdBytes,
};

pub(crate) mod actor;
pub mod protocol;

pub type RpcError = serde_error::Error;
pub type RpcResult<T> = std::result::Result<T, RpcError>;

type Client = irpc::Client<DocsProtocol>;

/// API wrapper for the docs service
#[derive(Debug, Clone)]
pub struct DocsApi {
    pub(crate) inner: Client,
}

impl DocsApi {
    /// Create a new docs API from an engine
    pub fn spawn(engine: Arc<Engine>) -> Self {
        RpcActor::spawn(engine)
    }

    /// Connect to a remote docs service
    #[cfg(feature = "rpc")]
    pub fn connect(endpoint: noq::Endpoint, addr: std::net::SocketAddr) -> Result<DocsApi> {
        Ok(DocsApi {
            inner: Client::noq(endpoint, addr),
        })
    }

    /// Listen for incoming RPC connections
    #[cfg(feature = "rpc")]
    pub fn listen(
        &self,
        endpoint: noq::Endpoint,
    ) -> Result<n0_future::task::AbortOnDropHandle<()>> {
        use anyhow::Context;
        let local = self
            .inner
            .as_local()
            .context("cannot listen on remote API")?;
        let handler: irpc::rpc::Handler<DocsProtocol> = Arc::new(move |msg, _rx, tx| {
            let local = local.clone();
            Box::pin(async move {
                match msg {
                    DocsProtocol::Open(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Close(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Status(msg) => local.send((msg, tx)).await,
                    DocsProtocol::List(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Create(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Drop(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Import(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Set(msg) => local.send((msg, tx)).await,
                    DocsProtocol::SetHash(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Get(msg) => local.send((msg, tx)).await,
                    DocsProtocol::GetExact(msg) => local.send((msg, tx)).await,
                    // DocsProtocol::ImportFile(msg) => local.send((msg, tx)).await,
                    // DocsProtocol::ExportFile(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Del(msg) => local.send((msg, tx)).await,
                    DocsProtocol::StartSync(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Leave(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Share(msg) => local.send((msg, tx)).await,
                    DocsProtocol::Subscribe(msg) => local.send((msg, tx)).await,
                    DocsProtocol::GetDownloadPolicy(msg) => local.send((msg, tx)).await,
                    DocsProtocol::SetDownloadPolicy(msg) => local.send((msg, tx)).await,
                    DocsProtocol::GetSyncPeers(msg) => local.send((msg, tx)).await,
                    DocsProtocol::AuthorList(msg) => local.send((msg, tx)).await,
                    DocsProtocol::AuthorCreate(msg) => local.send((msg, tx)).await,
                    DocsProtocol::AuthorGetDefault(msg) => local.send((msg, tx)).await,
                    DocsProtocol::AuthorSetDefault(msg) => local.send((msg, tx)).await,
                    DocsProtocol::AuthorImport(msg) => local.send((msg, tx)).await,
                    DocsProtocol::AuthorExport(msg) => local.send((msg, tx)).await,
                    DocsProtocol::AuthorDelete(msg) => local.send((msg, tx)).await,
                }
            })
        });
        let join_handle = n0_future::task::spawn(irpc::rpc::listen(endpoint, handler));
        Ok(n0_future::task::AbortOnDropHandle::new(join_handle))
    }

    /// Creates a new document author.
    ///
    /// You likely want to save the returned [`AuthorId`] somewhere so that you can use this author
    /// again.
    ///
    /// If you need only a single author, use [`Self::author_default`].
    pub async fn author_create(&self) -> Result<AuthorId> {
        let response = self.inner.rpc(AuthorCreateRequest).await??;
        Ok(response.author_id)
    }

    /// Returns the default document author of this node.
    ///
    /// On persistent nodes, the author is created on first start and its public key is saved
    /// in the data directory.
    ///
    /// The default author can be set with [`Self::author_set_default`].
    pub async fn author_default(&self) -> Result<AuthorId> {
        let response = self.inner.rpc(AuthorGetDefaultRequest).await??;
        Ok(response.author_id)
    }

    /// Sets the node-wide default author.
    ///
    /// If the author does not exist, an error is returned.
    ///
    /// On a persistent node, the author id will be saved to a file in the data directory and
    /// reloaded after a restart.
    pub async fn author_set_default(&self, author_id: AuthorId) -> Result<()> {
        self.inner
            .rpc(AuthorSetDefaultRequest { author_id })
            .await??;
        Ok(())
    }

    /// Lists document authors for which we have a secret key.
    ///
    /// It's only possible to create writes from authors that we have the secret key of.
    pub async fn author_list(&self) -> Result<impl Stream<Item = Result<AuthorId>>> {
        let stream = self.inner.server_streaming(AuthorListRequest, 64).await?;
        Ok(stream.into_stream().map(|res| match res {
            Err(err) => Err(err.into()),
            Ok(Err(err)) => Err(err.into()),
            Ok(Ok(res)) => Ok(res.author_id),
        }))
    }

    /// Exports the given author.
    ///
    /// Warning: The [`Author`] struct contains sensitive data.
    pub async fn author_export(&self, author: AuthorId) -> Result<Option<Author>> {
        let response = self.inner.rpc(AuthorExportRequest { author }).await??;
        Ok(response.author)
    }

    /// Imports the given author.
    ///
    /// Warning: The [`Author`] struct contains sensitive data.
    pub async fn author_import(&self, author: Author) -> Result<()> {
        self.inner.rpc(AuthorImportRequest { author }).await??;
        Ok(())
    }

    /// Deletes the given author by id.
    ///
    /// Warning: This permanently removes this author.
    ///
    /// Returns an error if attempting to delete the default author.
    pub async fn author_delete(&self, author: AuthorId) -> Result<()> {
        self.inner.rpc(AuthorDeleteRequest { author }).await??;
        Ok(())
    }

    /// Creates a new document.
    pub async fn create(&self) -> Result<Doc> {
        let response = self.inner.rpc(CreateRequest).await??;
        Ok(Doc::new(self.inner.clone(), response.id))
    }

    /// Deletes a document from the local node.
    ///
    /// This is a destructive operation. Both the document secret key and all entries in the
    /// document will be permanently deleted from the node's storage. Content blobs will be deleted
    /// through garbage collection unless they are referenced from another document or tag.
    pub async fn drop_doc(&self, doc_id: NamespaceId) -> Result<()> {
        self.inner.rpc(DropRequest { doc_id }).await??;
        Ok(())
    }

    /// Imports a document from a namespace capability.
    ///
    /// This does not start sync automatically. Use [`Doc::start_sync`] to start sync.
    pub async fn import_namespace(&self, capability: Capability) -> Result<Doc> {
        let response = self.inner.rpc(ImportRequest { capability }).await??;
        Ok(Doc::new(self.inner.clone(), response.doc_id))
    }

    /// Imports a document from a ticket and joins all peers in the ticket.
    pub async fn import(&self, ticket: DocTicket) -> Result<Doc> {
        let DocTicket { capability, nodes } = ticket;
        let doc = self.import_namespace(capability).await?;
        doc.start_sync(nodes).await?;
        Ok(doc)
    }

    /// Imports a document from a ticket, creates a subscription stream and joins all peers in the ticket.
    ///
    /// Returns the [`Doc`] and a [`Stream`] of [`LiveEvent`]s.
    ///
    /// The subscription stream is created before the sync is started, so the first call to this
    /// method after starting the node is guaranteed to not miss any sync events.
    pub async fn import_and_subscribe(
        &self,
        ticket: DocTicket,
    ) -> Result<(Doc, impl Stream<Item = Result<LiveEvent>>)> {
        let DocTicket { capability, nodes } = ticket;
        let response = self.inner.rpc(ImportRequest { capability }).await??;
        let doc = Doc::new(self.inner.clone(), response.doc_id);
        let events = doc.subscribe().await?;
        doc.start_sync(nodes).await?;
        Ok((doc, events))
    }

    /// Lists all documents.
    pub async fn list(
        &self,
    ) -> Result<impl Stream<Item = Result<(NamespaceId, CapabilityKind)>> + Unpin + Send + 'static>
    {
        let stream = self.inner.server_streaming(ListRequest, 64).await?;
        let stream = Box::pin(stream.into_stream());
        Ok(stream.map(|res| match res {
            Err(err) => Err(err.into()),
            Ok(Err(err)) => Err(err.into()),
            Ok(Ok(res)) => Ok((res.id, res.capability)),
        }))
    }

    /// Returns a [`Doc`] client for a single document.
    ///
    /// Returns None if the document cannot be found.
    pub async fn open(&self, id: NamespaceId) -> Result<Option<Doc>> {
        self.inner.rpc(OpenRequest { doc_id: id }).await??;
        Ok(Some(Doc::new(self.inner.clone(), id)))
    }
}

/// Document handle
#[derive(Debug, Clone)]
pub struct Doc {
    inner: Client,
    namespace_id: NamespaceId,
    closed: Arc<AtomicBool>,
}

impl Doc {
    fn new(inner: Client, namespace_id: NamespaceId) -> Self {
        Self {
            inner,
            namespace_id,
            closed: Default::default(),
        }
    }

    /// Returns the document id of this doc.
    pub fn id(&self) -> NamespaceId {
        self.namespace_id
    }

    /// Closes the document.
    pub async fn close(&self) -> Result<()> {
        self.closed.store(true, Ordering::Relaxed);
        self.inner
            .rpc(CloseRequest {
                doc_id: self.namespace_id,
            })
            .await??;
        Ok(())
    }

    fn ensure_open(&self) -> Result<()> {
        if self.closed.load(Ordering::Relaxed) {
            Err(anyhow::anyhow!("document is closed"))
        } else {
            Ok(())
        }
    }

    /// Sets the content of a key to a byte array.
    pub async fn set_bytes(
        &self,
        author_id: AuthorId,
        key: impl Into<Bytes>,
        value: impl Into<Bytes>,
    ) -> Result<Hash> {
        self.ensure_open()?;
        let response = self
            .inner
            .rpc(SetRequest {
                doc_id: self.namespace_id,
                author_id,
                key: key.into(),
                value: value.into(),
            })
            .await??;
        Ok(response.entry.content_hash())
    }

    /// Sets an entry on the doc via its key, hash, and size.
    pub async fn set_hash(
        &self,
        author_id: AuthorId,
        key: impl Into<Bytes>,
        hash: Hash,
        size: u64,
    ) -> Result<()> {
        self.ensure_open()?;
        self.inner
            .rpc(SetHashRequest {
                doc_id: self.namespace_id,
                author_id,
                key: key.into(),
                hash,
                size,
            })
            .await??;
        Ok(())
    }

    /// Deletes entries that match the given `author` and key `prefix`.
    ///
    /// This inserts an empty entry with the key set to `prefix`, effectively clearing all other
    /// entries whose key starts with or is equal to the given `prefix`.
    ///
    /// Returns the number of entries deleted.
    pub async fn del(&self, author_id: AuthorId, prefix: impl Into<Bytes>) -> Result<usize> {
        self.ensure_open()?;
        let response = self
            .inner
            .rpc(DelRequest {
                doc_id: self.namespace_id,
                author_id,
                prefix: prefix.into(),
            })
            .await??;
        Ok(response.removed)
    }

    /// Returns an entry for a key and author.
    ///
    /// Optionally also returns the entry unless it is empty (i.e. a deletion marker).
    pub async fn get_exact(
        &self,
        author: AuthorId,
        key: impl AsRef<[u8]>,
        include_empty: bool,
    ) -> Result<Option<Entry>> {
        self.ensure_open()?;
        let response = self
            .inner
            .rpc(GetExactRequest {
                author,
                key: key.as_ref().to_vec().into(),
                doc_id: self.namespace_id,
                include_empty,
            })
            .await??;
        Ok(response.entry.map(|entry| entry.into()))
    }

    /// Returns all entries matching the query.
    pub async fn get_many(
        &self,
        query: impl Into<Query>,
    ) -> Result<impl Stream<Item = Result<Entry>>> {
        self.ensure_open()?;
        let stream = self
            .inner
            .server_streaming(
                GetManyRequest {
                    doc_id: self.namespace_id,
                    query: query.into(),
                },
                64,
            )
            .await?;
        Ok(stream.into_stream().map(|res| match res {
            Err(err) => Err(err.into()),
            Ok(Err(err)) => Err(err.into()),
            Ok(Ok(res)) => Ok(res.into()),
        }))
    }

    /// Returns a single entry.
    pub async fn get_one(&self, query: impl Into<Query>) -> Result<Option<Entry>> {
        self.ensure_open()?;
        let stream = self.get_many(query).await?;
        tokio::pin!(stream);
        n0_future::StreamExt::next(&mut stream).await.transpose()
    }

    /// Shares this document with peers over a ticket.
    pub async fn share(&self, mode: ShareMode, addr_options: AddrInfoOptions) -> Result<DocTicket> {
        self.ensure_open()?;
        let response = self
            .inner
            .rpc(ShareRequest {
                doc_id: self.namespace_id,
                mode,
                addr_options,
            })
            .await??;
        Ok(response.0)
    }

    /// Starts to sync this document with a list of peers.
    pub async fn start_sync(&self, peers: Vec<EndpointAddr>) -> Result<()> {
        self.ensure_open()?;
        self.inner
            .rpc(StartSyncRequest {
                doc_id: self.namespace_id,
                peers,
            })
            .await??;
        Ok(())
    }

    /// Stops the live sync for this document.
    pub async fn leave(&self) -> Result<()> {
        self.ensure_open()?;
        self.inner
            .rpc(LeaveRequest {
                doc_id: self.namespace_id,
            })
            .await??;
        Ok(())
    }

    /// Subscribes to events for this document.
    pub async fn subscribe(
        &self,
    ) -> Result<impl Stream<Item = Result<LiveEvent>> + Send + Unpin + 'static> {
        self.ensure_open()?;
        let stream = self
            .inner
            .server_streaming(
                SubscribeRequest {
                    doc_id: self.namespace_id,
                },
                64,
            )
            .await?;
        Ok(Box::pin(stream.into_stream().map(|res| match res {
            Err(err) => Err(err.into()),
            Ok(Err(err)) => Err(err.into()),
            Ok(Ok(res)) => Ok(res.event),
        })))
    }

    /// Returns status info for this document
    pub async fn status(&self) -> Result<OpenState> {
        self.ensure_open()?;
        let response = self
            .inner
            .rpc(StatusRequest {
                doc_id: self.namespace_id,
            })
            .await??;
        Ok(response.status)
    }

    /// Sets the download policy for this document
    pub async fn set_download_policy(&self, policy: DownloadPolicy) -> Result<()> {
        self.ensure_open()?;
        self.inner
            .rpc(SetDownloadPolicyRequest {
                doc_id: self.namespace_id,
                policy,
            })
            .await??;
        Ok(())
    }

    /// Returns the download policy for this document
    pub async fn get_download_policy(&self) -> Result<DownloadPolicy> {
        self.ensure_open()?;
        let response = self
            .inner
            .rpc(GetDownloadPolicyRequest {
                doc_id: self.namespace_id,
            })
            .await??;
        Ok(response.policy)
    }

    /// Returns sync peers for this document
    pub async fn get_sync_peers(&self) -> Result<Option<Vec<PeerIdBytes>>> {
        self.ensure_open()?;
        let response = self
            .inner
            .rpc(GetSyncPeersRequest {
                doc_id: self.namespace_id,
            })
            .await??;
        Ok(response.peers)
    }

    /// Adds an entry from an absolute file path
    pub async fn import_file(
        &self,
        blobs: &iroh_blobs::api::Store,
        author: AuthorId,
        key: Bytes,
        path: impl AsRef<Path>,
        import_mode: iroh_blobs::api::blobs::ImportMode,
    ) -> Result<ImportFileProgress> {
        self.ensure_open()?;
        let progress = blobs.add_path_with_opts(AddPathOptions {
            path: path.as_ref().to_owned(),
            format: iroh_blobs::BlobFormat::Raw,
            mode: import_mode,
        });
        let stream = progress.stream().await;
        let doc = self.clone();
        let ctx = EntryContext {
            doc,
            author,
            key,
            size: None,
        };
        Ok(ImportFileProgress(ImportInner::Blobs(
            Box::pin(stream),
            Some(ctx),
        )))
    }

    /// Exports an entry as a file to a given absolute path.
    pub async fn export_file(
        &self,
        blobs: &iroh_blobs::api::Store,
        entry: Entry,
        path: impl AsRef<Path>,
        mode: ExportMode,
    ) -> Result<ExportProgress> {
        self.ensure_open()?;
        let hash = entry.content_hash();
        let progress = blobs.export_with_opts(ExportOptions {
            hash,
            mode,
            target: path.as_ref().to_path_buf(),
        });
        Ok(progress)
    }
}

#[derive(Debug)]
pub enum ImportFileProgressItem {
    Error(anyhow::Error),
    Blobs(AddProgressItem),
    Done(ImportFileOutcome),
}

#[derive(Debug)]
pub struct ImportFileProgress(ImportInner);

#[derive(derive_more::Debug)]
enum ImportInner {
    #[debug("Blobs")]
    Blobs(
        n0_future::boxed::BoxStream<AddProgressItem>,
        Option<EntryContext>,
    ),
    #[debug("Entry")]
    Entry(n0_future::boxed::BoxFuture<Result<ImportFileOutcome>>),
    Done,
}

struct EntryContext {
    doc: Doc,
    author: AuthorId,
    key: Bytes,
    size: Option<u64>,
}

impl Stream for ImportFileProgress {
    type Item = ImportFileProgressItem;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.0 {
            ImportInner::Blobs(ref mut progress, ref mut context) => {
                match ready!(progress.poll_next(cx)) {
                    Some(item) => match item {
                        AddProgressItem::Size(size) => {
                            context
                                .as_mut()
                                .expect("Size must be emitted before done")
                                .size = Some(size);
                            Poll::Ready(Some(ImportFileProgressItem::Blobs(AddProgressItem::Size(
                                size,
                            ))))
                        }
                        AddProgressItem::Error(err) => {
                            *this = Self(ImportInner::Done);
                            Poll::Ready(Some(ImportFileProgressItem::Error(err.into())))
                        }
                        AddProgressItem::Done(tag) => {
                            let EntryContext {
                                doc,
                                author,
                                key,
                                size,
                            } = context
                                .take()
                                .expect("AddProgressItem::Done may be emitted only once");
                            let size = size.expect("Size must be emitted before done");
                            let hash = tag.hash();
                            *this = Self(ImportInner::Entry(Box::pin(async move {
                                doc.set_hash(author, key.clone(), hash, size).await?;
                                Ok(ImportFileOutcome { hash, size, key })
                            })));
                            Poll::Ready(Some(ImportFileProgressItem::Blobs(AddProgressItem::Done(
                                tag,
                            ))))
                        }
                        item => Poll::Ready(Some(ImportFileProgressItem::Blobs(item))),
                    },
                    None => todo!(),
                }
            }
            ImportInner::Entry(ref mut fut) => {
                let res = ready!(fut.poll(cx));
                *this = Self(ImportInner::Done);
                match res {
                    Ok(outcome) => Poll::Ready(Some(ImportFileProgressItem::Done(outcome))),
                    Err(err) => Poll::Ready(Some(ImportFileProgressItem::Error(err))),
                }
            }
            ImportInner::Done => Poll::Ready(None),
        }
    }
}

impl Future for ImportFileProgress {
    type Output = Result<ImportFileOutcome>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.as_mut().poll_next(cx) {
                Poll::Ready(Some(item)) => match item {
                    ImportFileProgressItem::Error(error) => return Poll::Ready(Err(error)),
                    ImportFileProgressItem::Blobs(_add_progress_item) => continue,
                    ImportFileProgressItem::Done(outcome) => return Poll::Ready(Ok(outcome)),
                },
                Poll::Ready(None) => {
                    return Poll::Ready(Err(anyhow::anyhow!(
                        "ImportFileProgress polled after completion"
                    )));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Outcome of a [`Doc::import_file`] operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFileOutcome {
    /// The hash of the entry's content
    pub hash: Hash,
    /// The size of the entry
    pub size: u64,
    /// The key of the entry
    pub key: Bytes,
}
