//! Integration-style demo modelled on iroh's `socket::Metrics`.
//!
//! Compares the unlabeled style currently used in iroh (one counter per
//! transport kind) with the labeled style enabled by this PR. Run with:
//!
//!   cargo run --example iroh_socket_style

use std::sync::Arc;

use iroh_metrics::{
    Counter, EncodeLabelSet, EncodeLabelValue, Family, MetricsGroup, MetricsSource, Registry,
};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, EncodeLabelSet,
)]
struct TransportLabels {
    transport: Transport,
    direction: Direction,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EncodeLabelValue,
)]
enum Transport {
    Ipv4,
    Ipv6,
    Relay,
    Custom,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EncodeLabelValue,
)]
enum Direction {
    Send,
    Recv,
}

#[derive(Debug, Default, MetricsGroup, Serialize, Deserialize)]
#[metrics(name = "socket")]
#[non_exhaustive]
struct Metrics {
    /// Total bytes routed.
    bytes: Family<TransportLabels, Counter>,
    /// Connections opened (handshake completed).
    num_conns_opened: Counter,
    /// Connections closed.
    num_conns_closed: Counter,
}

fn main() {
    let metrics = Arc::new(Metrics::default());
    let mut registry = Registry::default();
    registry.register(metrics.clone());

    metrics.num_conns_opened.inc();
    metrics
        .bytes
        .get_or_create(&TransportLabels {
            transport: Transport::Ipv4,
            direction: Direction::Send,
        })
        .inc_by(1024);
    metrics
        .bytes
        .get_or_create(&TransportLabels {
            transport: Transport::Relay,
            direction: Direction::Recv,
        })
        .inc_by(2048);

    println!("{}", registry.encode_openmetrics_to_string().unwrap());
}
