use crate::{database::posts::core::OkuPost, fs::replica::ReplicaListItem};
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};

/// High-level events emitted when the local states of replicas have changed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReplicaEvent {
    /// Emitted when the node starts.
    Initialised,
    /// Emitted when replicas have been created.
    Created(NamespaceId),
    /// Emitted when replicas have been dropped locally.
    Deleted(NamespaceId),
    /// Emitted when replicas have been imported.
    Imported(ReplicaListItem),
    /// Emitted when replicas have been synchronised with peers.
    Synced(ReplicaListItem),
}

/// High-level events emitted when the local records of OkuNet posts have changed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OkuNetPostEvent {
    /// Emitted when the node starts.
    Initialised,
    /// Emitted when a post has been created or updated.
    Written(OkuPost),
    /// Emitted when a post has been deleted locally.
    Deleted(OkuPost),
    /// Emitted when the state of a post has been synchronised with peers.
    Synced(OkuPost),
}
