use super::super::core::*;
use super::super::users::*;
use crate::fs::FS_PATH;
use iroh_docs::sync::Entry;
use iroh_docs::AuthorId;
use log::error;
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, LazyLock},
    time::SystemTime,
};
use tantivy::{
    directory::MmapDirectory,
    schema::{Field, Schema, Value, FAST, STORED, TEXT},
    Directory, Index, IndexReader, IndexWriter, TantivyDocument, Term,
};
use tokio::sync::Mutex;
use url::Url;

pub(crate) static POST_INDEX_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(FS_PATH).join("POST_INDEX"));
pub(crate) static POST_SCHEMA: LazyLock<(Schema, HashMap<&str, Field>)> = LazyLock::new(|| {
    let mut schema_builder = Schema::builder();
    let fields = HashMap::from([
        ("id", schema_builder.add_bytes_field("id", STORED)),
        (
            "author_id",
            schema_builder.add_text_field("author_id", TEXT | STORED),
        ),
        ("path", schema_builder.add_text_field("path", TEXT | STORED)),
        ("url", schema_builder.add_text_field("url", TEXT | STORED)),
        (
            "title",
            schema_builder.add_text_field("title", TEXT | STORED),
        ),
        ("body", schema_builder.add_text_field("body", TEXT | STORED)),
        ("tag", schema_builder.add_text_field("tag", TEXT | STORED)),
        (
            "timestamp",
            schema_builder.add_date_field("timestamp", FAST),
        ),
    ]);
    let schema = schema_builder.build();
    (schema, fields)
});
pub(crate) static POST_INDEX: LazyLock<Index> = LazyLock::new(|| {
    if let Err(e) = std::fs::create_dir_all(&*POST_INDEX_PATH) {
        error!("{e}");
    }
    let mmap_directory: Box<dyn Directory> =
        Box::new(MmapDirectory::open(&*POST_INDEX_PATH).unwrap());
    Index::open_or_create(mmap_directory, POST_SCHEMA.0.clone()).unwrap()
});
pub(crate) static POST_INDEX_READER: LazyLock<IndexReader> =
    LazyLock::new(|| POST_INDEX.reader().unwrap());
pub(crate) static POST_INDEX_WRITER: LazyLock<Arc<Mutex<IndexWriter>>> =
    LazyLock::new(|| Arc::new(Mutex::new(POST_INDEX.writer(50_000_000).unwrap())));

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 2, version = 2)]
#[native_db(
    primary_key(primary_key -> (Vec<u8>, Vec<u8>))
)]
/// An OkuNet post.
pub struct OkuPost {
    /// A record of a version of the post file.
    pub entry: Entry,
    /// The content of the post on OkuNet.
    pub note: OkuNote,
}

impl PartialEq for OkuPost {
    fn eq(&self, other: &Self) -> bool {
        self.primary_key() == other.primary_key()
    }
}
impl Eq for OkuPost {}
impl Hash for OkuPost {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.primary_key().hash(state);
    }
}

impl From<OkuPost> for TantivyDocument {
    fn from(value: OkuPost) -> Self {
        let post_key: [Vec<u8>; 2] = value.primary_key().into();
        let post_key_bytes = post_key.concat();

        let mut doc = TantivyDocument::default();
        doc.add_bytes(POST_SCHEMA.1["id"], &post_key_bytes);
        doc.add_text(
            POST_SCHEMA.1["author_id"],
            crate::fs::util::fmt(value.entry.author()),
        );
        doc.add_text(
            POST_SCHEMA.1["path"],
            String::from_utf8_lossy(value.entry.key()),
        );
        doc.add_text(POST_SCHEMA.1["url"], value.note.url.to_string());
        doc.add_text(POST_SCHEMA.1["title"], value.note.title);
        doc.add_text(POST_SCHEMA.1["body"], value.note.body);
        for tag in value.note.tags {
            doc.add_text(POST_SCHEMA.1["tag"], tag);
        }
        doc.add_date(
            POST_SCHEMA.1["timestamp"],
            tantivy::DateTime::from_timestamp_micros(value.entry.timestamp() as i64),
        );
        doc
    }
}

impl TryFrom<TantivyDocument> for OkuPost {
    type Error = anyhow::Error;

    fn try_from(value: TantivyDocument) -> Result<Self, Self::Error> {
        let author_id = AuthorId::from_str(
            value
                .get_first(POST_SCHEMA.1["author_id"])
                .ok_or(anyhow::anyhow!("No author ID for document in index … "))?
                .as_str()
                .ok_or(anyhow::anyhow!("No author ID for document in index … "))?,
        )?;
        let path = value
            .get_first(POST_SCHEMA.1["path"])
            .ok_or(anyhow::anyhow!("No path for document in index … "))?
            .as_str()
            .ok_or(anyhow::anyhow!("No path for document in index … "))?
            .to_string();
        DATABASE
            .get_post(&author_id, &path.clone().into())
            .ok()
            .flatten()
            .ok_or(anyhow::anyhow!(
                "No post with author {} and path {} found … ",
                author_id,
                path
            ))
    }
}

impl OkuPost {
    pub(crate) fn primary_key(&self) -> (Vec<u8>, Vec<u8>) {
        (
            self.entry.author().as_bytes().to_vec(),
            self.entry.key().to_vec(),
        )
    }

    pub(crate) fn index_term(&self) -> Term {
        let post_key: [Vec<u8>; 2] = self.primary_key().into();
        let post_key_bytes = post_key.concat();
        Term::from_field_bytes(POST_SCHEMA.1["id"], &post_key_bytes)
    }

    /// Obtain the author of this post from the OkuNet database.
    pub fn user(&self) -> OkuUser {
        match DATABASE.get_user(&self.entry.author()).ok().flatten() {
            Some(user) => user,
            None => OkuUser {
                author_id: self.entry.author(),
                last_fetched: SystemTime::now(),
                posts: vec![self.entry.clone()],
                identity: None,
            },
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
/// A note left by an Oku user regarding some URL-addressed content.
pub struct OkuNote {
    /// The URL the note is regarding.
    pub url: Url,
    /// The title of the note.
    pub title: String,
    /// The body of the note.
    pub body: String,
    /// A list of tags associated with the note.
    pub tags: HashSet<String>,
}

impl OkuNote {
    /// Generate a post path for the note.
    pub fn post_path(&self) -> String {
        Self::post_path_from_url(&self.url.to_string())
    }

    /// Generate a post path using a URL.
    pub fn post_path_from_url(url: &String) -> String {
        format!("/posts/{}.toml", bs58::encode(url.as_bytes()).into_string())
    }

    /// Generate an embedding path for the note.
    pub fn embedding_path(&self) -> String {
        Self::embedding_path_from_url(&self.url.to_string())
    }

    /// Generate an archive path for the note.
    pub fn archive_path(&self) -> String {
        Self::archive_path_from_url(&self.url.to_string())
    }

    /// Generate an embedding path using a URL.
    pub fn embedding_path_from_url(url: &String) -> String {
        format!(
            "/embeddings/{}.json",
            bs58::encode(url.as_bytes()).into_string()
        )
    }

    /// Generate an archive path using a URL.
    pub fn archive_path_from_url(url: &String) -> String {
        format!("/archives/{}", bs58::encode(url.as_bytes()).into_string())
    }
}
