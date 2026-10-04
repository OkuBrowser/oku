use std::sync::Arc;

use anyhow::anyhow;
use irpc::{channel::mpsc, LocalSender, WithChannels};
use n0_future::{task, StreamExt};
use tokio::sync::mpsc as tokio_mpsc;
use tracing::error;

use super::{
    protocol::{
        AuthorCreateRequest, AuthorCreateResponse, AuthorDeleteRequest, AuthorDeleteResponse,
        AuthorExportRequest, AuthorExportResponse, AuthorGetDefaultRequest,
        AuthorGetDefaultResponse, AuthorImportRequest, AuthorImportResponse, AuthorListRequest,
        AuthorListResponse, AuthorSetDefaultRequest, AuthorSetDefaultResponse, CloseRequest,
        CloseResponse, CreateRequest, CreateResponse, DelRequest, DelResponse, DocsMessage,
        DocsProtocol, DropRequest, DropResponse, GetDownloadPolicyRequest,
        GetDownloadPolicyResponse, GetExactRequest, GetExactResponse, GetManyRequest,
        GetSyncPeersRequest, GetSyncPeersResponse, ImportRequest, ImportResponse, LeaveRequest,
        LeaveResponse, ListRequest, ListResponse, OpenRequest, OpenResponse,
        SetDownloadPolicyRequest, SetDownloadPolicyResponse, SetHashRequest, SetHashResponse,
        SetRequest, SetResponse, ShareMode, ShareRequest, ShareResponse, StartSyncRequest,
        StartSyncResponse, StatusRequest, StatusResponse, SubscribeRequest, SubscribeResponse,
    },
    DocsApi, RpcError, RpcResult,
};
use crate::{engine::Engine, Author, DocTicket, NamespaceSecret, SignedEntry};

/// The docs RPC actor that handles incoming messages
pub(crate) struct RpcActor {
    pub(crate) recv: tokio::sync::mpsc::Receiver<DocsMessage>,
    pub(crate) engine: Arc<Engine>,
}

impl RpcActor {
    pub(crate) fn spawn(engine: Arc<Engine>) -> DocsApi {
        let (tx, rx) = tokio_mpsc::channel(64);
        let actor = Self { recv: rx, engine };
        task::spawn(actor.run());
        let local = LocalSender::<DocsProtocol>::from(tx);
        DocsApi {
            inner: local.into(),
        }
    }

    pub(crate) async fn run(mut self) {
        while let Some(msg) = self.recv.recv().await {
            tracing::trace!("handle rpc request: {msg:?}");
            self.handle(msg).await;
        }
    }

    pub(crate) async fn handle(&mut self, msg: DocsMessage) {
        match msg {
            DocsMessage::Open(open) => {
                let WithChannels { tx, inner, .. } = open;
                let result = self.doc_open(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Open response: {}", e);
                }
            }
            DocsMessage::Close(close) => {
                let WithChannels { tx, inner, .. } = close;
                let result = self.doc_close(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Close response: {}", e);
                }
            }
            DocsMessage::Status(status) => {
                let WithChannels { tx, inner, .. } = status;
                let result = self.doc_status(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Status response: {}", e);
                }
            }
            DocsMessage::List(list) => {
                let WithChannels { tx, inner, .. } = list;
                self.doc_list(inner, tx).await;
            }
            DocsMessage::Create(create) => {
                let WithChannels { tx, inner, .. } = create;
                let result = self.doc_create(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Create response: {}", e);
                }
            }
            DocsMessage::Drop(drop) => {
                let WithChannels { tx, inner, .. } = drop;
                let result = self.doc_drop(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Drop response: {}", e);
                }
            }
            DocsMessage::Import(import) => {
                let WithChannels { tx, inner, .. } = import;
                let result = self.doc_import(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Import response: {}", e);
                }
            }
            DocsMessage::Set(set) => {
                let WithChannels { tx, inner, .. } = set;
                let result = self.doc_set(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Set response: {}", e);
                }
            }
            DocsMessage::SetHash(set_hash) => {
                let WithChannels { tx, inner, .. } = set_hash;
                let result = self.doc_set_hash(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send SetHash response: {}", e);
                }
            }
            DocsMessage::Get(get) => {
                let WithChannels { tx, inner, .. } = get;
                self.doc_get_many(inner, tx).await;
            }
            DocsMessage::GetExact(get_exact) => {
                let WithChannels { tx, inner, .. } = get_exact;
                let result = self.doc_get_exact(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send GetExact response: {}", e);
                }
            }
            DocsMessage::Del(del) => {
                let WithChannels { tx, inner, .. } = del;
                let result = self.doc_del(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Del response: {}", e);
                }
            }
            DocsMessage::StartSync(start_sync) => {
                let WithChannels { tx, inner, .. } = start_sync;
                let result = self.doc_start_sync(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send StartSync response: {}", e);
                }
            }
            DocsMessage::Leave(leave) => {
                let WithChannels { tx, inner, .. } = leave;
                let result = self.doc_leave(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Leave response: {}", e);
                }
            }
            DocsMessage::Share(share) => {
                let WithChannels { tx, inner, .. } = share;
                let result = self.doc_share(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send Share response: {}", e);
                }
            }
            DocsMessage::Subscribe(subscribe) => {
                let WithChannels { tx, inner, .. } = subscribe;
                self.doc_subscribe(inner, tx).await;
            }
            DocsMessage::GetDownloadPolicy(get_policy) => {
                let WithChannels { tx, inner, .. } = get_policy;
                let result = self.doc_get_download_policy(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send GetDownloadPolicy response: {}", e);
                }
            }
            DocsMessage::SetDownloadPolicy(set_policy) => {
                let WithChannels { tx, inner, .. } = set_policy;
                let result = self.doc_set_download_policy(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send SetDownloadPolicy response: {}", e);
                }
            }
            DocsMessage::GetSyncPeers(get_peers) => {
                let WithChannels { tx, inner, .. } = get_peers;
                let result = self.doc_get_sync_peers(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send GetSyncPeers response: {}", e);
                }
            }
            DocsMessage::AuthorList(author_list) => {
                let WithChannels { tx, inner, .. } = author_list;
                self.author_list(inner, tx).await;
            }
            DocsMessage::AuthorCreate(author_create) => {
                let WithChannels { tx, inner, .. } = author_create;
                let result = self.author_create(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send AuthorCreate response: {}", e);
                }
            }
            DocsMessage::AuthorGetDefault(author_get_default) => {
                let WithChannels { tx, inner, .. } = author_get_default;
                let result = self.author_default(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send AuthorGetDefault response: {}", e);
                }
            }
            DocsMessage::AuthorSetDefault(author_set_default) => {
                let WithChannels { tx, inner, .. } = author_set_default;
                let result = self.author_set_default(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send AuthorSetDefault response: {}", e);
                }
            }
            DocsMessage::AuthorImport(author_import) => {
                let WithChannels { tx, inner, .. } = author_import;
                let result = self.author_import(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send AuthorImport response: {}", e);
                }
            }
            DocsMessage::AuthorExport(author_export) => {
                let WithChannels { tx, inner, .. } = author_export;
                let result = self.author_export(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send AuthorExport response: {}", e);
                }
            }
            DocsMessage::AuthorDelete(author_delete) => {
                let WithChannels { tx, inner, .. } = author_delete;
                let result = self.author_delete(inner).await;
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send AuthorDelete response: {}", e);
                }
            }
        }
    }
}

impl std::ops::Deref for RpcActor {
    type Target = Engine;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

impl RpcActor {
    pub(super) async fn author_create(
        &self,
        _req: AuthorCreateRequest,
    ) -> RpcResult<AuthorCreateResponse> {
        // TODO: pass rng
        let author = Author::new(&mut rand::rng());
        self.sync
            .import_author(author.clone())
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(AuthorCreateResponse {
            author_id: author.id(),
        })
    }

    pub(super) async fn author_default(
        &self,
        _req: AuthorGetDefaultRequest,
    ) -> RpcResult<AuthorGetDefaultResponse> {
        let author_id = self.default_author.get();
        Ok(AuthorGetDefaultResponse { author_id })
    }

    pub(super) async fn author_set_default(
        &self,
        req: AuthorSetDefaultRequest,
    ) -> RpcResult<AuthorSetDefaultResponse> {
        self.default_author
            .set(req.author_id, &self.sync)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(AuthorSetDefaultResponse)
    }

    pub(super) async fn author_list(
        &self,
        _req: AuthorListRequest,
        reply: mpsc::Sender<RpcResult<AuthorListResponse>>,
    ) {
        if let Err(err) = self.sync.list_authors(reply.clone()).await {
            reply.send(Err(RpcError::new(&*err))).await.ok();
        }
    }

    pub(super) async fn author_import(
        &self,
        req: AuthorImportRequest,
    ) -> RpcResult<AuthorImportResponse> {
        let author_id = self
            .sync
            .import_author(req.author)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(AuthorImportResponse { author_id })
    }

    pub(super) async fn author_export(
        &self,
        req: AuthorExportRequest,
    ) -> RpcResult<AuthorExportResponse> {
        let author = self
            .sync
            .export_author(req.author)
            .await
            .map_err(|e| RpcError::new(&*e))?;

        Ok(AuthorExportResponse { author })
    }

    pub(super) async fn author_delete(
        &self,
        req: AuthorDeleteRequest,
    ) -> RpcResult<AuthorDeleteResponse> {
        if req.author == self.default_author.get() {
            return Err(RpcError::new(&*anyhow!(
                "Deleting the default author is not supported"
            )));
        }
        self.sync
            .delete_author(req.author)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(AuthorDeleteResponse)
    }

    pub(super) async fn doc_create(&self, _req: CreateRequest) -> RpcResult<CreateResponse> {
        let namespace = NamespaceSecret::new(&mut rand::rng());
        let id = namespace.id();
        self.sync
            .import_namespace(namespace.into())
            .await
            .map_err(|e| RpcError::new(&*e))?;
        self.sync
            .open(id, Default::default())
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(CreateResponse { id })
    }

    pub(super) async fn doc_drop(&self, req: DropRequest) -> RpcResult<DropResponse> {
        let DropRequest { doc_id } = req;
        self.leave(doc_id, true)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        self.sync
            .drop_replica(doc_id)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(DropResponse {})
    }

    pub(super) async fn doc_list(
        &self,
        _req: ListRequest,
        reply: irpc::channel::mpsc::Sender<RpcResult<ListResponse>>,
    ) {
        if let Err(err) = self.sync.list_replicas(reply.clone()).await {
            reply.send(Err(RpcError::new(&*err))).await.ok();
        }
    }

    pub(super) async fn doc_open(&self, req: OpenRequest) -> RpcResult<OpenResponse> {
        self.sync
            .open(req.doc_id, Default::default())
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(OpenResponse {})
    }

    pub(super) async fn doc_close(&self, req: CloseRequest) -> RpcResult<CloseResponse> {
        tracing::debug!("close req received");
        self.sync
            .close(req.doc_id)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        tracing::debug!("close req handled");
        Ok(CloseResponse {})
    }

    pub(super) async fn doc_status(&self, req: StatusRequest) -> RpcResult<StatusResponse> {
        let status = self
            .sync
            .get_state(req.doc_id)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(StatusResponse { status })
    }

    pub(super) async fn doc_share(&self, req: ShareRequest) -> RpcResult<ShareResponse> {
        let ShareRequest {
            doc_id,
            mode,
            addr_options,
        } = req;
        let me = self.endpoint.addr();
        let me = addr_options.apply(&me);

        let capability = match mode {
            ShareMode::Read => crate::Capability::Read(doc_id),
            ShareMode::Write => {
                let secret = self
                    .sync
                    .export_secret_key(doc_id)
                    .await
                    .map_err(|e| RpcError::new(&*e))?;
                crate::Capability::Write(secret)
            }
        };
        self.start_sync(doc_id, vec![])
            .await
            .map_err(|e| RpcError::new(&*e))?;

        Ok(ShareResponse(DocTicket {
            capability,
            nodes: vec![me],
        }))
    }

    pub(super) async fn doc_subscribe(
        &self,
        req: SubscribeRequest,
        reply: irpc::channel::mpsc::Sender<RpcResult<SubscribeResponse>>,
    ) {
        let mut stream = match self
            .subscribe(req.doc_id)
            .await
            .map_err(|e| RpcError::new(&*e))
        {
            Ok(stream) => stream,
            Err(err) => {
                reply.send(Err(err)).await.ok();
                return;
            }
        };
        n0_future::task::spawn(async move {
            loop {
                tokio::select! {
                    msg = stream.next() => {
                        let Some(msg) = msg else {
                            break;
                        };
                        let msg = msg
                            .map_err(|err| RpcError::new(&*err))
                            .map(|event| SubscribeResponse { event });
                        if let Err(_err) = reply.send(msg).await {
                            break;
                        }
                    },
                    _ = reply.closed() => break,
                }
            }
        });
    }

    pub(super) async fn doc_import(&self, req: ImportRequest) -> RpcResult<ImportResponse> {
        let ImportRequest { capability } = req;
        let doc_id = self
            .sync
            .import_namespace(capability)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        self.sync
            .open(doc_id, Default::default())
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(ImportResponse { doc_id })
    }

    pub(super) async fn doc_start_sync(
        &self,
        req: StartSyncRequest,
    ) -> RpcResult<StartSyncResponse> {
        let StartSyncRequest { doc_id, peers } = req;
        self.start_sync(doc_id, peers)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(StartSyncResponse {})
    }

    pub(super) async fn doc_leave(&self, req: LeaveRequest) -> RpcResult<LeaveResponse> {
        let LeaveRequest { doc_id } = req;
        self.leave(doc_id, false)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(LeaveResponse {})
    }

    pub(super) async fn doc_set(&self, req: SetRequest) -> RpcResult<SetResponse> {
        let blobs_store = self.blob_store();
        let SetRequest {
            doc_id,
            author_id,
            key,
            value,
        } = req;
        let len = value.len();
        let tag = blobs_store
            .add_bytes(value)
            .temp_tag()
            .await
            .map_err(|e| RpcError::new(&e))?;
        self.sync
            .insert_local(doc_id, author_id, key.clone(), tag.hash(), len as u64)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        let entry = self
            .sync
            .get_exact(doc_id, author_id, key, false)
            .await
            .map_err(|e| RpcError::new(&*e))?
            .ok_or_else(|| RpcError::new(&*anyhow!("failed to get entry after insertion")))?;
        Ok(SetResponse { entry })
    }

    pub(super) async fn doc_del(&self, req: DelRequest) -> RpcResult<DelResponse> {
        let DelRequest {
            doc_id,
            author_id,
            prefix,
        } = req;
        let removed = self
            .sync
            .delete_prefix(doc_id, author_id, prefix)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(DelResponse { removed })
    }

    pub(super) async fn doc_set_hash(&self, req: SetHashRequest) -> RpcResult<SetHashResponse> {
        let SetHashRequest {
            doc_id,
            author_id,
            key,
            hash,
            size,
        } = req;
        self.sync
            .insert_local(doc_id, author_id, key.clone(), hash, size)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(SetHashResponse {})
    }

    pub(super) async fn doc_get_many(
        &self,
        req: GetManyRequest,
        reply: irpc::channel::mpsc::Sender<RpcResult<SignedEntry>>,
    ) {
        let GetManyRequest { doc_id, query } = req;
        if let Err(err) = self.sync.get_many(doc_id, query, reply.clone()).await {
            reply.send(Err(RpcError::new(&*err))).await.ok();
        }
    }

    pub(super) async fn doc_get_exact(&self, req: GetExactRequest) -> RpcResult<GetExactResponse> {
        let GetExactRequest {
            doc_id,
            author,
            key,
            include_empty,
        } = req;
        let entry = self
            .sync
            .get_exact(doc_id, author, key, include_empty)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(GetExactResponse { entry })
    }

    pub(super) async fn doc_set_download_policy(
        &self,
        req: SetDownloadPolicyRequest,
    ) -> RpcResult<SetDownloadPolicyResponse> {
        self.sync
            .set_download_policy(req.doc_id, req.policy)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(SetDownloadPolicyResponse {})
    }

    pub(super) async fn doc_get_download_policy(
        &self,
        req: GetDownloadPolicyRequest,
    ) -> RpcResult<GetDownloadPolicyResponse> {
        let policy = self
            .sync
            .get_download_policy(req.doc_id)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(GetDownloadPolicyResponse { policy })
    }

    pub(super) async fn doc_get_sync_peers(
        &self,
        req: GetSyncPeersRequest,
    ) -> RpcResult<GetSyncPeersResponse> {
        let peers = self
            .sync
            .get_sync_peers(req.doc_id)
            .await
            .map_err(|e| RpcError::new(&*e))?;
        Ok(GetSyncPeersResponse { peers })
    }
}
