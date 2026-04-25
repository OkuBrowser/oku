use crate::database::core::DATABASE;
use crate::database::dht::ReplicaAnnouncement;
use crate::{error::OkuDiscoveryError, fs::OkuFs};
use iroh_blobs::HashAndFormat;
use iroh_docs::api::protocol::ShareMode;
use iroh_docs::NamespaceId;
use iroh_tickets::Ticket;
use log::{debug, error, info};
use miette::IntoDiagnostic;
use std::{path::PathBuf, time::Duration};
use tokio::task::JoinSet;

/// The delay between republishing content to the Mainline DHT.
pub const DEFAULT_REPUBLISH_DELAY: Duration = Duration::from_secs(60 * 60);

/// The initial delay before publishing content to the Mainline DHT.
pub const DEFAULT_INITIAL_PUBLISH_DELAY: Duration = Duration::from_millis(500);

impl OkuFs {
    /// Announces a replica to the Mainline DHT.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica to announce.
    pub async fn announce_replica(
        &self,
        namespace_id: &NamespaceId,
    ) -> miette::Result<NamespaceId> {
        let public_key_bytes = namespace_id
            .into_public_key()
            .map_err(|e| miette::miette!("{}", e))?
            .as_bytes()
            .to_vec();
        let existing_announcement = DATABASE.get_announcement(&public_key_bytes).ok().flatten();

        let ticket = self
            .create_document_ticket(namespace_id, &ShareMode::Read)
            .await?
            .to_bytes();
        let newest_timestamp = self
            .get_newest_timestamp_in_folder(namespace_id, &PathBuf::from("/"))
            .await? as i64;

        // Ideally, we can repeat an announcement we've already heard for this replica
        let mutable_item = match existing_announcement {
            None => {
                debug!(
                    "Prior announcement not found in database for replica {} … ",
                    crate::fs::util::fmt(namespace_id)
                );
                // Even if we don't have someone else's announcement saved, we can create our own if we have write access to the replica
                let replica_private_key = mainline::SigningKey::from_bytes(
                    &self
                        .create_document_ticket(namespace_id, &ShareMode::Write)
                        .await?
                        .capability
                        .secret_key()
                        .into_diagnostic()?
                        .to_bytes(),
                );
                mainline::MutableItem::new(replica_private_key, &ticket, newest_timestamp, None)
            }
            Some(announcement) => mainline::MutableItem::new_signed_unchecked(
                announcement.key.try_into().map_err(|_e| {
                    miette::miette!("Replica announcement key does not fit into 32 bytes … ")
                })?,
                announcement.signature.try_into().map_err(|_e| {
                    miette::miette!("Replica announcement signature does not fit into 64 bytes … ")
                })?,
                &ticket,
                newest_timestamp,
                None,
            ),
        };
        let replica_announcement = ReplicaAnnouncement {
            key: mutable_item.key().to_vec(),
            signature: mutable_item.signature().to_vec(),
        };
        match self.dht.put_mutable(mutable_item, None).await {
            Ok(_) => {
                info!(
                    "Announced replica {} … ",
                    crate::fs::util::fmt(namespace_id)
                );
                if let Err(e) = DATABASE.upsert_announcement(&replica_announcement) {
                    error!("{e}");
                }
            }
            Err(e) => error!(
                "{}",
                OkuDiscoveryError::ProblemAnnouncingContent(
                    crate::fs::util::fmt(namespace_id),
                    e.to_string()
                )
            ),
        }
        Ok(*namespace_id)
    }

    /// Announces read-only tickets for all known replicas to the Mainline DHT.
    pub async fn announce_replicas(&self) -> miette::Result<()> {
        let mut future_set = JoinSet::new();

        // Prepare to announce all replicas
        let replicas = self.list_replicas().await?;
        for (replica, _capability_kind, _is_home_replica) in replicas {
            let self_clone = self.clone();
            future_set.spawn(async move { self_clone.announce_replica(&replica).await });
        }
        info!("Pending announcements: {} … ", future_set.len());
        // Execute announcements in parallel
        while let Some(res) = future_set.join_next().await {
            match res {
                Ok(result) => match result {
                    Ok(_) => (),
                    Err(e) => error!("{}", e),
                },
                Err(e) => error!("{}", e),
            }
        }

        Ok(())
    }
}

/// From: <https://github.com/n0-computer/iroh-experiments/blob/4e052c6b34720e26683083270706926a84e49411/content-discovery/iroh-mainline-content-discovery/src/client.rs#L53>
///
/// The mapping from an iroh [HashAndFormat] to a bittorrent infohash, aka [mainline::Id].
///
/// Since an infohash is just 20 bytes, this can not be a bidirectional mapping.
pub fn to_infohash(haf: &HashAndFormat) -> mainline::Id {
    let mut data = [0u8; 20];
    data.copy_from_slice(&haf.hash.as_bytes()[..20]);
    mainline::Id::from_bytes(data).unwrap()
}
