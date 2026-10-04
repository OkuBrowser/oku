//! Metrics for iroh-gossip

use iroh_metrics::{Counter, MetricsGroup};

/// Enum of metrics for the module
#[derive(Debug, Default, MetricsGroup)]
#[metrics(name = "gossip")]
pub struct Metrics {
    /// Number of control messages sent
    pub msgs_ctrl_sent: Counter,
    /// Number of control messages received
    pub msgs_ctrl_recv: Counter,
    /// Number of data messages sent
    pub msgs_data_sent: Counter,
    /// Number of data messages received
    pub msgs_data_recv: Counter,
    /// Total size of all data messages sent
    pub msgs_data_sent_size: Counter,
    /// Total size of all data messages received
    pub msgs_data_recv_size: Counter,
    /// Total size of all control messages sent
    pub msgs_ctrl_sent_size: Counter,
    /// Total size of all control messages received
    pub msgs_ctrl_recv_size: Counter,
    /// Number of times we connected to a peer
    pub neighbor_up: Counter,
    /// Number of times we disconnected from a peer
    pub neighbor_down: Counter,
    /// Number of times the main actor loop ticked
    pub actor_tick_main: Counter,
    /// Number of times the actor ticked for a message received
    pub actor_tick_rx: Counter,
    /// Number of times the actor ticked for an endpoint event
    pub actor_tick_endpoint: Counter,
    /// Number of times the actor ticked for a dialer event
    pub actor_tick_dialer: Counter,
    /// Number of times the actor ticked for a successful dialer event
    pub actor_tick_dialer_success: Counter,
    /// Number of times the actor ticked for a failed dialer event
    pub actor_tick_dialer_failure: Counter,
    /// Number of times the actor ticked for an incoming event
    pub actor_tick_in_event_rx: Counter,
    /// Number of times the actor ticked for a timer event
    pub actor_tick_timers: Counter,
}
