use super::core::*;
use miette::IntoDiagnostic;
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 3, version = 1)]
#[native_db(
    primary_key(primary_key -> (Vec<u8>, Vec<u8>))
)]
/// A record of a replica announcement on the DHT.
pub struct ReplicaAnnouncement {
    /// The public key of the announcement.
    #[primary_key]
    pub key: Vec<u8>,
    /// The signature of the announcement.
    pub signature: Vec<u8>,
}

impl OkuDatabase {
    /// Insert or update a replica announcement record.
    ///
    /// # Arguments
    ///
    /// * `announcement` - A replica announcement record to upsert.
    ///
    /// # Returns
    ///
    /// The previous record of the announcement, if one existed.
    pub fn upsert_announcement(
        &self,
        announcement: &ReplicaAnnouncement,
    ) -> miette::Result<Option<ReplicaAnnouncement>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let old_value: Option<ReplicaAnnouncement> =
            rw.upsert(announcement.to_owned()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;
        Ok(old_value)
    }

    /// Insert or update multiple replica announcement records.
    ///
    /// # Arguments
    ///
    /// * `announcements` - A list of replica announcement records to upsert.
    ///
    /// # Returns
    ///
    /// A list containing the previous record of each announcement, if one existed.
    pub fn upsert_announcements(
        &self,
        announcements: &[ReplicaAnnouncement],
    ) -> miette::Result<Vec<Option<ReplicaAnnouncement>>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let old_announcements: Vec<_> = announcements
            .iter()
            .cloned()
            .filter_map(|announcement| rw.upsert(announcement).ok())
            .collect();
        rw.commit().into_diagnostic()?;
        Ok(old_announcements)
    }

    /// Delete a replica announcement record.
    ///
    /// # Arguments
    ///
    /// * `announcement` - A replica announcement record to delete.
    ///
    /// # Returns
    ///
    /// The deleted replica announcement record.
    pub fn delete_announcement(
        &self,
        announcement: &ReplicaAnnouncement,
    ) -> miette::Result<ReplicaAnnouncement> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_announcement = rw.remove(announcement.to_owned()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;
        Ok(removed_announcement)
    }

    /// Delete multiple replica announcement records.
    ///
    /// # Arguments
    ///
    /// * `announcements` - A list of replica announcement records to delete.
    ///
    /// # Returns
    ///
    /// A list containing the deleted replica announcement records.
    pub fn delete_announcements(
        &self,
        announcements: &[ReplicaAnnouncement],
    ) -> miette::Result<Vec<ReplicaAnnouncement>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_announcements = announcements
            .iter()
            .filter_map(|announcement| rw.remove(announcement.to_owned()).ok())
            .collect();
        rw.commit().into_diagnostic()?;
        Ok(removed_announcements)
    }

    /// Gets the replica announcements recorded by this node.
    ///
    /// # Returns
    ///
    /// The replica announcements recorded by this node.
    pub fn get_announcements(&self) -> miette::Result<Vec<ReplicaAnnouncement>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        r.scan()
            .primary()
            .into_diagnostic()?
            .all()
            .into_diagnostic()?
            .collect::<Result<Vec<_>, _>>()
            .into_diagnostic()
    }

    /// Gets a replica announcement record by its public key.
    ///
    /// # Arguments
    ///
    /// * `key` - The public key of the DHT announcement.
    ///
    /// # Returns
    ///
    /// A replica announcement record.
    pub fn get_announcement(&self, key: &Vec<u8>) -> miette::Result<Option<ReplicaAnnouncement>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        r.get().primary(key.to_owned()).into_diagnostic()
    }
}
