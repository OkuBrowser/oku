//! Metrics for iroh-docs

use iroh_metrics::{Counter, MetricsGroup};

/// Metrics for iroh-docs
#[derive(Debug, Default, MetricsGroup)]
pub struct Metrics {
    /// Number of document entries added locally
    pub new_entries_local: Counter,
    /// Number of document entries added by peers
    pub new_entries_remote: Counter,
    /// Total size of entry contents added locally
    pub new_entries_local_size: Counter,
    /// Total size of entry contents added by peers
    pub new_entries_remote_size: Counter,
    /// Number of successful syncs (via accept)
    pub sync_via_accept_success: Counter,
    /// Number of failed syncs (via accept)
    pub sync_via_accept_failure: Counter,
    /// Number of successful syncs (via connect)
    pub sync_via_connect_success: Counter,
    /// Number of failed syncs (via connect)
    pub sync_via_connect_failure: Counter,

    /// Number of times the main actor loop ticked
    pub actor_tick_main: Counter,

    /// Number of times the gossip actor loop ticked
    pub doc_gossip_tick_main: Counter,
    /// Number of times the gossip actor processed an event
    pub doc_gossip_tick_event: Counter,
    /// Number of times the gossip actor processed an actor event
    pub doc_gossip_tick_actor: Counter,
    /// Number of times the gossip actor processed a pending join
    pub doc_gossip_tick_pending_join: Counter,

    /// Number of times the live actor loop ticked
    pub doc_live_tick_main: Counter,
    /// Number of times the live actor processed an actor event
    pub doc_live_tick_actor: Counter,
    /// Number of times the live actor processed a replica event
    pub doc_live_tick_replica_event: Counter,
    /// Number of times the live actor processed a running sync connect
    pub doc_live_tick_running_sync_connect: Counter,
    /// Number of times the live actor processed a running sync accept
    pub doc_live_tick_running_sync_accept: Counter,
    /// Number of times the live actor processed a pending download
    pub doc_live_tick_pending_downloads: Counter,
}
