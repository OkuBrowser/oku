use super::{BrowserDatabase, DATABASE};
use miette::IntoDiagnostic;
use native_db::*;
use native_model::{native_model, Model};
use oku_fs::{database::posts::OkuNote, fs::FS_PATH};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, LazyLock},
};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::QueryParser,
    schema::{Field, Schema, Value, STORED, TEXT},
    Directory, Index, IndexReader, IndexWriter, TantivyDocument, Term,
};
use tokio::sync::Mutex;

pub(crate) static BOOKMARK_INDEX_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(FS_PATH).join("BOOKMARK_INDEX"));
pub(crate) static BOOKMARK_SCHEMA: LazyLock<(Schema, HashMap<&str, Field>)> = LazyLock::new(|| {
    let mut schema_builder = Schema::builder();
    let fields = HashMap::from([
        ("url", schema_builder.add_text_field("url", TEXT | STORED)),
        (
            "title",
            schema_builder.add_text_field("title", TEXT | STORED),
        ),
        ("body", schema_builder.add_text_field("body", TEXT | STORED)),
        ("tag", schema_builder.add_text_field("tag", TEXT | STORED)),
    ]);
    let schema = schema_builder.build();
    (schema, fields)
});
pub(crate) static BOOKMARK_INDEX: LazyLock<Index> = LazyLock::new(|| {
    let _ = std::fs::create_dir_all(&*BOOKMARK_INDEX_PATH);
    let mmap_directory: Box<dyn Directory> =
        Box::new(MmapDirectory::open(&*BOOKMARK_INDEX_PATH).unwrap());
    Index::open_or_create(mmap_directory, BOOKMARK_SCHEMA.0.clone()).unwrap()
});
pub(crate) static BOOKMARK_INDEX_READER: LazyLock<IndexReader> =
    LazyLock::new(|| BOOKMARK_INDEX.reader().unwrap());
pub(crate) static BOOKMARK_INDEX_WRITER: LazyLock<Arc<Mutex<IndexWriter>>> =
    LazyLock::new(|| Arc::new(Mutex::new(BOOKMARK_INDEX.writer(50_000_000).unwrap())));

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[native_model(id = 2, version = 1)]
#[native_db]
pub struct Bookmark {
    #[primary_key]
    pub url: String,
    pub title: String,
    pub body: String,
    pub tags: HashSet<String>,
}

impl Bookmark {
    fn index_term(&self) -> Term {
        Term::from_field_text(BOOKMARK_SCHEMA.1["url"], &self.url)
    }
}

impl TryFrom<Bookmark> for OkuNote {
    type Error = miette::Report;

    fn try_from(value: Bookmark) -> Result<Self, Self::Error> {
        Ok(OkuNote {
            url: url::Url::parse(&value.url).into_diagnostic()?,
            title: value.title,
            body: value.body,
            tags: value.tags,
        })
    }
}

impl From<OkuNote> for Bookmark {
    fn from(value: OkuNote) -> Self {
        Self {
            url: value.url.to_string(),
            title: value.title,
            body: value.body,
            tags: value.tags,
        }
    }
}

impl From<Bookmark> for TantivyDocument {
    fn from(value: Bookmark) -> Self {
        let mut doc = TantivyDocument::default();
        doc.add_text(BOOKMARK_SCHEMA.1["url"], value.url);
        doc.add_text(BOOKMARK_SCHEMA.1["title"], value.title);
        doc.add_text(BOOKMARK_SCHEMA.1["body"], value.body);
        for tag in value.tags {
            doc.add_text(BOOKMARK_SCHEMA.1["tag"], tag);
        }
        doc
    }
}

impl TryFrom<TantivyDocument> for Bookmark {
    type Error = miette::Report;

    fn try_from(value: TantivyDocument) -> Result<Self, Self::Error> {
        let url = value
            .get_first(BOOKMARK_SCHEMA.1["url"])
            .ok_or(miette::miette!("No URL for document in index … "))?
            .as_str()
            .ok_or(miette::miette!("No URL for document in index … "))?
            .to_string();
        DATABASE
            .get_bookmark(url.clone())
            .ok()
            .flatten()
            .ok_or(miette::miette!("No bookmark with URL {} found … ", url))
    }
}

impl BrowserDatabase {
    pub fn search_bookmarks(
        query_string: String,
        result_limit: Option<usize>,
    ) -> miette::Result<Vec<Bookmark>> {
        let searcher = BOOKMARK_INDEX_READER.searcher();
        let query_parser = QueryParser::for_index(
            &BOOKMARK_INDEX,
            BOOKMARK_SCHEMA.1.clone().into_values().collect(),
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

    pub fn rebuild_bookmark_index(&self) -> miette::Result<()> {
        let mut index_writer = BOOKMARK_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        index_writer.delete_all_documents().into_diagnostic()?;
        self.get_bookmarks()?
            .into_par_iter()
            .filter_map(|x| index_writer.add_document(x.into()).ok())
            .collect::<Vec<_>>();
        index_writer.commit().into_diagnostic()?;
        Ok(())
    }

    pub fn upsert_bookmark(&self, bookmark: Bookmark) -> miette::Result<Option<Bookmark>> {
        let rw: transaction::RwTransaction<'_> =
            self.database.rw_transaction().into_diagnostic()?;
        let old_value: Option<Bookmark> = rw.upsert(bookmark.clone()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;
        self.bookmark_sender.send_replace(());

        let mut index_writer = BOOKMARK_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        if let Some(old_bookmark) = old_value.clone() {
            index_writer.delete_term(old_bookmark.index_term());
        }
        index_writer
            .add_document(bookmark.into())
            .into_diagnostic()?;
        index_writer.commit().into_diagnostic()?;

        Ok(old_value)
    }

    pub fn upsert_bookmarks(
        &self,
        bookmarks: Vec<Bookmark>,
    ) -> miette::Result<Vec<Option<Bookmark>>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let old_bookmarks: Vec<_> = bookmarks
            .clone()
            .into_iter()
            .filter_map(|bookmark| rw.upsert(bookmark).ok())
            .collect();
        rw.commit().into_diagnostic()?;
        self.bookmark_sender.send_replace(());

        let mut index_writer = BOOKMARK_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        old_bookmarks.par_iter().for_each(|old_bookmark| {
            if let Some(old_bookmark) = old_bookmark {
                index_writer.delete_term(old_bookmark.index_term());
            }
        });
        bookmarks.par_iter().for_each(|bookmark| {
            let _ = index_writer.add_document(bookmark.clone().into());
        });
        index_writer.commit().into_diagnostic()?;

        Ok(old_bookmarks)
    }

    pub fn delete_bookmark(&self, bookmark: Bookmark) -> miette::Result<Bookmark> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_bookmark = rw.remove(bookmark).into_diagnostic()?;
        rw.commit().into_diagnostic()?;
        self.bookmark_sender.send_replace(());

        let mut index_writer = BOOKMARK_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        index_writer.delete_term(removed_bookmark.index_term());
        index_writer.commit().into_diagnostic()?;

        Ok(removed_bookmark)
    }

    pub fn delete_bookmarks(&self, bookmarks: Vec<Bookmark>) -> miette::Result<Vec<Bookmark>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_bookmarks: Vec<_> = bookmarks
            .into_iter()
            .filter_map(|bookmark| rw.remove(bookmark).ok())
            .collect();
        rw.commit().into_diagnostic()?;
        self.bookmark_sender.send_replace(());

        let mut index_writer = BOOKMARK_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        removed_bookmarks.par_iter().for_each(|removed_bookmark| {
            index_writer.delete_term(removed_bookmark.index_term());
        });
        index_writer.commit().into_diagnostic()?;

        Ok(removed_bookmarks)
    }

    pub fn get_bookmarks(&self) -> miette::Result<Vec<Bookmark>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        r.scan()
            .primary()
            .into_diagnostic()?
            .all()
            .into_diagnostic()?
            .collect::<Result<Vec<_>, _>>()
            .into_diagnostic()
    }

    pub fn get_bookmark(&self, url: String) -> miette::Result<Option<Bookmark>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        r.get().primary(url).into_diagnostic()
    }
}
