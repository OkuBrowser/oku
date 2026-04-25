use super::core::*;
use super::posts::core::OkuPost;
use iroh_docs::sync::Entry;
use iroh_docs::AuthorId;
use log::error;
use miette::IntoDiagnostic;
use native_db::*;
use native_model::{native_model, Model};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::{collections::HashSet, time::SystemTime};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[native_model(id = 1, version = 1)]
#[native_db(
    primary_key(author_id -> Vec<u8>)
)]
/// An Oku user.
pub struct OkuUser {
    /// The content authorship identifier associated with the Oku user.
    pub author_id: AuthorId,
    /// The system time of when this user's content was last retrieved from OkuNet.
    pub last_fetched: SystemTime,
    /// The posts made by this user on OkuNet.
    pub posts: Vec<Entry>,
    /// The OkuNet identity of the user.
    pub identity: Option<OkuIdentity>,
}

impl PartialEq for OkuUser {
    fn eq(&self, other: &Self) -> bool {
        self.author_id == other.author_id
    }
}
impl Eq for OkuUser {}
impl Hash for OkuUser {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.author_id.hash(state);
    }
}

impl OkuUser {
    fn author_id(&self) -> Vec<u8> {
        self.author_id.as_bytes().to_vec()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
/// An OkuNet identity for an Oku user.
pub struct OkuIdentity {
    /// The display name of the Oku user.
    pub name: String,
    /// The content authors followed by the Oku user.
    /// OkuNet content is retrieved from followed users and the users those users follow.
    pub following: HashSet<AuthorId>,
    /// The content authors blocked by the Oku user.
    /// Blocked authors are ignored when fetching new OkuNet posts.
    pub blocked: HashSet<AuthorId>,
}

impl OkuDatabase {
    /// Insert or update an OkuNet user.
    ///
    /// # Arguments
    ///
    /// * `user` - An OkuNet user to upsert.
    ///
    /// # Returns
    ///
    /// The previous version of the user, if one existed.
    pub fn upsert_user(&self, user: &OkuUser) -> miette::Result<Option<OkuUser>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let old_value: Option<OkuUser> = rw.upsert(user.to_owned()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;
        Ok(old_value)
    }

    /// Delete an OkuNet user.
    ///
    /// # Arguments
    ///
    /// * `user` - An OkuNet user to delete.
    ///
    /// # Returns
    ///
    /// The deleted user.
    pub fn delete_user(&self, user: &OkuUser) -> miette::Result<OkuUser> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_user = rw.remove(user.to_owned()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;
        Ok(removed_user)
    }

    /// Delete multiple OkuNet users.
    ///
    /// # Arguments
    ///
    /// * `users` - A list of OkuNet users to delete.
    ///
    /// # Returns
    ///
    /// A list containing the deleted users.
    pub fn delete_users(&self, users: &[OkuUser]) -> miette::Result<Vec<OkuUser>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_users = users
            .iter()
            .filter_map(|user| rw.remove(user.to_owned()).ok())
            .collect();
        rw.commit().into_diagnostic()?;
        Ok(removed_users)
    }

    /// Delete multiple OkuNet users and their posts.
    ///
    /// # Arguments
    ///
    /// * `users` - A list of OkuNet users to delete.
    ///
    /// # Returns
    ///
    /// A list containing the deleted posts.
    pub fn delete_users_with_posts(&self, users: &[OkuUser]) -> miette::Result<Vec<OkuPost>> {
        Ok(self
            .delete_users(users)?
            .par_iter()
            .filter_map(|x| self.get_posts_by_author(&x.author_id).ok())
            .collect::<Vec<_>>()
            .into_par_iter()
            .flat_map(|x| self.delete_posts(&x).ok())
            .collect::<Vec<_>>()
            .concat())
    }

    /// Deletes OkuNet users by their author IDs and posts by authors with those IDs.
    ///
    /// Differs from [`Self::delete_users_with_posts`] as a post will still be deleted even if a record for the authoring user is not found.
    ///
    /// # Arguments
    ///
    /// * `author_ids` - A list of content authorship IDs.
    pub fn delete_by_author_ids(&self, author_ids: &Vec<AuthorId>) -> miette::Result<()> {
        let users: Vec<_> = author_ids
            .par_iter()
            .filter_map(|x| self.get_user(x).ok().flatten())
            .collect();
        let posts: Vec<_> = author_ids
            .into_par_iter()
            .filter_map(|x| self.get_posts_by_author(x).ok())
            .flatten()
            .collect();
        if let Err(e) = self.delete_users(&users) {
            error!("{}", e);
        }
        if let Err(e) = self.delete_posts(&posts) {
            error!("{}", e);
        }
        Ok(())
    }

    /// Gets the content authorship IDs of all locally-known users.
    ///
    /// This differs from [`Self::get_users`] as IDs of authors with posts but no user records are included.
    ///
    /// # Returns
    ///
    /// A list of IDs for all users that have content in the local database.
    pub fn all_local_users(&self) -> Vec<AuthorId> {
        let user_records: HashSet<_> = self
            .get_users()
            .unwrap_or_default()
            .par_iter()
            .map(|x| x.author_id)
            .collect();
        let post_record_users: HashSet<_> = self
            .get_posts()
            .unwrap_or_default()
            .par_iter()
            .map(|x| x.entry.author())
            .collect();
        user_records
            .union(&post_record_users)
            .map(|x| x.to_owned())
            .collect()
    }

    /// Gets the OkuNet content of all known users.
    ///
    /// # Returns
    ///
    /// The OkuNet content of all users known to this node.
    pub fn get_users(&self) -> miette::Result<Vec<OkuUser>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        r.scan()
            .primary()
            .into_diagnostic()?
            .all()
            .into_diagnostic()?
            .collect::<Result<Vec<_>, _>>()
            .into_diagnostic()
    }

    /// Gets an OkuNet user's content by their content authorship ID.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A content authorship ID.
    ///
    /// # Returns
    ///
    /// An OkuNet user's content.
    pub fn get_user(&self, author_id: &AuthorId) -> miette::Result<Option<OkuUser>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        r.get()
            .primary(author_id.as_bytes().to_vec())
            .into_diagnostic()
    }
}
