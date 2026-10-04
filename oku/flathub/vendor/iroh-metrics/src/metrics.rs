//! This module defines the individual metric types.
//!
//! If the `metrics` feature is enabled, they contain metric types based on atomics
//! which can be modified without needing mutable access.
//!
//! If the `metrics` feature is disabled, all operations defined on these types are noops,
//! and the structs don't collect actual data.

use std::any::Any;

#[cfg(feature = "metrics")]
use portable_atomic::{AtomicI64, AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

/// The types of metrics supported by this crate.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MetricType {
    /// A [`Counter`].
    Counter,
    /// A [`Gauge`].
    Gauge,
    /// A [`Histogram`].
    Histogram,
}

impl MetricType {
    /// Returns the given metric type's str representation.
    pub fn as_str(&self) -> &str {
        match self {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
        }
    }
}

/// The value of an individual metric item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MetricValue {
    /// A [`Counter`] value.
    Counter(u64),
    /// A [`Gauge`] value.
    Gauge(i64),
    /// A [`Histogram`] value with buckets, sum, and count.
    Histogram {
        /// Bucket upper bounds and cumulative counts
        buckets: Vec<(f64, u64)>,
        /// Sum of all observed values
        sum: f64,
        /// Total count of observations
        count: u64,
    },
}

impl MetricValue {
    /// Returns the value as [`f32`].
    pub fn to_f32(&self) -> f32 {
        match self {
            MetricValue::Counter(value) => *value as f32,
            MetricValue::Gauge(value) => *value as f32,
            MetricValue::Histogram { count, .. } => *count as f32,
        }
    }

    /// Returns the [`MetricType`] for this metric value.
    pub fn r#type(&self) -> MetricType {
        match self {
            MetricValue::Counter(_) => MetricType::Counter,
            MetricValue::Gauge(_) => MetricType::Gauge,
            MetricValue::Histogram { .. } => MetricType::Histogram,
        }
    }
}

impl Metric for MetricValue {
    fn r#type(&self) -> MetricType {
        self.r#type()
    }

    fn value(&self) -> MetricValue {
        self.clone()
    }

    fn set_value(&self, _value: MetricValue) {
        // MetricValue is immutable, this is a no-op
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trait for metric items.
pub trait Metric: std::fmt::Debug + Send + Sync {
    /// Returns the type of this metric.
    fn r#type(&self) -> MetricType;

    /// Returns the current value of this metric.
    fn value(&self) -> MetricValue;

    /// Sets the value of this metric from a [`MetricValue`].
    ///
    /// Used by the [`Family`](crate::Family) deserialize impl: a family
    /// stores generic `Arc<M>` entries and round-trips through serde as
    /// `Vec<(L, MetricValue)>`, so on deserialize each `M` is constructed via
    /// `Default` and then populated through this setter. If the variant
    /// doesn't match this metric's type the implementation must silently
    /// ignore it.
    fn set_value(&self, value: MetricValue);

    /// Casts this metric to [`Any`] for downcasting to concrete types.
    fn as_any(&self) -> &dyn Any;
}

/// OpenMetrics [`Counter`] to measure discrete events.
///
/// Single monotonically increasing value metric.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Counter {
    /// The counter value.
    #[cfg(feature = "metrics")]
    pub(crate) value: AtomicU64,
}

impl Metric for Counter {
    fn value(&self) -> MetricValue {
        MetricValue::Counter(self.get())
    }

    fn r#type(&self) -> MetricType {
        MetricType::Counter
    }

    fn set_value(&self, value: MetricValue) {
        if let MetricValue::Counter(v) = value {
            self.set(v);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Counter {
    /// Constructs a new counter, based on the given `help`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increases the [`Counter`] by 1, returning the previous value.
    pub fn inc(&self) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_add(1, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }

    /// Increases the [`Counter`] by `u64`, returning the previous value.
    pub fn inc_by(&self, v: u64) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_add(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Sets the [`Counter`] value, returning the previous value.
    ///
    /// Warning: this is not default behavior for a counter that should always be monotonically increasing.
    pub fn set(&self, v: u64) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.value.swap(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Returns the current value of the [`Counter`].
    pub fn get(&self) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.value.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }
}

/// OpenMetrics [`Gauge`].
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Gauge {
    /// The gauge value.
    #[cfg(feature = "metrics")]
    pub(crate) value: AtomicI64,
}

/// OpenMetrics [`Histogram`] to track distributions of values.
#[derive(Debug, Serialize, Deserialize)]
pub struct Histogram {
    /// Bucket upper bounds.
    #[cfg(feature = "metrics")]
    pub(crate) buckets: Vec<f64>,
    /// Individual counts for each bucket.
    #[cfg(feature = "metrics")]
    pub(crate) counts: Vec<AtomicU64>,
    /// Sum of all observed values (stored as bits for atomic operations).
    #[cfg(feature = "metrics")]
    pub(crate) sum: AtomicU64,
    /// Total count of observations.
    #[cfg(feature = "metrics")]
    pub(crate) count: AtomicU64,
}

impl Histogram {
    /// Constructs a new histogram with the given bucket boundaries.
    ///
    /// The `buckets` parameter defines the upper bounds for each bucket.
    /// Buckets should be in ascending order. An infinity bucket is automatically
    /// added if not present to ensure all observations are captured.
    #[cfg_attr(not(feature = "metrics"), allow(unused_mut))]
    pub fn new(mut buckets: Vec<f64>) -> Self {
        #[cfg(feature = "metrics")]
        {
            // Ensure there's an infinity bucket to catch all values
            if buckets.is_empty() || !buckets.last().unwrap().is_infinite() {
                buckets.push(f64::INFINITY);
            }

            let counts = buckets.iter().map(|_| AtomicU64::new(0)).collect();
            Self {
                buckets,
                counts,
                sum: AtomicU64::new(0.0_f64.to_bits()),
                count: AtomicU64::new(0),
            }
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = buckets;
            Self {}
        }
    }

    /// Records a value in the histogram.
    pub fn observe(&self, value: f64) {
        #[cfg(feature = "metrics")]
        {
            self.count.fetch_add(1, Ordering::Relaxed);

            self.sum
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                    let current_sum = f64::from_bits(current);
                    Some((current_sum + value).to_bits())
                })
                .ok();

            for (i, &upper_bound) in self.buckets.iter().enumerate() {
                if value <= upper_bound {
                    self.counts[i].fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = value;
        }
    }

    /// Returns the total count of observations.
    pub fn count(&self) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.count.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }

    /// Returns the sum of all observed values.
    pub fn sum(&self) -> f64 {
        #[cfg(feature = "metrics")]
        {
            f64::from_bits(self.sum.load(Ordering::Relaxed))
        }
        #[cfg(not(feature = "metrics"))]
        0.0
    }

    /// Returns the bucket counts as a vector of (upper_bound, cumulative_count) pairs.
    ///
    /// The counts are cumulative, meaning each bucket contains the count of all
    /// observations less than or equal to its upper bound.
    pub fn buckets(&self) -> Vec<(f64, u64)> {
        #[cfg(feature = "metrics")]
        {
            let mut cumulative = 0u64;
            self.buckets
                .iter()
                .zip(self.counts.iter())
                .map(|(&bound, count)| {
                    cumulative += count.load(Ordering::Relaxed);
                    (bound, cumulative)
                })
                .collect()
        }
        #[cfg(not(feature = "metrics"))]
        Vec::new()
    }

    /// Calculates the approximate percentile value.
    ///
    /// Returns the bucket upper bound where the percentile falls.
    /// For example, `percentile(0.99)` returns the p99 value.
    pub fn percentile(&self, p: f64) -> f64 {
        #[cfg(feature = "metrics")]
        {
            let total = self.count.load(Ordering::Relaxed);
            if total == 0 {
                return 0.0;
            }

            let target = (total as f64 * p) as u64;
            let mut cumulative = 0u64;

            for (i, count) in self.counts.iter().enumerate() {
                cumulative += count.load(Ordering::Relaxed);
                if cumulative >= target {
                    return self.buckets[i];
                }
            }

            self.buckets.last().copied().unwrap_or(0.0)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = p;
            0.0
        }
    }
}

impl Metric for Histogram {
    fn r#type(&self) -> MetricType {
        MetricType::Histogram
    }

    fn value(&self) -> MetricValue {
        MetricValue::Histogram {
            buckets: self.buckets(),
            sum: self.sum(),
            count: self.count(),
        }
    }

    fn set_value(&self, value: MetricValue) {
        #[cfg(feature = "metrics")]
        if let MetricValue::Histogram {
            buckets,
            sum,
            count,
        } = value
        {
            // Only set if bucket count matches
            if buckets.len() != self.buckets.len() {
                return;
            }

            // Validate cumulative counts are monotonically increasing
            for i in 1..buckets.len() {
                if buckets[i].1 < buckets[i - 1].1 {
                    return;
                }
            }

            self.count.store(count, Ordering::Relaxed);
            self.sum.store(sum.to_bits(), Ordering::Relaxed);

            // Convert cumulative counts back to individual bucket counts
            for (i, (_, cumulative)) in buckets.iter().enumerate() {
                let prev = if i > 0 { buckets[i - 1].1 } else { 0 };
                self.counts[i].store(cumulative - prev, Ordering::Relaxed);
            }
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = value;
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Metric for Gauge {
    fn r#type(&self) -> MetricType {
        MetricType::Gauge
    }

    fn value(&self) -> MetricValue {
        MetricValue::Gauge(self.get())
    }

    fn set_value(&self, value: MetricValue) {
        if let MetricValue::Gauge(v) = value {
            self.set(v);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Gauge {
    /// Constructs a new gauge, based on the given `help`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increases the [`Gauge`] by 1, returning the previous value.
    pub fn inc(&self) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_add(1, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }

    /// Increases the [`Gauge`] by `i64`, returning the previous value.
    pub fn inc_by(&self, v: i64) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_add(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Decreases the [`Gauge`] by 1, returning the previous value.
    pub fn dec(&self) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_sub(1, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }

    /// Decreases the [`Gauge`] by `i64`, returning the previous value.
    pub fn dec_by(&self, v: i64) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_sub(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Sets the [`Gauge`] to `v`, returning the previous value.
    pub fn set(&self, v: i64) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.swap(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Returns the [`Gauge`] value.
    pub fn get(&self) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }
}
