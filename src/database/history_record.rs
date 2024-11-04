use super::{BrowserDatabase, DATABASE};
use miette::IntoDiagnostic;
use native_db::*;
use native_model::{native_model, Model};
use oku_fs::fs::FS_PATH;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Reverse,
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, LazyLock},
};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::QueryParser,
    schema::{Field, Schema, Value, FAST, STORED, TEXT},
    Directory, Index, IndexReader, IndexWriter, TantivyDocument, Term,
};
use tokio::sync::Mutex;
use uuid::Uuid;

pub(crate) static HISTORY_RECORD_INDEX_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(FS_PATH).join("HISTORY_RECORD_INDEX"));
pub(crate) static HISTORY_RECORD_SCHEMA: LazyLock<(Schema, HashMap<&str, Field>)> =
    LazyLock::new(|| {
        let mut schema_builder = Schema::builder();
        let fields = HashMap::from([
            ("id", schema_builder.add_text_field("id", STORED)),
            (
                "original_uri",
                schema_builder.add_text_field("original_uri", TEXT | STORED),
            ),
            ("uri", schema_builder.add_text_field("uri", TEXT | STORED)),
            (
                "title",
                schema_builder.add_text_field("title", TEXT | STORED),
            ),
            (
                "timestamp",
                schema_builder.add_date_field("timestamp", FAST),
            ),
        ]);
        let schema = schema_builder.build();
        (schema, fields)
    });
pub(crate) static HISTORY_RECORD_INDEX: LazyLock<Index> = LazyLock::new(|| {
    let _ = std::fs::create_dir_all(&*HISTORY_RECORD_INDEX_PATH);
    let mmap_directory: Box<dyn Directory> =
        Box::new(MmapDirectory::open(&*HISTORY_RECORD_INDEX_PATH).unwrap());
    Index::open_or_create(mmap_directory, HISTORY_RECORD_SCHEMA.0.clone()).unwrap()
});
pub(crate) static HISTORY_RECORD_INDEX_READER: LazyLock<IndexReader> =
    LazyLock::new(|| HISTORY_RECORD_INDEX.reader().unwrap());
pub(crate) static HISTORY_RECORD_INDEX_WRITER: LazyLock<Arc<Mutex<IndexWriter>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HISTORY_RECORD_INDEX.writer(50_000_000).unwrap())));

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[native_model(id = 1, version = 1)]
#[native_db(
    primary_key(id_string -> String)
)]
pub struct HistoryRecord {
    pub id: Uuid,
    pub original_uri: String,
    pub uri: String,
    pub title: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HistoryRecord {
    fn id_string(&self) -> String {
        self.id.to_string()
    }

    fn index_term(&self) -> Term {
        Term::from_field_text(HISTORY_RECORD_SCHEMA.1["id"], &self.id.to_string())
    }
}

impl From<HistoryRecord> for TantivyDocument {
    fn from(value: HistoryRecord) -> Self {
        let mut doc = TantivyDocument::default();
        doc.add_text(HISTORY_RECORD_SCHEMA.1["id"], value.id);
        doc.add_text(HISTORY_RECORD_SCHEMA.1["original_uri"], value.original_uri);
        doc.add_text(HISTORY_RECORD_SCHEMA.1["uri"], value.uri);
        if let Some(title) = value.title {
            doc.add_text(HISTORY_RECORD_SCHEMA.1["title"], title);
        }
        doc.add_date(
            HISTORY_RECORD_SCHEMA.1["timestamp"],
            tantivy::DateTime::from_timestamp_micros(value.timestamp.timestamp_micros()),
        );
        doc
    }
}

impl TryFrom<TantivyDocument> for HistoryRecord {
    type Error = miette::Report;

    fn try_from(value: TantivyDocument) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(
            value
                .get_first(HISTORY_RECORD_SCHEMA.1["id"])
                .map(|x| x.as_str())
                .flatten()
                .ok_or(miette::miette!("No ID for document in index … "))?,
        )
        .into_diagnostic()?;
        DATABASE
            .get_history_record(id.clone())
            .ok()
            .flatten()
            .ok_or(miette::miette!(
                "No history record with original URI {} found … ",
                id
            ))
    }
}

impl BrowserDatabase {
    pub fn search_history_records(
        query_string: String,
        result_limit: Option<usize>,
    ) -> miette::Result<Vec<HistoryRecord>> {
        let searcher = HISTORY_RECORD_INDEX_READER.searcher();
        let query_parser = QueryParser::for_index(
            &*HISTORY_RECORD_INDEX,
            vec![
                HISTORY_RECORD_SCHEMA.1["original_uri"],
                HISTORY_RECORD_SCHEMA.1["uri"],
                HISTORY_RECORD_SCHEMA.1["title"],
            ],
        );
        let query = query_parser.parse_query(&query_string).into_diagnostic()?;
        let limit = result_limit.unwrap_or(10);
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .into_diagnostic()?;
        Ok(top_docs
            .par_iter()
            .filter_map(|x| searcher.doc(x.1).ok())
            .collect::<Vec<TantivyDocument>>()
            .into_par_iter()
            .filter_map(|x| TryInto::try_into(x).ok())
            .collect())
    }

    pub fn rebuild_history_record_index(&self) -> miette::Result<()> {
        let mut index_writer = HISTORY_RECORD_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        index_writer.delete_all_documents().into_diagnostic()?;
        self.get_history_records()?
            .into_par_iter()
            .filter_map(|x| index_writer.add_document(x.into()).ok())
            .collect::<Vec<_>>();
        index_writer.commit().into_diagnostic()?;
        Ok(())
    }

    pub fn upsert_history_record(
        &self,
        history_record: HistoryRecord,
    ) -> miette::Result<Option<HistoryRecord>> {
        let rw: transaction::RwTransaction<'_> =
            self.database.rw_transaction().into_diagnostic()?;
        let old_value: Option<HistoryRecord> =
            rw.upsert(history_record.clone()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;
        self.history_sender.send_replace(());

        let mut index_writer = HISTORY_RECORD_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        if let Some(old_history_record) = old_value.clone() {
            index_writer.delete_term(old_history_record.index_term());
        }
        index_writer
            .add_document(history_record.into())
            .into_diagnostic()?;
        index_writer.commit().into_diagnostic()?;

        Ok(old_value)
    }

    pub fn upsert_history_records(
        &self,
        history_records: Vec<HistoryRecord>,
    ) -> miette::Result<Vec<Option<HistoryRecord>>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let old_history_records: Vec<_> = history_records
            .clone()
            .into_iter()
            .filter_map(|history_record| rw.upsert(history_record).ok())
            .collect();
        rw.commit().into_diagnostic()?;
        self.history_sender.send_replace(());

        let mut index_writer = HISTORY_RECORD_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        old_history_records
            .par_iter()
            .for_each(|old_history_record| {
                if let Some(old_history_record) = old_history_record {
                    index_writer.delete_term(old_history_record.index_term());
                }
            });
        history_records.par_iter().for_each(|history_record| {
            let _ = index_writer.add_document(history_record.clone().into());
        });
        index_writer.commit().into_diagnostic()?;

        Ok(old_history_records)
    }

    pub fn delete_history_record(
        &self,
        history_record: HistoryRecord,
    ) -> miette::Result<HistoryRecord> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_history_record = rw.remove(history_record).into_diagnostic()?;
        rw.commit().into_diagnostic()?;
        self.history_sender.send_replace(());

        let mut index_writer = HISTORY_RECORD_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        index_writer.delete_term(removed_history_record.index_term());
        index_writer.commit().into_diagnostic()?;

        Ok(removed_history_record)
    }

    pub fn delete_history_records(
        &self,
        history_records: Vec<HistoryRecord>,
    ) -> miette::Result<Vec<HistoryRecord>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_history_records: Vec<_> = history_records
            .into_iter()
            .filter_map(|history_record| rw.remove(history_record).ok())
            .collect();
        rw.commit().into_diagnostic()?;
        self.history_sender.send_replace(());

        let mut index_writer = HISTORY_RECORD_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        removed_history_records
            .par_iter()
            .for_each(|removed_history_record| {
                index_writer.delete_term(removed_history_record.index_term());
            });
        index_writer.commit().into_diagnostic()?;

        Ok(removed_history_records)
    }

    pub fn get_history_records(&self) -> miette::Result<Vec<HistoryRecord>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        let mut history_records = r
            .scan()
            .primary()
            .into_diagnostic()?
            .all()
            .into_diagnostic()?
            .collect::<Result<Vec<_>, _>>()
            .into_diagnostic()?;
        history_records.sort_unstable_by_key(|x: &HistoryRecord| Reverse(x.timestamp));
        Ok(history_records)
    }

    pub fn get_history_record(&self, id: Uuid) -> miette::Result<Option<HistoryRecord>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        Ok(r.get().primary(id.to_string()).into_diagnostic()?)
    }
}
