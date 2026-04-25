use super::*;
use crate::database::core::DATABASE;
use crate::database::dht::ReplicaAnnouncement;
use crate::error::{OkuDiscoveryError, OkuFsError, OkuFuseError};
use anyhow::anyhow;
use futures::{pin_mut, StreamExt};
use iroh_docs::api::protocol::AddrInfoOptions;
use iroh_docs::api::protocol::ShareMode;
use iroh_docs::engine::LiveEvent;
use iroh_docs::store::FilterKind;
use iroh_docs::sync::CapabilityKind;
use iroh_docs::NamespaceId;
use iroh_docs::{AuthorId, DocTicket};
use iroh_tickets::Ticket;
use log::{error, info};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::collections::HashSet;
use std::path::PathBuf;
use util::{merge_tickets, path_to_entry_prefix};

impl OkuFs {
    /// Creates a new replica in the file system.
    ///
    /// # Returns
    ///
    /// The ID of the new replica, being its public key.
    pub async fn create_replica(&self) -> miette::Result<NamespaceId> {
        let docs_client = &self.docs;
        let new_document = docs_client.create().await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotCreateReplica
        })?;
        let document_id = new_document.id();
        new_document.close().await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotExitReplica
        })?;
        self.replica_sender.send_replace(());
        Ok(document_id)
    }

    /// Returns whether a replica is a home replica.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica.
    ///
    /// # Returns
    ///
    /// If the given replica is a home replica.
    pub async fn is_home_replica(&self, namespace_id: &NamespaceId) -> bool {
        let replica_capability = self.get_replica_capability(namespace_id).await;
        match replica_capability {
            Err(e) => {
                error!("{e}");
                false
            }
            Ok(CapabilityKind::Read) => false,
            Ok(CapabilityKind::Write) => {
                let authors_list = self.docs.author_list().await;
                match authors_list {
                    Err(e) => {
                        error!("{e}");
                        false
                    }
                    Ok(authors_list) => {
                        let authors: HashSet<AuthorId> = authors_list
                            .filter_map(|x| async move { x.ok() })
                            .collect()
                            .await;
                        authors.contains(&AuthorId::from(namespace_id.as_bytes()))
                    }
                }
            }
        }
    }

    /// Deletes a replica from the file system.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica to delete.
    pub async fn delete_replica(&self, namespace_id: &NamespaceId) -> miette::Result<()> {
        let docs_client: &Docs = &self.docs;
        if self.is_home_replica(namespace_id).await {
            return Err(miette::miette!(
                "Cannot delete a home replica (replica ID: {})",
                crate::fs::util::fmt(namespace_id)
            ));
        }
        self.replica_sender.send_replace(());
        Ok(docs_client.drop_doc(*namespace_id).await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotDeleteReplica
        })?)
    }

    /// Lists all replicas in the file system.
    ///
    /// # Returns
    ///
    /// A list of all replicas in the file system, including their ID, capability kind (read or write), and if it's a home replica.
    pub async fn list_replicas(&self) -> miette::Result<Vec<(NamespaceId, CapabilityKind, bool)>> {
        let docs_client = &self.docs;

        let authors: HashSet<AuthorId> = docs_client
            .author_list()
            .await
            .map_err(|e| miette::miette!(e))?
            .filter_map(|x| async move { x.ok() })
            .collect()
            .await;

        let replicas = docs_client.list().await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotListReplicas
        })?;
        pin_mut!(replicas);
        let mut replica_ids: Vec<(NamespaceId, CapabilityKind, bool)> = replicas
            .filter_map(|replica| async {
                replica.ok().map(|(x, y)| {
                    (
                        x,
                        y,
                        (&authors).contains(&AuthorId::from(x.as_bytes()))
                            && matches!(y, CapabilityKind::Write),
                    )
                })
            })
            .collect()
            .await;

        replica_ids.sort_unstable_by_key(|(_namespace_id, capability_kind, is_home_replica)| {
            (
                !is_home_replica,
                !matches!(capability_kind, CapabilityKind::Write),
            )
        });
        Ok(replica_ids)
    }

    /// Retrieves the permissions for a local replica.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica.
    ///
    /// # Returns
    ///
    /// If either the replica can be read from & written to, or if it can only be read from.
    pub async fn get_replica_capability(
        &self,
        namespace_id: &NamespaceId,
    ) -> miette::Result<CapabilityKind> {
        let replicas_vec = self.list_replicas().await?;
        match replicas_vec
            .par_iter()
            .find_any(|replica| replica.0 == *namespace_id)
        {
            Some(replica) => Ok(replica.1),
            None => Err(OkuFuseError::NoReplica(crate::fs::util::fmt(namespace_id)).into()),
        }
    }

    /// Join a swarm to fetch the latest version of a replica and save it to the local machine.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica to fetch.
    ///
    /// * `path` - An optional path of requested files within the replica.
    pub async fn fetch_replica_by_id(
        &self,
        namespace_id: &NamespaceId,
        path: &Option<PathBuf>,
    ) -> anyhow::Result<()> {
        let ticket = self.resolve_namespace_id(namespace_id).await?;
        let docs_client = &self.docs;
        let replica_sender = self.replica_sender.clone();
        match path.clone() {
            Some(path) => {
                let replica = docs_client.import_namespace(ticket.capability).await?;
                let filter = FilterKind::Prefix(path_to_entry_prefix(&path));
                replica
                    .set_download_policy(iroh_docs::store::DownloadPolicy::NothingExcept(vec![
                        filter,
                    ]))
                    .await?;
                replica.start_sync(ticket.nodes).await?;
                let mut events = replica.subscribe().await?;
                let sync_start = std::time::Instant::now();
                while let Some(event) = events.next().await {
                    if matches!(event?, LiveEvent::SyncFinished(_)) {
                        let elapsed = sync_start.elapsed();
                        info!(
                            "Synchronisation took {elapsed:?} for {} … ",
                            crate::fs::util::fmt(namespace_id),
                        );
                        break;
                    }
                }
            }
            None => {
                if let Some(replica) = docs_client.open(*namespace_id).await.unwrap_or(None) {
                    replica
                        .set_download_policy(iroh_docs::store::DownloadPolicy::default())
                        .await?;
                    replica.start_sync(ticket.nodes).await?;
                    let mut events = replica.subscribe().await?;
                    let sync_start = std::time::Instant::now();
                    while let Some(event) = events.next().await {
                        if matches!(event?, LiveEvent::SyncFinished(_)) {
                            let elapsed = sync_start.elapsed();
                            info!(
                                "Synchronisation took {elapsed:?} for {} … ",
                                crate::fs::util::fmt(namespace_id),
                            );
                            break;
                        }
                    }
                } else {
                    let (_replica, mut events) = docs_client.import_and_subscribe(ticket).await?;
                    let sync_start = std::time::Instant::now();
                    while let Some(event) = events.next().await {
                        if matches!(event?, LiveEvent::SyncFinished(_)) {
                            let elapsed = sync_start.elapsed();
                            info!(
                                "Synchronisation took {elapsed:?} for {} … ",
                                crate::fs::util::fmt(namespace_id),
                            );
                            break;
                        }
                    }
                }
            }
        }
        replica_sender.send_replace(());
        Ok(())
    }

    /// Join a swarm to fetch the latest version of a replica and save it to the local machine.
    ///
    /// # Arguments
    ///
    /// * `ticket` - A ticket for the replica to fetch.
    ///
    /// * `path` - An optional path of requested files within the replica.
    ///
    /// # Returns
    ///
    /// A handle to the replica.
    pub async fn fetch_replica_by_ticket(
        &self,
        ticket: &DocTicket,
        path: &Option<PathBuf>,
        filters: &Option<Vec<FilterKind>>,
    ) -> anyhow::Result<()> {
        let namespace_id = ticket.capability.id();
        let docs_client = &self.docs;
        let replica_sender = self.replica_sender.clone();
        match path.clone() {
            Some(path) => {
                let replica = docs_client
                    .import_namespace(ticket.capability.clone())
                    .await?;
                let filters = filters
                    .clone()
                    .unwrap_or(vec![FilterKind::Prefix(path_to_entry_prefix(&path))]);
                replica
                    .set_download_policy(iroh_docs::store::DownloadPolicy::NothingExcept(filters))
                    .await?;
                replica.start_sync(ticket.nodes.clone()).await?;
                let mut events = replica.subscribe().await?;
                let sync_start = std::time::Instant::now();
                while let Some(event) = events.next().await {
                    if matches!(event?, LiveEvent::SyncFinished(_)) {
                        let elapsed = sync_start.elapsed();
                        info!(
                            "Synchronisation took {elapsed:?} for {} … ",
                            crate::fs::util::fmt(namespace_id),
                        );
                        break;
                    }
                }
            }
            None => {
                if let Some(replica) = docs_client.open(namespace_id).await.unwrap_or(None) {
                    replica
                        .set_download_policy(iroh_docs::store::DownloadPolicy::default())
                        .await?;
                    replica.start_sync(ticket.nodes.clone()).await?;
                    let mut events = replica.subscribe().await?;
                    let sync_start = std::time::Instant::now();
                    while let Some(event) = events.next().await {
                        if matches!(event?, LiveEvent::SyncFinished(_)) {
                            let elapsed = sync_start.elapsed();
                            info!(
                                "Synchronisation took {elapsed:?} for {} … ",
                                crate::fs::util::fmt(namespace_id),
                            );
                            break;
                        }
                    }
                } else {
                    let (_replica, mut events) =
                        docs_client.import_and_subscribe(ticket.clone()).await?;
                    let sync_start = std::time::Instant::now();
                    while let Some(event) = events.next().await {
                        if matches!(event?, LiveEvent::SyncFinished(_)) {
                            let elapsed = sync_start.elapsed();
                            info!(
                                "Synchronisation took {elapsed:?} for {} … ",
                                crate::fs::util::fmt(namespace_id),
                            );
                            break;
                        }
                    }
                }
            }
        };
        replica_sender.send_replace(());
        Ok(())
    }

    /// Join a swarm to fetch the latest version of a replica and save it to the local machine.
    ///
    /// If a version of the replica already exists locally, only the last-fetched paths will be fetched.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica to fetch.
    pub async fn sync_replica(&self, namespace_id: &NamespaceId) -> anyhow::Result<()> {
        let ticket = self.resolve_namespace_id(namespace_id).await?;
        let docs_client = &self.docs;
        let replica_sender = self.replica_sender.clone();
        let (_replica, mut events) = docs_client.import_and_subscribe(ticket).await?;
        let sync_start = std::time::Instant::now();
        while let Some(event) = events.next().await {
            if matches!(event?, LiveEvent::SyncFinished(_)) {
                let elapsed = sync_start.elapsed();
                info!(
                    "Synchronisation took {elapsed:?} for {} … ",
                    crate::fs::util::fmt(namespace_id),
                );
                break;
            }
        }
        replica_sender.send_replace(());
        Ok(())
    }

    /// Use the mainline DHT to obtain a ticket for the replica with the given ID.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica to fetch.
    ///
    /// # Returns
    ///
    /// A ticket for the replica with the given ID.
    pub async fn resolve_namespace_id(
        &self,
        namespace_id: &NamespaceId,
    ) -> anyhow::Result<DocTicket> {
        let get_stream = self.dht.get_mutable(namespace_id.as_bytes(), None, None);
        tokio::pin!(get_stream);
        let mut tickets = Vec::new();
        while let Some(mutable_item) = get_stream.next().await {
            let ticket = DocTicket::from_bytes(mutable_item.value())?;
            let ticket_namespace_id = &ticket.capability.id();
            if ticket_namespace_id != namespace_id {
                error!("Ticket is for replica with ID {}, but claims to be for replica with ID {}; ignoring ticket … ", crate::fs::util::fmt(ticket_namespace_id), crate::fs::util::fmt(namespace_id));
                continue;
            }
            if let Err(e) = DATABASE.upsert_announcement(&ReplicaAnnouncement {
                key: mutable_item.key().to_vec(),
                signature: mutable_item.signature().to_vec(),
            }) {
                error!("{e}");
            }
            tickets.push(ticket)
        }
        merge_tickets(&tickets).ok_or(anyhow!(
            "Could not find tickets for {} … ",
            crate::fs::util::fmt(namespace_id)
        ))
    }

    /// Create a sharing ticket for a given replica.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica to share.
    ///
    /// * `share_mode` - Whether the replica should be shared as read-only, or if read & write permissions are to be shared.
    ///
    /// # Returns
    ///
    /// A ticket to retrieve the given replica with the requested permissions.
    pub async fn create_document_ticket(
        &self,
        namespace_id: &NamespaceId,
        share_mode: &ShareMode,
    ) -> miette::Result<DocTicket> {
        if matches!(share_mode, ShareMode::Write)
            && matches!(
                self.get_replica_capability(namespace_id).await?,
                CapabilityKind::Read
            )
        {
            Err(OkuFsError::CannotShareReplicaWritable(*namespace_id).into())
        } else {
            let docs_client = &self.docs;
            let document = docs_client
                .open(*namespace_id)
                .await
                .map_err(|e| {
                    error!("{}", e);
                    OkuFsError::CannotOpenReplica
                })?
                .ok_or(OkuFsError::FsEntryNotFound)?;
            Ok(document
                .share(share_mode.clone(), AddrInfoOptions::RelayAndAddresses)
                .await
                .map_err(|e| {
                    error!("{}", e);
                    OkuDiscoveryError::CannotGenerateSharingTicket
                })?)
        }
    }
}
