use super::*;
use crate::{suggestion_item::SuggestionItem, DATA_DIR};
use miette::IntoDiagnostic;
use native_db::*;
use oku_fs::database::OkuDatabase;
use std::{path::PathBuf, sync::LazyLock};
use webkit2gtk::FaviconDatabase;

pub(crate) static DATABASE_PATH: LazyLock<PathBuf> = LazyLock::new(|| DATA_DIR.join("database"));
pub(crate) static DATABASE: LazyLock<BrowserDatabase> =
    LazyLock::new(|| BrowserDatabase::new().unwrap());
pub(crate) static MODELS: LazyLock<Models> = LazyLock::new(|| {
    let mut models = Models::new();
    models.define::<HistoryRecord>().unwrap();
    models.define::<Bookmark>().unwrap();
    models
});

pub struct BrowserDatabase {
    pub(super) database: Database<'static>,
    pub history_sender: tokio::sync::watch::Sender<()>,
}

impl BrowserDatabase {
    pub fn new() -> miette::Result<Self> {
        let database = Self {
            database: native_db::Builder::new()
                .create(&MODELS, &*DATABASE_PATH)
                .into_diagnostic()?,
            history_sender: tokio::sync::watch::channel(()).0,
        };
        if database.get_history_records()?.len() as u64
            != HISTORY_RECORD_INDEX_READER.searcher().num_docs()
        {
            database.rebuild_history_record_index()?;
        }
        Ok(database)
    }

    pub fn search(
        &self,
        query_string: String,
        favicon_database: &FaviconDatabase,
    ) -> miette::Result<Vec<SuggestionItem>> {
        let history_records = Self::search_history_records(query_string.clone(), None)?;
        let bookmarks = Self::search_bookmarks(query_string.clone(), None)?;
        let okunet_posts = OkuDatabase::search_posts(query_string.clone(), None)?;

        let history_record_suggestions: Vec<_> = history_records
            .into_iter()
            .map(|x| {
                SuggestionItem::new(x.title.unwrap_or(String::new()), x.uri, &favicon_database)
            })
            .collect();
        let bookmark_suggestions = bookmarks
            .into_iter()
            .map(|x| SuggestionItem::new(x.title, x.url, &favicon_database))
            .collect();
        let okunet_post_suggestions = okunet_posts
            .into_iter()
            .map(|x| SuggestionItem::new(x.note.title, x.note.url.to_string(), &favicon_database))
            .collect();

        Ok(vec![
            history_record_suggestions,
            bookmark_suggestions,
            okunet_post_suggestions,
        ]
        .concat())
    }
}
