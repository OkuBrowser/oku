use super::super::core::*;
use super::core::OkuPost;
use super::core::POST_INDEX;
use super::core::POST_INDEX_READER;
use super::core::POST_INDEX_WRITER;
use super::core::POST_SCHEMA;
use crate::fs::util::path_to_entry_key;
use iroh_docs::AuthorId;
use log::error;
use miette::IntoDiagnostic;
use native_db::*;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{collections::HashSet, path::PathBuf};
use tantivy::{collector::TopDocs, query::QueryParser, TantivyDocument};

impl OkuDatabase {
    /// Search OkuNet posts with a query string.
    ///
    /// # Arguments
    ///
    /// * `query_string` - The string used to query for posts.
    ///
    /// * `result_limit` - The maximum number of results to get (defaults to 10).
    ///
    /// # Returns
    ///
    /// A list of OkuNet posts.
    pub fn search_posts(
        query_string: &str,
        result_limit: &Option<usize>,
    ) -> miette::Result<Vec<OkuPost>> {
        let searcher = POST_INDEX_READER.searcher();
        let query_parser = QueryParser::for_index(
            &POST_INDEX,
            vec![
                POST_SCHEMA.1["author_id"],
                POST_SCHEMA.1["path"],
                POST_SCHEMA.1["title"],
                POST_SCHEMA.1["body"],
                POST_SCHEMA.1["tag"],
            ],
        );
        let query = query_parser.parse_query(query_string).into_diagnostic()?;
        let limit = result_limit.unwrap_or(10);
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit).order_by_score())
            .into_diagnostic()?;
        Ok(top_docs
            .par_iter()
            .filter_map(|x| searcher.doc(x.1).ok())
            .collect::<Vec<TantivyDocument>>()
            .into_par_iter()
            .filter_map(|x| TryInto::try_into(x).ok())
            .collect())
    }

    /// Insert or update an OkuNet post.
    ///
    /// # Arguments
    ///
    /// * `post` - An OkuNet post to upsert.
    ///
    /// # Returns
    ///
    /// The previous version of the post, if one existed.
    pub fn upsert_post(&self, post: &OkuPost) -> miette::Result<Option<OkuPost>> {
        let rw: transaction::RwTransaction<'_> =
            self.database.rw_transaction().into_diagnostic()?;
        let old_value: Option<OkuPost> = rw.upsert(post.clone()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;

        let mut index_writer = POST_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        if let Some(old_post) = old_value.clone() {
            index_writer.delete_term(old_post.index_term());
        }
        index_writer
            .add_document(post.to_owned().into())
            .into_diagnostic()?;
        index_writer.commit().into_diagnostic()?;

        Ok(old_value)
    }

    /// Insert or update multiple OkuNet posts.
    ///
    /// # Arguments
    ///
    /// * `posts` - A list of OkuNet posts to upsert.
    ///
    /// # Returns
    ///
    /// A list containing the previous version of each post, if one existed.
    pub fn upsert_posts(&self, posts: &Vec<OkuPost>) -> miette::Result<Vec<Option<OkuPost>>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let old_posts: Vec<_> = posts
            .clone()
            .into_iter()
            .filter_map(|post| rw.upsert(post).ok())
            .collect();
        rw.commit().into_diagnostic()?;

        let mut index_writer = POST_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        old_posts.par_iter().for_each(|old_post| {
            if let Some(old_post) = old_post {
                index_writer.delete_term(old_post.index_term());
            }
        });
        posts.par_iter().for_each(|post| {
            if let Err(e) = index_writer.add_document(post.clone().into()) {
                error!("{e}");
            }
        });
        index_writer.commit().into_diagnostic()?;

        Ok(old_posts)
    }

    /// Delete an OkuNet post.
    ///
    /// # Arguments
    ///
    /// * `post` - An OkuNet post to delete.
    ///
    /// # Returns
    ///
    /// The deleted post.
    pub fn delete_post(&self, post: &OkuPost) -> miette::Result<OkuPost> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_post = rw.remove(post.to_owned()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;

        let mut index_writer = POST_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        index_writer.delete_term(removed_post.index_term());
        index_writer.commit().into_diagnostic()?;

        Ok(removed_post)
    }

    /// Delete multiple OkuNet posts.
    ///
    /// # Arguments
    ///
    /// * `posts` - A list of OkuNet posts to delete.
    ///
    /// # Returns
    ///
    /// A list containing the deleted posts.
    pub fn delete_posts(&self, posts: &[OkuPost]) -> miette::Result<Vec<OkuPost>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_posts: Vec<_> = posts
            .iter()
            .filter_map(|post| rw.remove(post.to_owned()).ok())
            .collect();
        rw.commit().into_diagnostic()?;

        let mut index_writer = POST_INDEX_WRITER
            .clone()
            .try_lock_owned()
            .into_diagnostic()?;
        removed_posts.par_iter().for_each(|removed_post| {
            index_writer.delete_term(removed_post.index_term());
        });
        index_writer.commit().into_diagnostic()?;

        Ok(removed_posts)
    }

    /// Retrieves all known OkuNet posts.
    ///
    /// # Returns
    ///
    /// A list of all known OkuNet posts.
    pub fn get_posts(&self) -> miette::Result<Vec<OkuPost>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        r.scan()
            .primary()
            .into_diagnostic()?
            .all()
            .into_diagnostic()?
            .collect::<Result<Vec<_>, _>>()
            .into_diagnostic()
    }

    /// Retrieves all known OkuNet posts by a given author.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A content authorship ID.
    ///
    /// # Returns
    ///
    /// A list of all known OkuNet posts by the given author.
    pub fn get_posts_by_author(&self, author_id: &AuthorId) -> miette::Result<Vec<OkuPost>> {
        Ok(self
            .get_posts()?
            .into_par_iter()
            .filter(|x| x.entry.author() == *author_id)
            .collect())
    }

    /// Retrieves all known OkuNet posts by a given tag.
    ///
    /// # Arguments
    ///
    /// * `tag` - A tag.
    ///
    /// # Returns
    ///
    /// A list of all known OkuNet posts with the given tag.
    pub fn get_posts_by_tag(&self, tag: &String) -> miette::Result<Vec<OkuPost>> {
        Ok(self
            .get_posts()?
            .into_par_iter()
            .filter(|x| x.note.tags.contains(tag))
            .collect())
    }

    /// Retrieves all distinct tags used in OkuNet posts.
    ///
    /// # Returns
    ///
    /// A list of all tags that appear in an OkuNet post.
    pub fn get_tags(&self) -> miette::Result<HashSet<String>> {
        Ok(self
            .get_posts()?
            .into_iter()
            .flat_map(|x| x.note.tags)
            .collect())
    }

    /// Retrieves an OkuNet post.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A content authorship ID.
    ///
    /// * `path` - A path to a post in the author's home replica.
    ///
    /// # Returns
    ///
    /// The OkuNet post by the given author at the given path, if one exists.
    pub fn get_post(
        &self,
        author_id: &AuthorId,
        path: &PathBuf,
    ) -> miette::Result<Option<OkuPost>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        let entry_key = (
            author_id.as_bytes().to_vec(),
            path_to_entry_key(path).to_vec(),
        );
        r.get().primary(entry_key).into_diagnostic()
    }
}
