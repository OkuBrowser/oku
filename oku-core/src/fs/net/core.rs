use crate::fs::util::{path_to_entry_key, path_to_entry_prefix};
use iroh_docs::store::FilterKind;
use iroh_docs::Author;
use iroh_docs::DocTicket;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
/// An Oku user's credentials, which are sensitive, exported from a node, able to be imported into another.
pub struct ExportedUser {
    pub(crate) author: Author,
    pub(crate) home_replica_ticket: Option<DocTicket>,
}

/// Filters to prevent downloading the entirety of a home replica.
/// Only the `/profile.toml` file and the `/posts/` directory are downloaded.
///
/// # Returns
///
/// The download filters specifying the only content allowed to be downloaded from a home replica.
pub fn home_replica_filters() -> Vec<FilterKind> {
    let profile_filter = FilterKind::Exact(path_to_entry_key(&"/profile.toml".into()));
    let posts_filter = FilterKind::Prefix(path_to_entry_prefix(&"/posts/".into()));
    vec![profile_filter, posts_filter]
}
