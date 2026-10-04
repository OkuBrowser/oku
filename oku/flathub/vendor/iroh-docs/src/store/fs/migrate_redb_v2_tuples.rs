//! Migrate stores written by iroh-docs 0.94..=0.98 (redb 2.x) so they open under redb 4.
//!
//! redb 3.0 changed the on-disk type tag for variable-width tuples. Stores written by
//! redb 2.x carry the old tag for `records-1`, `records-by-key-1`, and `latest-by-author-1`,
//! so redb 4 rejects them with `TableTypeMismatch`. We open the file with redb 3 using
//! `Legacy<_>` wrappers on those tables, copy everything into a fresh redb 3 file with
//! plain types (re-stamping the metadata), and swap files.

use std::path::{Path, PathBuf};

use anyhow::Result;
use redb_v3::{
    MultimapTableHandle, ReadableDatabase, ReadableMultimapTable, ReadableTable, TableHandle,
};
use tempfile::NamedTempFile;
use tracing::info;

type RecordsKey<'a> = (&'a [u8; 32], &'a [u8; 32], &'a [u8]);
type RecordsValue<'a> = (u64, &'a [u8; 64], &'a [u8; 64], u64, &'a [u8; 32]);
type LatestKey<'a> = (&'a [u8; 32], &'a [u8; 32]);
type LatestValue<'a> = (u64, &'a [u8]);
type RecordsByKeyKey<'a> = (&'a [u8; 32], &'a [u8], &'a [u8; 32]);
type NamespacesValue<'a> = (u8, &'a [u8; 32]);
type NamespacePeersValue<'a> = (u64, &'a crate::PeerIdBytes);

pub(super) mod old {
    use redb_v3::{Legacy, TableDefinition};

    use super::{LatestKey, LatestValue, RecordsByKeyKey, RecordsKey, RecordsValue};

    pub const RECORDS_TABLE: TableDefinition<Legacy<RecordsKey>, RecordsValue> =
        TableDefinition::new("records-1");
    pub const LATEST_PER_AUTHOR_TABLE: TableDefinition<LatestKey, Legacy<LatestValue>> =
        TableDefinition::new("latest-by-author-1");
    pub const RECORDS_BY_KEY_TABLE: TableDefinition<Legacy<RecordsByKeyKey>, ()> =
        TableDefinition::new("records-by-key-1");
}

mod new {
    use redb_v3::{MultimapTableDefinition, TableDefinition};

    use super::{
        LatestKey, LatestValue, NamespacePeersValue, NamespacesValue, RecordsByKeyKey, RecordsKey,
        RecordsValue,
    };

    pub const AUTHORS_TABLE: TableDefinition<&[u8; 32], &[u8; 32]> =
        TableDefinition::new("authors-1");
    pub const NAMESPACES_TABLE_V1: TableDefinition<&[u8; 32], &[u8; 32]> =
        TableDefinition::new("namespaces-1");
    pub const NAMESPACES_TABLE: TableDefinition<&[u8; 32], NamespacesValue> =
        TableDefinition::new("namespaces-2");
    pub const RECORDS_TABLE: TableDefinition<RecordsKey, RecordsValue> =
        TableDefinition::new("records-1");
    pub const LATEST_PER_AUTHOR_TABLE: TableDefinition<LatestKey, LatestValue> =
        TableDefinition::new("latest-by-author-1");
    pub const RECORDS_BY_KEY_TABLE: TableDefinition<RecordsByKeyKey, ()> =
        TableDefinition::new("records-by-key-1");
    pub const NAMESPACE_PEERS_TABLE: MultimapTableDefinition<&[u8; 32], NamespacePeersValue> =
        MultimapTableDefinition::new("sync-peers-1");
    pub const DOWNLOAD_POLICY_TABLE: TableDefinition<&[u8; 32], &[u8]> =
        TableDefinition::new("download-policy-1");
}

macro_rules! migrate_table {
    ($existing:expr, $rtx:expr, $wtx:expr, $old:expr, $new:expr) => {{
        if $existing.contains($new.name()) {
            let old_t = $rtx.open_table($old)?;
            let mut new_t = $wtx.open_table($new)?;
            for entry in old_t.iter()? {
                let (k, v) = entry?;
                new_t.insert(k.value(), v.value())?;
            }
        }
    }};
}

macro_rules! migrate_multimap_table {
    ($existing:expr, $rtx:expr, $wtx:expr, $def:expr) => {{
        if $existing.contains($def.name()) {
            let old_t = $rtx.open_multimap_table($def)?;
            let mut new_t = $wtx.open_multimap_table($def)?;
            for entry in old_t.iter()? {
                let (k, values) = entry?;
                let key = k.value();
                for value in values {
                    let value = value?;
                    new_t.insert(key, value.value())?;
                }
            }
        }
    }};
}

pub fn run(source: &Path) -> Result<()> {
    let dir = source
        .parent()
        .ok_or_else(|| anyhow::anyhow!("database path has no parent directory"))?;
    let target = NamedTempFile::with_prefix_in("docs.db.migrate", dir)?.into_temp_path();
    info!(
        "migrating redb 2.x docs store {} -> {}",
        source.display(),
        target.display()
    );

    {
        let old_db = redb_v3::Database::open(source)?;
        let new_db = redb_v3::Database::create(&target)?;
        let rtx = old_db.begin_read()?;
        let wtx = new_db.begin_write()?;

        let existing: std::collections::HashSet<String> =
            rtx.list_tables()?.map(|h| h.name().to_string()).collect();

        migrate_table!(existing, rtx, wtx, new::AUTHORS_TABLE, new::AUTHORS_TABLE);
        migrate_table!(
            existing,
            rtx,
            wtx,
            new::NAMESPACES_TABLE_V1,
            new::NAMESPACES_TABLE_V1
        );
        migrate_table!(
            existing,
            rtx,
            wtx,
            new::NAMESPACES_TABLE,
            new::NAMESPACES_TABLE
        );
        migrate_table!(
            existing,
            rtx,
            wtx,
            new::DOWNLOAD_POLICY_TABLE,
            new::DOWNLOAD_POLICY_TABLE
        );
        migrate_multimap_table!(existing, rtx, wtx, new::NAMESPACE_PEERS_TABLE);
        migrate_table!(existing, rtx, wtx, old::RECORDS_TABLE, new::RECORDS_TABLE);
        migrate_table!(
            existing,
            rtx,
            wtx,
            old::LATEST_PER_AUTHOR_TABLE,
            new::LATEST_PER_AUTHOR_TABLE
        );
        migrate_table!(
            existing,
            rtx,
            wtx,
            old::RECORDS_BY_KEY_TABLE,
            new::RECORDS_BY_KEY_TABLE
        );

        wtx.commit()?;
        drop(rtx);
        drop(old_db);
        drop(new_db);
    }

    let backup: PathBuf = {
        let mut p = source.to_owned().into_os_string();
        p.push(".backup-redb-v2-tuples");
        p.into()
    };
    info!("rename {} -> {}", source.display(), backup.display());
    std::fs::rename(source, &backup)?;
    target.persist_noclobber(source)?;
    Ok(())
}
