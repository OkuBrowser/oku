use super::dht::*;
use super::posts::core::OkuPost;
use super::users::*;
use crate::fs::FS_PATH;
use miette::IntoDiagnostic;
use native_db::*;
use std::{path::PathBuf, sync::LazyLock};

pub(crate) static DATABASE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(FS_PATH).join("OKU_FS_DATABASE"));
/// An Oku node's database.
pub static DATABASE: LazyLock<OkuDatabase> = LazyLock::new(|| OkuDatabase::new().unwrap());
pub(crate) static MODELS: LazyLock<Models> = LazyLock::new(|| {
    let mut models = Models::new();
    models.define::<OkuUser>().unwrap();
    models.define::<OkuPost>().unwrap();
    models.define::<ReplicaAnnouncement>().unwrap();
    models
});

/// The database used by Oku's protocol.
pub struct OkuDatabase {
    pub(crate) database: Database<'static>,
}

impl OkuDatabase {
    /// Open an existing Oku database, or create one if it does not exist.
    ///
    /// # Returns
    ///
    /// An Oku database.
    pub fn new() -> miette::Result<Self> {
        Ok(Self {
            database: native_db::Builder::new()
                .create(&MODELS, &*DATABASE_PATH)
                .into_diagnostic()?,
        })
    }

    /// Perform a database migration.
    pub fn migrate(&self) -> miette::Result<()> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        rw.migrate::<OkuUser>().into_diagnostic()?;
        rw.migrate::<OkuPost>().into_diagnostic()?;
        rw.migrate::<ReplicaAnnouncement>().into_diagnostic()?;
        rw.commit().into_diagnostic()
    }
}
