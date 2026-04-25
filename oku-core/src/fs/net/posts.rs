use super::core::home_replica_filters;
use crate::fs::util::entry_key_to_path;
use crate::{
    database::{
        core::DATABASE,
        posts::core::{OkuNote, OkuPost},
        users::OkuUser,
    },
    fs::OkuFs,
};
use dashmap::DashMap;
use iroh_blobs::Hash;
use iroh_docs::sync::Entry;
use iroh_docs::AuthorId;
use log::error;
use miette::IntoDiagnostic;
use rayon::iter::{
    FromParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::atomic::AtomicUsize,
};
use url::Url;

impl OkuFs {
    /// Retrieves the OkuNet posts by the local user, if any.
    ///
    /// # Returns
    ///
    /// A list of the OkuNet posts by the local user.
    pub async fn posts(&self) -> Option<Vec<OkuPost>> {
        let post_files = self
            .read_directory(&self.home_replica().await?, Path::new("/posts/"))
            .await
            .ok()
            .unwrap_or_default();
        Some(
            post_files
                .par_iter()
                .filter(|(entry, _)| {
                    entry_key_to_path(entry.key())
                        .map(|x| matches!(x.extension(), Some(y) if y == "toml"))
                        .unwrap_or(false)
                })
                .filter_map(|(entry, bytes)| {
                    toml::from_str::<OkuNote>(String::from_utf8_lossy(bytes).as_ref())
                        .ok()
                        .map(|x| OkuPost {
                            entry: entry.clone(),
                            note: x,
                        })
                })
                .collect(),
        )
    }

    /// Retrieve all posts known to this Oku node.
    ///
    /// # Returns
    ///
    /// All posts known to this Oku node.
    pub async fn all_posts(&self) -> HashSet<OkuPost> {
        let mut posts = HashSet::<_>::from_par_iter(self.posts().await.unwrap_or_default());
        posts.extend(DATABASE.get_posts().unwrap_or_default());
        posts
    }

    /// Filters posts containing at least one of the given tags.
    ///
    /// # Arguments
    ///
    /// * `posts` - A set of posts.
    ///
    /// * `tags` - A set of tags.
    ///
    /// # Returns
    ///
    /// A list of OkuNet posts with the given tags.
    pub async fn posts_with_tags(&self, posts: &[OkuPost], tags: &HashSet<String>) -> Vec<OkuPost> {
        posts
            .to_owned()
            .into_par_iter()
            .filter(|x| !x.note.tags.is_disjoint(tags))
            .collect()
    }

    /// Retrieves the set of all tags that appear in the given posts.
    ///
    /// # Arguments
    ///
    /// * `posts` - A set of posts.
    ///
    /// # Returns
    ///
    /// All tags that appear across the posts.
    pub async fn all_tags(&self, posts: &HashSet<OkuPost>) -> HashSet<String> {
        HashSet::<_>::from_par_iter(posts.into_par_iter().flat_map(|x| x.note.tags.clone()))
    }

    /// Retrieves a mapping of tags to the number of posts containing them.
    ///
    /// # Arguments
    ///
    /// * `posts` - A set of posts.
    ///
    /// # Returns
    ///
    /// All tags that appear across the posts, and how often they appear.
    pub async fn count_tags(&self, posts: &HashSet<OkuPost>) -> HashMap<String, usize> {
        let result: DashMap<String, AtomicUsize> = DashMap::new();
        posts.into_par_iter().for_each(|x| {
            x.note.tags.par_iter().for_each(|y| match result.get(y) {
                None => {
                    result.insert(y.to_owned(), AtomicUsize::new(1));
                }
                Some(v) => {
                    v.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            });
        });
        result
            .into_par_iter()
            .map(|(k, v)| (k, v.into_inner()))
            .collect()
    }

    /// Retrieves an OkuNet post authored by the local user using its path.
    ///
    /// # Arguments
    ///
    /// * `path` - A path to a post in the user's home replica.
    ///
    /// # Returns
    ///
    /// The OkuNet post at the given path.
    pub async fn post(&self, path: &PathBuf) -> miette::Result<OkuPost> {
        let namespace_id = self
            .home_replica()
            .await
            .ok_or(miette::miette!("Home replica not set … "))?;
        match self.read_file(&namespace_id, path).await {
            Ok(bytes) => {
                let note = toml::from_str::<OkuNote>(String::from_utf8_lossy(&bytes).as_ref())
                    .into_diagnostic()?;
                Ok(OkuPost {
                    entry: self.get_entry(&namespace_id, path).await?,
                    note,
                })
            }
            Err(e) => Err(miette::miette!("{}", e)),
        }
    }

    /// Attempts to retrieve an OkuNet post from a file entry.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to parse.
    ///
    /// # Returns
    ///
    /// An OkuNet post, if the entry represents one.
    pub async fn post_from_entry(&self, entry: &Entry) -> miette::Result<OkuPost> {
        let bytes = self
            .content_bytes(entry)
            .await
            .map_err(|e| miette::miette!("{}", e))?;
        let note = toml::from_str::<OkuNote>(String::from_utf8_lossy(&bytes).as_ref())
            .into_diagnostic()?;
        Ok(OkuPost {
            entry: entry.clone(),
            note,
        })
    }

    /// Retrieves OkuNet posts from the file entries in an [`OkuUser`].
    ///
    /// # Arguments
    ///
    /// * `user` - The OkuNet user record containing the file entries.
    ///
    /// # Returns
    ///
    /// A list of OkuNet posts contained within the user record.
    pub async fn posts_from_user(&self, user: &OkuUser) -> miette::Result<Vec<OkuPost>> {
        let mut posts: Vec<_> = Vec::new();
        for post in user.posts.clone() {
            posts.push(self.post_from_entry(&post).await?);
        }
        Ok(posts)
    }

    /// Create or modify an OkuNet post in the user's home replica.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to create, or modify, the post at; a suggested path is generated if none is provided.
    ///
    /// * `url` - The URL the post is regarding.
    ///
    /// * `title` - The title of the post.
    ///
    /// * `body` - The body of the post.
    ///
    /// * `tags` - A list of tags associated with the post.
    ///
    /// # Returns
    ///
    /// A hash of the post's content.
    pub async fn create_or_modify_post(
        &self,
        url: &Url,
        title: &String,
        body: &String,
        tags: &HashSet<String>,
    ) -> miette::Result<Hash> {
        let home_replica_id = self
            .home_replica()
            .await
            .ok_or(miette::miette!("No home replica set … "))?;
        let new_note = OkuNote {
            url: url.clone(),
            title: title.to_string(),
            body: body.to_string(),
            tags: tags.clone(),
        };
        let post_path = &new_note.post_path().into();
        self.create_or_modify_file(
            &home_replica_id,
            post_path,
            toml::to_string_pretty(&new_note).into_diagnostic()?,
        )
        .await
    }

    /// Delete an OkuNet post in the user's home replica.
    ///
    /// # Arguments
    ///
    /// * `path` - A path to a post in the user's home replica.
    ///
    /// # Returns
    ///
    /// The number of entries deleted in the replica, which should be 1 if the file was successfully deleted.
    pub async fn delete_post(&self, path: &PathBuf) -> miette::Result<usize> {
        let home_replica_id = self
            .home_replica()
            .await
            .ok_or(miette::miette!("No home replica set … "))?;
        self.delete_file(&home_replica_id, path).await
    }

    /// Join a swarm to fetch the latest version of an OkuNet post.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The authorship ID of the post's author.
    ///
    /// * `path` - The path to the post in the author's home replica.
    ///
    /// # Returns
    ///
    /// The requested OkuNet post.
    pub async fn fetch_post(
        &self,
        author_id: &AuthorId,
        path: &PathBuf,
    ) -> miette::Result<OkuPost> {
        let ticket = self
            .resolve_author_id(author_id)
            .await
            .map_err(|e| miette::miette!("{}", e))?;
        let namespace_id = ticket.capability.id();
        match self
            .fetch_file_with_ticket(&ticket, path, &Some(home_replica_filters()))
            .await
        {
            Ok(bytes) => {
                let note = toml::from_str::<OkuNote>(String::from_utf8_lossy(&bytes).as_ref())
                    .into_diagnostic()?;
                if let Err(e) = self
                    .fetch_post_embeddings(&ticket, author_id, note.url.as_ref())
                    .await
                {
                    error!("{e}")
                }
                Ok(OkuPost {
                    entry: self.get_entry(&namespace_id, path).await?,
                    note,
                })
            }
            Err(e) => Err(miette::miette!("{}", e)),
        }
    }

    /// Retrieves an OkuNet post from the database, or from the mainline DHT if not found locally.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The authorship ID of the post's author.
    ///
    /// * `path` - The path to the post in the author's home replica.
    ///
    /// # Returns
    ///
    /// The requested OkuNet post.
    pub async fn get_or_fetch_post(
        &self,
        author_id: &AuthorId,
        path: &PathBuf,
    ) -> miette::Result<OkuPost> {
        match DATABASE.get_post(author_id, path).ok().flatten() {
            Some(post) => Ok(post),
            None => self.fetch_post(author_id, path).await,
        }
    }
}
