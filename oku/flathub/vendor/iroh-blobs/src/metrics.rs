//! Metrics for iroh-blobs

use iroh_metrics::{Counter, MetricsGroup};

/// Enum of metrics for the module
#[allow(missing_docs)]
#[allow(dead_code)]
#[derive(Debug, Default, MetricsGroup)]
#[metrics(name = "iroh-blobs")]
pub struct Metrics {
    /// Total number of content bytes downloaded
    pub download_bytes_total: Counter,
    /// Total time in ms spent downloading content bytes
    pub download_time_total: Counter,
    /// Total number of successful downloads
    pub downloads_success: Counter,
    /// Total number of downloads failed with error
    pub downloads_error: Counter,
    /// Total number of downloads failed with not found
    pub downloads_notfound: Counter,
    /// Number of times the main downloader actor loop ticked
    pub downloader_tick_main: Counter,
    /// Number of times the downloader actor ticked for a connection ready
    pub downloader_tick_connection_ready: Counter,
    /// Number of times the downloader actor ticked for a message received
    pub downloader_tick_message_received: Counter,
    /// Number of times the downloader actor ticked for a transfer completed
    pub downloader_tick_transfer_completed: Counter,
    /// Number of times the downloader actor ticked for a transfer failed
    pub downloader_tick_transfer_failed: Counter,
    /// Number of times the downloader actor ticked for a retry node
    pub downloader_tick_retry_node: Counter,
    /// Number of times the downloader actor ticked for a goodbye node
    pub downloader_tick_goodbye_node: Counter,
}
