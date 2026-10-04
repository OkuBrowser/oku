//! Labeled metrics example.
//!
//! Run with: `cargo run --example labels`

use std::sync::Arc;

use iroh_metrics::{
    Counter, EncodeLabelSet, Family, Histogram, MetricsGroup, MetricsSource, Registry,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, EncodeLabelSet)]
struct TransportLabels {
    protocol: &'static str,
    #[label(name = "direction")]
    dir: &'static str,
}

#[derive(Debug, MetricsGroup)]
#[metrics(default, name = "example")]
struct ExampleMetrics {
    /// Total requests
    requests: Counter,
    /// Requests by transport
    requests_by_transport: Family<TransportLabels, Counter>,
    /// Latency histogram, by transport
    #[default(Family::with_constructor(|| Histogram::new(vec![0.1, 0.5, 1.0, 5.0])))]
    latency: Family<TransportLabels, Histogram>,
}

fn main() {
    let metrics = Arc::new(ExampleMetrics::default());
    let mut registry = Registry::default();
    registry.register(metrics.clone());

    metrics.requests.inc();

    let send = TransportLabels {
        protocol: "quic",
        dir: "send",
    };
    let recv = TransportLabels {
        protocol: "quic",
        dir: "recv",
    };
    metrics.requests_by_transport.get_or_create(&send).inc();
    metrics.requests_by_transport.get_or_create(&recv).inc();
    metrics.latency.get_or_create(&send).observe(0.25);
    metrics.latency.get_or_create(&recv).observe(0.75);

    let output = registry.encode_openmetrics_to_string().unwrap();
    println!("{output}");
}
