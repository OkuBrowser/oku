//! Table definitions and accessors for the redb database.
use redb::{ReadableTable, TableDefinition, TableError};

use super::EntryState;
use crate::{api::Tag, store::fs::delete_set::FileTransaction, Hash, HashAndFormat};

pub(super) const BLOBS_TABLE: TableDefinition<Hash, EntryState> = TableDefinition::new("blobs-0");

pub(super) const TAGS_TABLE: TableDefinition<Tag, HashAndFormat> = TableDefinition::new("tags-0");

pub(super) const INLINE_DATA_TABLE: TableDefinition<Hash, &[u8]> =
    TableDefinition::new("inline-data-0");

pub(super) const INLINE_OUTBOARD_TABLE: TableDefinition<Hash, &[u8]> =
    TableDefinition::new("inline-outboard-0");

/// A trait similar to [`redb::ReadableTable`] but for all tables that make up
/// the blob store. This can be used in places where either a readonly or
/// mutable table is needed.
pub(super) trait ReadableTables {
    fn blobs(&self) -> &impl ReadableTable<Hash, EntryState>;
    fn tags(&self) -> &impl ReadableTable<Tag, HashAndFormat>;
    fn inline_data(&self) -> &impl ReadableTable<Hash, &'static [u8]>;
    fn inline_outboard(&self) -> &impl ReadableTable<Hash, &'static [u8]>;
}

/// A struct similar to [`redb::Table`] but for all tables that make up the
/// blob store.
pub(super) struct Tables<'a> {
    pub blobs: redb::Table<'a, Hash, EntryState>,
    pub tags: redb::Table<'a, Tag, HashAndFormat>,
    pub inline_data: redb::Table<'a, Hash, &'static [u8]>,
    pub inline_outboard: redb::Table<'a, Hash, &'static [u8]>,
    pub ftx: &'a FileTransaction<'a>,
}

impl<'txn> Tables<'txn> {
    pub fn new(
        tx: &'txn redb::WriteTransaction,
        ds: &'txn FileTransaction<'txn>,
    ) -> std::result::Result<Self, TableError> {
        Ok(Self {
            blobs: tx.open_table(BLOBS_TABLE)?,
            tags: tx.open_table(TAGS_TABLE)?,
            inline_data: tx.open_table(INLINE_DATA_TABLE)?,
            inline_outboard: tx.open_table(INLINE_OUTBOARD_TABLE)?,
            ftx: ds,
        })
    }
}

impl ReadableTables for Tables<'_> {
    fn blobs(&self) -> &impl ReadableTable<Hash, EntryState> {
        &self.blobs
    }
    fn tags(&self) -> &impl ReadableTable<Tag, HashAndFormat> {
        &self.tags
    }
    fn inline_data(&self) -> &impl ReadableTable<Hash, &'static [u8]> {
        &self.inline_data
    }
    fn inline_outboard(&self) -> &impl ReadableTable<Hash, &'static [u8]> {
        &self.inline_outboard
    }
}

/// A struct similar to [`redb::ReadOnlyTable`] but for all tables that make up
/// the blob store.
#[derive(Debug)]
pub(crate) struct ReadOnlyTables {
    pub blobs: redb::ReadOnlyTable<Hash, EntryState>,
    pub tags: redb::ReadOnlyTable<Tag, HashAndFormat>,
    pub inline_data: redb::ReadOnlyTable<Hash, &'static [u8]>,
    pub inline_outboard: redb::ReadOnlyTable<Hash, &'static [u8]>,
}

impl ReadOnlyTables {
    pub fn new(tx: &redb::ReadTransaction) -> std::result::Result<Self, TableError> {
        Ok(Self {
            blobs: tx.open_table(BLOBS_TABLE)?,
            tags: tx.open_table(TAGS_TABLE)?,
            inline_data: tx.open_table(INLINE_DATA_TABLE)?,
            inline_outboard: tx.open_table(INLINE_OUTBOARD_TABLE)?,
        })
    }
}

impl ReadableTables for ReadOnlyTables {
    fn blobs(&self) -> &impl ReadableTable<Hash, EntryState> {
        &self.blobs
    }
    fn tags(&self) -> &impl ReadableTable<Tag, HashAndFormat> {
        &self.tags
    }
    fn inline_data(&self) -> &impl ReadableTable<Hash, &'static [u8]> {
        &self.inline_data
    }
    fn inline_outboard(&self) -> &impl ReadableTable<Hash, &'static [u8]> {
        &self.inline_outboard
    }
}
