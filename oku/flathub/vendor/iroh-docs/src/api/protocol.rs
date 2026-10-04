//! Protocol definitions for irpc-based RPC.

use std::path::PathBuf;

use bytes::Bytes;
use iroh::EndpointAddr;
use iroh_blobs::{api::blobs::ExportMode, Hash};
use irpc::{
    channel::{mpsc, oneshot},
    rpc_requests,
};
use serde::{Deserialize, Serialize};

use super::RpcResult;
use crate::{
    actor::OpenState,
    engine::LiveEvent,
    store::{DownloadPolicy, Query},
    Author, AuthorId, Capability, CapabilityKind, DocTicket, Entry, NamespaceId, PeerIdBytes,
    SignedEntry,
};

/// Progress during import operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportProgress {
    /// Found the blob
    Found { size: u64 },
    /// Progress
    Progress { offset: u64 },
    /// Done
    Done { hash: Hash },
    /// All done
    AllDone,
}

/// Mode for sharing documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShareMode {
    /// Share with read access
    Read,
    /// Share with write access
    Write,
}

// Request types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenRequest {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloseRequest {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloseResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusRequest {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusResponse {
    pub status: OpenState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListRequest;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListResponse {
    pub id: NamespaceId,
    pub capability: CapabilityKind,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateRequest;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateResponse {
    pub id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DropRequest {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DropResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportRequest {
    pub capability: Capability,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportResponse {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetRequest {
    pub doc_id: NamespaceId,
    pub author_id: AuthorId,
    pub key: Bytes,
    pub value: Bytes,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetResponse {
    pub entry: SignedEntry,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetHashRequest {
    pub doc_id: NamespaceId,
    pub author_id: AuthorId,
    pub key: Bytes,
    pub hash: Hash,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetHashResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetManyRequest {
    pub doc_id: NamespaceId,
    pub query: Query,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetExactRequest {
    pub doc_id: NamespaceId,
    pub key: Bytes,
    pub author: AuthorId,
    pub include_empty: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetExactResponse {
    pub entry: Option<SignedEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportFileRequest {
    pub doc_id: NamespaceId,
    pub author_id: AuthorId,
    pub key: Bytes,
    pub path: PathBuf,
    pub in_place: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportFileRequest {
    pub entry: Entry,
    pub path: PathBuf,
    pub mode: ExportMode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DelRequest {
    pub doc_id: NamespaceId,
    pub author_id: AuthorId,
    pub prefix: Bytes,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DelResponse {
    pub removed: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StartSyncRequest {
    pub doc_id: NamespaceId,
    pub peers: Vec<EndpointAddr>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StartSyncResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeaveRequest {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeaveResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShareRequest {
    pub doc_id: NamespaceId,
    pub mode: ShareMode,
    pub addr_options: AddrInfoOptions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShareResponse(pub DocTicket);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubscribeRequest {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubscribeResponse {
    pub event: LiveEvent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetDownloadPolicyRequest {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetDownloadPolicyResponse {
    pub policy: DownloadPolicy,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetDownloadPolicyRequest {
    pub doc_id: NamespaceId,
    pub policy: DownloadPolicy,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetDownloadPolicyResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetSyncPeersRequest {
    pub doc_id: NamespaceId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetSyncPeersResponse {
    pub peers: Option<Vec<PeerIdBytes>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorListRequest;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorListResponse {
    pub author_id: AuthorId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorCreateRequest;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorCreateResponse {
    pub author_id: AuthorId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorGetDefaultRequest;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorGetDefaultResponse {
    pub author_id: AuthorId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorSetDefaultRequest {
    pub author_id: AuthorId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorSetDefaultResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorImportRequest {
    pub author: Author,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorImportResponse {
    pub author_id: AuthorId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorExportRequest {
    pub author: AuthorId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorExportResponse {
    pub author: Option<Author>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorDeleteRequest {
    pub author: AuthorId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorDeleteResponse;

// Use the macro to generate both the DocsProtocol and DocsMessage enums
// plus implement Channels for each type
#[rpc_requests(message = DocsMessage, rpc_feature = "rpc")]
#[derive(Serialize, Deserialize, Debug)]
pub enum DocsProtocol {
    #[rpc(tx = oneshot::Sender<RpcResult<OpenResponse>>)]
    Open(OpenRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<CloseResponse>>)]
    Close(CloseRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<StatusResponse>>)]
    Status(StatusRequest),
    #[rpc(tx = mpsc::Sender<RpcResult<ListResponse>>)]
    List(ListRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<CreateResponse>>)]
    Create(CreateRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<DropResponse>>)]
    Drop(DropRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<ImportResponse>>)]
    Import(ImportRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<SetResponse>>)]
    Set(SetRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<SetHashResponse>>)]
    SetHash(SetHashRequest),
    #[rpc(tx = mpsc::Sender<RpcResult<SignedEntry>>)]
    Get(GetManyRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<GetExactResponse>>)]
    GetExact(GetExactRequest),
    // #[rpc(tx = mpsc::Sender<ImportProgress>)]
    // ImportFile(ImportFileRequest),
    // #[rpc(tx = mpsc::Sender<ExportProgress>)]
    // ExportFile(ExportFileRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<DelResponse>>)]
    Del(DelRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<StartSyncResponse>>)]
    StartSync(StartSyncRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<LeaveResponse>>)]
    Leave(LeaveRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<ShareResponse>>)]
    Share(ShareRequest),
    #[rpc(tx = mpsc::Sender<RpcResult<SubscribeResponse>>)]
    Subscribe(SubscribeRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<GetDownloadPolicyResponse>>)]
    GetDownloadPolicy(GetDownloadPolicyRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<SetDownloadPolicyResponse>>)]
    SetDownloadPolicy(SetDownloadPolicyRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<GetSyncPeersResponse>>)]
    GetSyncPeers(GetSyncPeersRequest),
    #[rpc(tx = mpsc::Sender<RpcResult<AuthorListResponse>>)]
    AuthorList(AuthorListRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<AuthorCreateResponse>>)]
    AuthorCreate(AuthorCreateRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<AuthorGetDefaultResponse>>)]
    AuthorGetDefault(AuthorGetDefaultRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<AuthorSetDefaultResponse>>)]
    AuthorSetDefault(AuthorSetDefaultRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<AuthorImportResponse>>)]
    AuthorImport(AuthorImportRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<AuthorExportResponse>>)]
    AuthorExport(AuthorExportRequest),
    #[rpc(tx = oneshot::Sender<RpcResult<AuthorDeleteResponse>>)]
    AuthorDelete(AuthorDeleteRequest),
}

/// Options to configure what is included in a [`iroh::EndpointAddr`].
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Debug,
    derive_more::Display,
    derive_more::FromStr,
    Serialize,
    Deserialize,
)]
pub enum AddrInfoOptions {
    /// Only the Node ID is added.
    ///
    /// This usually means that iroh-dns address lookup is used to find address information.
    #[default]
    Id,
    /// Includes the Node ID and both the relay URL, and the direct addresses.
    RelayAndAddresses,
    /// Includes the Node ID and the relay URL.
    Relay,
    /// Includes the Node ID and the direct addresses.
    Addresses,
}

impl AddrInfoOptions {
    /// Apply the options to the given address.
    pub fn apply(
        &self,
        iroh::EndpointAddr { id, addrs }: &iroh::EndpointAddr,
    ) -> iroh::EndpointAddr {
        match self {
            Self::Id => iroh::EndpointAddr::new(*id),
            Self::Relay => iroh::EndpointAddr {
                id: *id,
                addrs: addrs
                    .iter()
                    .filter(|addr| matches!(addr, iroh::TransportAddr::Relay(_)))
                    .cloned()
                    .collect(),
            },
            Self::Addresses => iroh::EndpointAddr {
                id: *id,
                addrs: addrs
                    .iter()
                    .filter(|addr| matches!(addr, iroh::TransportAddr::Ip(_)))
                    .cloned()
                    .collect(),
            },
            Self::RelayAndAddresses => iroh::EndpointAddr {
                id: *id,
                addrs: addrs.clone(),
            },
        }
    }
}
