use super::core::{home_replica_filters, ExportedUser};
use crate::{
    config::OkuFsConfig,
    database::{
        core::DATABASE,
        posts::core::{OkuNote, OkuPost},
        users::{OkuIdentity, OkuUser},
    },
    fs::OkuFs,
};
use futures::StreamExt;
use iroh_blobs::Hash;
use iroh_docs::sync::CapabilityKind;
use iroh_docs::AuthorId;
use iroh_docs::DocTicket;
use iroh_docs::NamespaceId;
use iroh_docs::{api::protocol::ShareMode, Author, NamespaceSecret};
use log::debug;
use miette::IntoDiagnostic;
use rayon::iter::{
    FromParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use std::{collections::HashSet, path::Path, time::SystemTime};

impl OkuFs {
    /// Retrieve the content authorship ID used by the node.
    ///
    /// # Returns
    ///
    /// The content authorship ID used by the node.
    pub async fn default_author(&self) -> AuthorId {
        self.docs.author_default().await.unwrap_or_default()
    }

    /// Exports the local Oku user's credentials.
    ///
    /// # Returns
    ///
    /// The local Oku user's credentials, containing sensitive information.
    pub async fn export_user(&self) -> Option<ExportedUser> {
        let default_author = self.get_author().await.ok();
        let home_replica = self.home_replica().await;
        let home_replica_ticket = match home_replica {
            Some(home_replica_id) => self
                .create_document_ticket(&home_replica_id, &ShareMode::Write)
                .await
                .ok(),
            None => None,
        };
        default_author.map(|author| ExportedUser {
            author,
            home_replica,
            home_replica_ticket,
        })
    }

    /// Imports Oku user credentials that were exported from another node.
    ///
    /// # Arguments
    ///
    /// * `exported_user` - Oku user credentials, which contain sensitive information.
    pub async fn import_user(&self, exported_user: &ExportedUser) -> miette::Result<()> {
        self.docs
            .author_import(exported_user.author.clone())
            .await
            .map_err(|e| miette::miette!("{}", e))?;
        self.docs
            .author_set_default(exported_user.author.id())
            .await
            .map_err(|e| miette::miette!("{}", e))?;
        match (
            exported_user.home_replica,
            exported_user.home_replica_ticket.clone(),
        ) {
            (Some(home_replica), Some(home_replica_ticket)) => match self
                .fetch_replica_by_ticket(&home_replica_ticket, &None, &None)
                .await
            {
                Ok(_) => (),
                Err(_e) => self
                    .fetch_replica_by_id(&home_replica, &None)
                    .await
                    .map_err(|e| miette::miette!("{}", e))?,
            },
            (Some(home_replica), None) => self
                .fetch_replica_by_id(&home_replica, &None)
                .await
                .map_err(|e| miette::miette!("{}", e))?,
            _ => (),
        }
        Ok(())
    }

    /// Exports the local Oku user's credentials in TOML format.
    ///
    /// # Returns
    ///
    /// The local Oku user's credentials, containing sensitive information.
    pub async fn export_user_toml(&self) -> miette::Result<String> {
        toml::to_string(
            &self
                .export_user()
                .await
                .ok_or(miette::miette!("No authorship credentials to export … "))?,
        )
        .into_diagnostic()
    }

    /// Imports Oku user credentials that were exported from another node.
    ///
    /// # Arguments
    ///
    /// * `exported_user` - Oku user credentials, encoded in TOML format. They contain sensitive information.
    pub async fn import_user_toml(&self, exported_user_toml: &str) -> miette::Result<()> {
        let exported_user: ExportedUser = toml::from_str(exported_user_toml).into_diagnostic()?;
        self.import_user(&exported_user).await
    }

    /// Creates the home replica of a known author.
    ///
    /// # Arguments
    ///
    /// * `author` - An optional author keypair to create the home replica for. If not provided, a home replica for the default author is created. The author keypair must be one already imported.
    ///
    /// # Returns
    ///
    /// The replica ID of the created home replica.
    pub async fn create_home_replica(
        &self,
        author: &Option<Author>,
    ) -> miette::Result<NamespaceId> {
        let given_author_id = author.as_ref().map(|x| x.id());
        if let Some(given_author_id) = given_author_id.as_ref() {
            let is_known_author_id = self
                .docs
                .author_list()
                .await
                .map_err(|e| miette::miette!(e))?
                .any(|x| async move { x.map_or(false, |x| x == *given_author_id) })
                .await;
            if !is_known_author_id {
                return Err(miette::miette!("Cannot create home replica for authors whose private key is unknown (author ID: {})", crate::fs::util::fmt_short(given_author_id)));
            }
        }

        let default_author = self.get_author().await.map_err(|e| miette::miette!(e))?;
        let author = author.as_ref().unwrap_or(&default_author);
        debug!(
            "Attempting to create home replica for author with ID {} … ",
            crate::fs::util::fmt_short(author.id())
        );
        let home_replica = self
            .docs
            .import_namespace(iroh_docs::Capability::Write(NamespaceSecret::from_bytes(
                &author.to_bytes(),
            )))
            .await
            .map_err(|e| miette::miette!(e))?;
        self.replica_sender.send_replace(());
        debug!(
            "Created home replica for author with ID {} … ",
            crate::fs::util::fmt_short(author.id())
        );
        Ok(home_replica.id())
    }

    /// Retrieve the home replica of the Oku user, creating it if it does not yet exist.
    ///
    /// # Returns
    ///
    /// The home replica of the Oku user, if it already existed or was able to be created successfully.
    pub async fn home_replica(&self) -> Option<NamespaceId> {
        let home_replica = NamespaceId::from(self.default_author().await.as_bytes());
        let home_replica_capability = self.get_replica_capability(&home_replica).await.ok();
        let home_replica_exists = match home_replica_capability {
            Some(CapabilityKind::Write) => Some(home_replica),
            Some(CapabilityKind::Read) => None,
            None => None,
        };
        if let None = home_replica_exists {
            debug!("Home replica does not exist; creating … ");
            self.create_home_replica(&None).await.ok()
        } else {
            home_replica_exists
        }
    }

    /// Retrieves the OkuNet identity of the local user.
    ///
    /// # Returns
    ///
    /// The local user's OkuNet identity, if they have one.
    pub async fn identity(&self) -> Option<OkuIdentity> {
        let profile_bytes = self
            .read_file(&self.home_replica().await?, &"/profile.toml".into())
            .await
            .ok()?;
        toml::from_str(String::from_utf8_lossy(&profile_bytes).as_ref()).ok()
    }

    /// Replaces the current OkuNet identity of the local user.
    ///
    /// # Arguments
    ///
    /// * `identity` - The new OkuNet identity.
    ///
    /// # Returns
    ///
    /// The hash of the new identity file in the local user's home replica.
    pub async fn set_identity(&self, identity: &OkuIdentity) -> miette::Result<Hash> {
        // It is not valid to follow or unfollow yourself.
        let mut validated_identity = identity.clone();
        let me = self.default_author().await;
        validated_identity.following.retain(|y| me != *y);
        validated_identity.blocked.retain(|y| me != *y);
        // It is not valid to follow blocked people.
        validated_identity.following = validated_identity
            .following
            .difference(&validated_identity.blocked)
            .copied()
            .collect();

        self.create_or_modify_file(
            &self
                .home_replica()
                .await
                .ok_or(miette::miette!("No home replica set … "))?,
            &"/profile.toml".into(),
            toml::to_string_pretty(&validated_identity).into_diagnostic()?,
        )
        .await
    }

    /// Replaces the current display name of the local user.
    ///
    /// # Arguments
    ///
    /// * `display_name` - The new display name.
    ///
    /// # Returns
    ///
    /// # The hash of the new identity file in the local user's home replica.
    pub async fn set_display_name(&self, display_name: &String) -> miette::Result<Hash> {
        let mut identity = self.identity().await.unwrap_or_default();
        identity.name = display_name.to_string();
        self.set_identity(&identity).await
    }

    /// Follow or unfollow a user.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The user to follow or unfollow's content authorship ID.
    ///
    /// # Returns
    ///
    /// The hash of the new identity file in the local user's home replica.
    pub async fn toggle_follow(&self, author_id: &AuthorId) -> miette::Result<Hash> {
        let mut identity = self.identity().await.unwrap_or_default();
        match identity.following.contains(author_id) {
            true => identity.following.remove(author_id),
            false => identity.following.insert(*author_id),
        };
        self.set_identity(&identity).await
    }

    /// Follow a user.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The user to follow's content authorship ID.
    ///
    /// # Returns
    ///
    /// The hash of the new identity file in the local user's home replica.
    pub async fn follow(&self, author_id: &AuthorId) -> miette::Result<Hash> {
        let mut identity = self.identity().await.unwrap_or_default();
        match identity.following.contains(author_id) {
            true => (),
            false => {
                identity.following.insert(*author_id);
            }
        };
        self.set_identity(&identity).await
    }

    /// Unfollow a user.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The user to unfollow's content authorship ID.
    ///
    /// # Returns
    ///
    /// The hash of the new identity file in the local user's home replica.
    pub async fn unfollow(&self, author_id: &AuthorId) -> miette::Result<Hash> {
        let mut identity = self.identity().await.unwrap_or_default();
        if identity.following.contains(author_id) {
            identity.following.remove(author_id);
        };
        self.set_identity(&identity).await
    }

    /// Block or unblock a user.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The user to block or unblock's content authorship ID.
    ///
    /// # Returns
    ///
    /// The hash of the new identity file in the local user's home replica.
    pub async fn toggle_block(&self, author_id: &AuthorId) -> miette::Result<Hash> {
        let mut identity = self.identity().await.unwrap_or_default();
        match identity.blocked.contains(author_id) {
            true => identity.blocked.remove(author_id),
            false => identity.blocked.insert(*author_id),
        };
        self.set_identity(&identity).await
    }

    /// Block a user.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The user to block's content authorship ID.
    ///
    /// # Returns
    ///
    /// The hash of the new identity file in the local user's home replica.
    pub async fn block(&self, author_id: &AuthorId) -> miette::Result<Hash> {
        let mut identity = self.identity().await.unwrap_or_default();
        match identity.blocked.contains(author_id) {
            true => (),
            false => {
                identity.blocked.insert(*author_id);
            }
        };
        self.set_identity(&identity).await
    }

    /// Unblock a user.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The user to unblock's content authorship ID.
    ///
    /// # Returns
    ///
    /// The hash of the new identity file in the local user's home replica.
    pub async fn unblock(&self, author_id: &AuthorId) -> miette::Result<Hash> {
        let mut identity = self.identity().await.unwrap_or_default();
        if identity.blocked.contains(author_id) {
            identity.blocked.remove(author_id);
        };
        self.set_identity(&identity).await
    }

    /// Check if a user is followed.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The user's content authorship ID.
    ///
    /// # Returns
    ///
    /// Whether or not the user is followed.
    pub async fn is_followed(&self, author_id: &AuthorId) -> bool {
        self.identity()
            .await
            .map(|x| x.following.contains(author_id))
            .unwrap_or(false)
    }

    /// Check if a user is blocked.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The user's content authorship ID.
    ///
    /// # Returns
    ///
    /// Whether or not the user is blocked.
    pub async fn is_blocked(&self, author_id: &AuthorId) -> bool {
        self.identity()
            .await
            .map(|x| x.blocked.contains(author_id))
            .unwrap_or(false)
    }

    /// Check whether or not an author ID is the local user's.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A user's content authorship ID.
    ///
    /// # Returns
    ///
    /// Whether or not the user's authorship ID is the local user's.
    pub async fn is_me(&self, author_id: &AuthorId) -> bool {
        &self.default_author().await == author_id
    }

    /// Retrieves an [`OkuUser`] representing the local user.
    ///
    /// # Returns
    ///
    /// An [`OkuUser`] representing the current user, as if it were retrieved from another Oku user's database.
    pub async fn user(&self) -> miette::Result<OkuUser> {
        Ok(OkuUser {
            author_id: self.default_author().await,
            last_fetched: SystemTime::now(),
            posts: self
                .posts()
                .await
                .map(|x| x.into_par_iter().map(|y| y.entry).collect())
                .unwrap_or_default(),
            identity: self.identity().await,
        })
    }

    /// Refreshes any user data last retrieved longer than [`crate::config::OkuFsConfig::get_republish_delay`] ago according to the system time; the users one is following, and the users they're following, are recorded locally.
    /// Blocked users are not recorded.
    pub async fn refresh_users(&self) -> miette::Result<()> {
        // Wanted users: followed users
        // Unwanted users: blocked users, unfollowed users
        let (followed_users, blocked_users) = match self.identity().await {
            Some(identity) => (identity.following, identity.blocked),
            None => (HashSet::new(), HashSet::new()),
        };
        // In case a user is somehow followed and blocked (additional checks should already prevent this)
        let users_to_add: HashSet<_> = followed_users
            .difference(&blocked_users)
            .map(|x| x.to_owned())
            .collect();
        let local_users: HashSet<_> = DATABASE.all_local_users().into_par_iter().collect();
        let users_to_delete: HashSet<_> = local_users
            .difference(&users_to_add)
            .map(|x| x.to_owned())
            .collect();

        for user_id in users_to_add {
            let user = self.get_or_fetch_user(&user_id).await?;
            let (user_followed_users, user_blocked_users) = match user.identity {
                Some(identity) => (identity.following, identity.blocked),
                None => (HashSet::new(), HashSet::new()),
            };
            for user_user in user_followed_users.difference(&user_blocked_users) {
                self.get_or_fetch_user(user_user).await?;
            }
        }
        DATABASE.delete_by_author_ids(&Vec::from_par_iter(users_to_delete))?;
        Ok(())
    }

    /// Retrieves user data regardless of when last retrieved; the users one is following, and the users they're following, are recorded locally.
    /// Blocked users are not recorded.
    pub async fn fetch_users(&self) -> miette::Result<()> {
        // Wanted users: followed users
        // Unwanted users: blocked users, unfollowed users
        let (followed_users, blocked_users) = match self.identity().await {
            Some(identity) => (identity.following, identity.blocked),
            None => (HashSet::new(), HashSet::new()),
        };
        // In case a user is somehow followed and blocked (additional checks should already prevent this)
        let users_to_add: HashSet<_> = followed_users
            .difference(&blocked_users)
            .map(|x| x.to_owned())
            .collect();
        let local_users: HashSet<_> = DATABASE.all_local_users().into_par_iter().collect();
        let users_to_delete: HashSet<_> = local_users
            .difference(&users_to_add)
            .map(|x| x.to_owned())
            .collect();

        for user_id in users_to_add {
            let user = self.fetch_user(&user_id).await?;
            let (user_followed_users, user_blocked_users) = match user.identity {
                Some(identity) => (identity.following, identity.blocked),
                None => (HashSet::new(), HashSet::new()),
            };
            for user_user in user_followed_users.difference(&user_blocked_users) {
                self.fetch_user(user_user).await?;
            }
        }
        DATABASE.delete_by_author_ids(&Vec::from_par_iter(users_to_delete))?;
        Ok(())
    }

    /// Use the mainline DHT to obtain a ticket for the home replica of the user with the given content authorship ID.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A content authorship ID.
    ///
    /// # Returns
    ///
    /// A ticket for the home replica of the user with the given content authorship ID.
    pub async fn resolve_author_id(&self, author_id: &AuthorId) -> anyhow::Result<DocTicket> {
        self.okunet_fetch_sender.send_replace(true);
        let ticket = self
            .resolve_namespace_id(&NamespaceId::from(author_id.as_bytes()))
            .await;
        self.okunet_fetch_sender.send_replace(false);
        ticket
    }

    /// Join a swarm to fetch the latest version of a home replica and obtain the OkuNet identity within it.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A content authorship ID.
    ///
    /// # Returns
    ///
    /// The OkuNet identity within the home replica of the user with the given content authorship ID.
    pub async fn fetch_profile(&self, ticket: &DocTicket) -> miette::Result<OkuIdentity> {
        match self
            .fetch_file_with_ticket(
                ticket,
                &"/profile.toml".into(),
                &Some(home_replica_filters()),
            )
            .await
        {
            Ok(profile_bytes) => Ok(toml::from_str(
                String::from_utf8_lossy(&profile_bytes).as_ref(),
            )
            .into_diagnostic()?),
            Err(e) => Err(miette::miette!("{}", e)),
        }
    }

    /// Join a swarm to fetch the latest version of a home replica and obtain the OkuNet posts within it.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A content authorship ID.
    ///
    /// # Returns
    ///
    /// The OkuNet posts within the home replica of the user with the given content authorship ID.
    pub async fn fetch_posts(&self, ticket: &DocTicket) -> miette::Result<Vec<OkuPost>> {
        match self
            .fetch_directory_with_ticket(
                ticket,
                Path::new("/posts/"),
                &Some(home_replica_filters()),
            )
            .await
        {
            Ok(post_files) => Ok(post_files
                .par_iter()
                .filter_map(|(entry, bytes)| {
                    toml::from_str::<OkuNote>(String::from_utf8_lossy(bytes).as_ref())
                        .ok()
                        .map(|x| OkuPost {
                            entry: entry.clone(),
                            note: x,
                        })
                })
                .collect()),
            Err(e) => Err(miette::miette!("{}", e)),
        }
    }

    /// Obtain an OkuNet user's content, identified by their content authorship ID.
    ///
    /// If last retrieved longer than [`crate::config::OkuFsConfig::get_republish_delay`] ago according to the system time, a known user's content will be re-fetched.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A content authorship ID.
    ///
    /// # Returns
    ///
    /// An OkuNet user's content.
    pub async fn get_or_fetch_user(&self, author_id: &AuthorId) -> miette::Result<OkuUser> {
        let config = OkuFsConfig::load_or_create_config().unwrap_or_default();
        let republish_delay = config.get_republish_delay();
        match DATABASE.get_user(author_id).ok().flatten() {
            Some(user) => {
                match SystemTime::now()
                    .duration_since(user.last_fetched)
                    .into_diagnostic()?
                    > republish_delay
                {
                    true => self.fetch_user(author_id).await,
                    false => Ok(user),
                }
            }
            None => self.fetch_user(author_id).await,
        }
    }

    /// Fetch the latest version of an OkuNet user's content, identified by their content authorship ID.
    ///
    /// # Arguments
    ///
    /// * `author_id` - A content authorship ID.
    ///
    /// # Returns
    ///
    /// The latest version of an OkuNet user's content.
    pub async fn fetch_user(&self, author_id: &AuthorId) -> miette::Result<OkuUser> {
        self.okunet_fetch_sender.send_replace(true);
        let ticket = self
            .resolve_author_id(author_id)
            .await
            .map_err(|e| miette::miette!("{}", e))?;

        let profile = self.fetch_profile(&ticket).await.ok();
        let posts = self.fetch_posts(&ticket).await.unwrap_or_default();
        DATABASE.upsert_posts(&posts)?;
        DATABASE.upsert_user(&OkuUser {
            author_id: *author_id,
            last_fetched: SystemTime::now(),
            posts: posts.into_par_iter().map(|y| y.entry).collect(),
            identity: profile,
        })?;
        self.okunet_fetch_sender.send_replace(false);
        DATABASE
            .get_user(author_id)?
            .ok_or(miette::miette!("User {} not found … ", author_id))
    }
}
