//! Metric families with labels.
//!
//! A [`Family`] is a collection of metrics indexed by label sets. Labels
//! should be low cardinality: each unique combination becomes a separate
//! timeseries on the backend, and the internal map grows without bound.

use std::{
    borrow::Cow,
    fmt::{self, Write},
    sync::Arc,
};
#[cfg(feature = "metrics")]
use std::{
    collections::HashMap,
    panic::AssertUnwindSafe,
    sync::{OnceLock, RwLock},
};

use portable_atomic::AtomicU64;
#[cfg(feature = "metrics")]
use portable_atomic::Ordering;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "metrics")]
use crate::MetricValue;
#[cfg(feature = "metrics")]
use crate::encoding::{ItemSchema, encode_help_text, encode_metric_value, encode_prefix_name};
use crate::{
    Metric,
    encoding::{Schema, Values},
    labels::EncodeLabelSet,
};

/// Type-erased encoding interface for a [`Family`].
///
/// `Family<L, M>` is generic in both label and metric type. This trait
/// collapses those generics so a metrics group containing multiple families
/// can iterate them as `&dyn FamilyEncoder`.
pub trait FamilyEncoder: Send + Sync + 'static {
    /// Encodes to OpenMetrics text format.
    fn encode_openmetrics(
        &self,
        writer: &mut dyn Write,
        name: &str,
        help: &str,
        prefixes: &[&str],
        registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
    ) -> fmt::Result;

    /// Encodes the binary export of this family.
    ///
    /// Schema items (when `schema` is `Some`) and values are pushed under a
    /// single read lock per family, so the two slices stay aligned even when
    /// other threads are inserting new label combinations concurrently.
    fn encode_schema(
        &self,
        schema: Option<&mut Schema>,
        values: &mut Values,
        name: &str,
        help: &str,
        prefixes: &[&str],
        registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
    );

    /// Returns true if the family has no entries.
    fn is_empty(&self) -> bool;

    /// Wires this family up to a registry's schema-version counter.
    ///
    /// Called by the registry when the parent metrics group is registered, so
    /// that adding a new label combination (a new entry to the family) bumps
    /// the version and re-publishes the schema on the next binary export.
    fn attach_schema_version(&self, version: Arc<AtomicU64>);
}

/// A family metric item for iteration.
#[derive(Clone, Copy)]
pub struct FamilyItem<'a> {
    pub(crate) name: &'static str,
    pub(crate) help: &'static str,
    pub(crate) family: &'a dyn FamilyEncoder,
}

impl fmt::Debug for FamilyItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FamilyItem")
            .field("name", &self.name)
            .field("help", &self.help)
            .finish_non_exhaustive()
    }
}

impl<'a> FamilyItem<'a> {
    /// Creates a new family item.
    pub fn new(name: &'static str, help: &'static str, family: &'a dyn FamilyEncoder) -> Self {
        Self { name, help, family }
    }

    /// Returns the name of this family.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the help text of this family.
    pub fn help(&self) -> &'static str {
        self.help
    }

    /// Returns true if the family has no entries.
    pub fn is_empty(&self) -> bool {
        self.family.is_empty()
    }

    /// Encodes to OpenMetrics text format.
    pub fn encode_openmetrics(
        &self,
        writer: &mut dyn fmt::Write,
        prefixes: &[&str],
        registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
    ) -> fmt::Result {
        self.family
            .encode_openmetrics(writer, self.name, self.help, prefixes, registry_labels)
    }

    /// Encodes the binary export of this family (schema and/or values).
    pub fn encode_schema(
        &self,
        schema: Option<&mut Schema>,
        values: &mut Values,
        prefixes: &[&str],
        registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
    ) {
        self.family.encode_schema(
            schema,
            values,
            self.name,
            self.help,
            prefixes,
            registry_labels,
        );
    }

    /// Attaches a schema-version counter to the underlying family.
    ///
    /// Used by [`Registry::register`](crate::Registry::register) to wire
    /// every family in a metrics group up to the registry's version counter
    /// so that adding a new label combination invalidates the cached schema.
    pub fn attach_schema_version(&self, version: Arc<AtomicU64>) {
        self.family.attach_schema_version(version);
    }
}

/// The closure a [`Family`] calls to create the metric for a new label set.
///
/// We wrap the Arc'd closure in [`AssertUnwindSafe`] here. A plain `Arc<dyn Fn>`
/// is not `RefUnwindSafe`, which would make `Family` and every metrics group
/// containing it become unwind-unsafe. The proper fix would be to add the bound to
/// [`Family::with_constructor`], but that would break the public API.
///
/// Reasoning why the [`AssertUnwindSafe`] is acceptable:
///
/// - `Family` never runs the closure after a panic inside it. The closure is only
///   invoked under the `inner` write lock, so such a panic poisons the lock and
///   every later access to the map panics on the poison check instead.
/// - A panic elsewhere that corrupts the closure's captured state is the closure
///   author's concern, as it would be for any other shared `Sync` value they hold.
/// - `RefUnwindSafe` is a safe marker trait with no soundness contract. Per the
///   std docs it is advisory only: it warns `catch_unwind` callers that they may
///   observe broken logical invariants, but a panic cannot cause memory unsafety.
#[cfg(feature = "metrics")]
type Constructor<M> = AssertUnwindSafe<Arc<dyn Fn() -> M + Send + Sync>>;

/// One entry in a [`Family`]: the metric plus the rendered label strings
/// computed once at insert time.
#[cfg(feature = "metrics")]
struct FamilyEntry<M> {
    metric: Arc<M>,
    /// Cached `(name, value-as-string)` pairs, rendered from the
    /// `LabelValue` enum once at insert time so the scrape hot path doesn't
    /// reallocate per-series. The strings are RAW — OpenMetrics escaping is
    /// NOT applied here; it happens at write time via `EncodeLabelTo` for the
    /// text path and via the same path on the decoder side for the binary
    /// export. Do not emit these directly to a wire format that requires
    /// escaping.
    encoded_labels: Vec<(&'static str, String)>,
}

/// A family of metrics indexed by labels.
///
/// Thread-safe: multiple threads can look up or create metrics concurrently.
/// Each metric is reference-counted so it can be used independently after lookup.
#[cfg(feature = "metrics")]
pub struct Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric,
{
    inner: Arc<RwLock<HashMap<L, FamilyEntry<M>>>>,
    constructor: Constructor<M>,
    // Set once when the parent group is registered. Bumped on each new label
    // combo so the binary encoder re-publishes the schema.
    //
    // Why `Arc<OnceLock<Arc<AtomicU64>>>` and not just `Arc<AtomicU64>`?
    // - The counter is owned by the registry; a `Family` constructed inside a
    //   user struct does not know about it until `Registry::register` walks
    //   the group's families and attaches it. `OnceLock` lets us bind it
    //   exactly once after construction without introducing a `Mutex`.
    // - The outer `Arc` keeps that "attached" state shared between any
    //   `Family::clone` instances (the `RwLock<HashMap>` is shared too, so
    //   inserts on a clone must bump the same counter).
    schema_version: Arc<OnceLock<Arc<AtomicU64>>>,
}

#[cfg(feature = "metrics")]
impl<L, M> Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric + Default + 'static,
{
    /// Creates a new family using `M::default()` for new metrics.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            constructor: AssertUnwindSafe(Arc::new(M::default)),
            schema_version: Arc::new(OnceLock::new()),
        }
    }
}

#[cfg(feature = "metrics")]
impl<L, M> Default for Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric + Default + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "metrics")]
impl<L, M> Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric,
{
    /// Creates a new family with a custom constructor (useful for Histogram buckets).
    pub fn with_constructor<F: Fn() -> M + Send + Sync + 'static>(constructor: F) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            constructor: AssertUnwindSafe(Arc::new(constructor)),
            schema_version: Arc::new(OnceLock::new()),
        }
    }

    /// Gets or creates a metric for the given labels.
    ///
    /// Each call performs a `HashMap` lookup under a read lock. For hot paths
    /// where the label set is stable, hold on to the returned `Arc<M>` and
    /// reuse it instead of calling `get_or_create` on every record.
    pub fn get_or_create(&self, labels: &L) -> Arc<M> {
        if let Some(entry) = self.inner.read().expect("poisoned").get(labels) {
            return Arc::clone(&entry.metric);
        }

        let mut guard = self.inner.write().expect("poisoned");
        if let Some(entry) = guard.get(labels) {
            return Arc::clone(&entry.metric);
        }

        let metric = Arc::new((self.constructor)());
        let encoded_labels = labels
            .encode_label_pairs()
            .into_iter()
            .map(|(k, v)| (k, v.as_str().into_owned()))
            .collect();
        guard.insert(
            labels.clone(),
            FamilyEntry {
                metric: Arc::clone(&metric),
                encoded_labels,
            },
        );
        if let Some(v) = self.schema_version.get() {
            v.fetch_add(1, Ordering::Relaxed);
        }
        metric
    }

    /// Looks up an existing metric without creating one. Read-only fast path.
    pub fn get(&self, labels: &L) -> Option<Arc<M>> {
        self.inner
            .read()
            .expect("poisoned")
            .get(labels)
            .map(|entry| Arc::clone(&entry.metric))
    }

    /// Removes the metric for the given labels.
    pub fn remove(&self, labels: &L) -> Option<Arc<M>> {
        self.inner
            .write()
            .expect("poisoned")
            .remove(labels)
            .map(|entry| entry.metric)
    }

    /// Removes all metrics.
    pub fn clear(&self) {
        self.inner.write().expect("poisoned").clear();
    }

    /// Returns the number of label combinations tracked.
    pub fn len(&self) -> usize {
        self.inner.read().expect("poisoned").len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.inner.read().expect("poisoned").is_empty()
    }
}

/// A family of metrics indexed by labels (no-op when metrics disabled).
#[cfg(not(feature = "metrics"))]
pub struct Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric,
{
    /// Shared no-op / placeholder metric returned by all `get_or_create` calls when metrics are disabled.
    default_metric: Arc<M>,
    _labels: std::marker::PhantomData<L>,
}

#[cfg(not(feature = "metrics"))]
impl<L, M> Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric + Default + 'static,
{
    /// Creates a new family using `M::default()` for new metrics.
    pub fn new() -> Self {
        Self {
            default_metric: Arc::new(M::default()),
            _labels: std::marker::PhantomData,
        }
    }
}

#[cfg(not(feature = "metrics"))]
impl<L, M> Default for Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric + Default + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "metrics"))]
impl<L, M> Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric,
{
    /// Creates a new family with a custom constructor (useful for Histogram buckets).
    pub fn with_constructor<F: Fn() -> M + Send + Sync + 'static>(constructor: F) -> Self {
        Self {
            default_metric: Arc::new(constructor()),
            _labels: std::marker::PhantomData,
        }
    }

    /// Gets or creates a metric for the given labels (returns default metric).
    pub fn get_or_create(&self, _labels: &L) -> Arc<M> {
        Arc::clone(&self.default_metric)
    }

    /// Looks up an existing metric without creating one (always returns `None`).
    pub fn get(&self, _labels: &L) -> Option<Arc<M>> {
        None
    }

    /// Removes the metric for the given labels (no-op).
    pub fn remove(&self, _labels: &L) -> Option<Arc<M>> {
        None
    }

    /// Removes all metrics (no-op).
    pub fn clear(&self) {}

    /// Returns the number of label combinations tracked.
    pub fn len(&self) -> usize {
        0
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        true
    }
}

// ============================================================================
// Trait impls: metrics ENABLED
// ============================================================================

#[cfg(feature = "metrics")]
impl<L, M> Clone for Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            constructor: AssertUnwindSafe(Arc::clone(&self.constructor.0)),
            schema_version: Arc::clone(&self.schema_version),
        }
    }
}

#[cfg(feature = "metrics")]
impl<L, M> FamilyEncoder for Family<L, M>
where
    L: EncodeLabelSet + Ord,
    M: Metric + 'static,
{
    fn encode_openmetrics(
        &self,
        writer: &mut dyn Write,
        name: &str,
        help: &str,
        prefixes: &[&str],
        registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
    ) -> fmt::Result {
        let guard = self.inner.read().expect("poisoned");
        if guard.is_empty() {
            return Ok(());
        }

        let mut entries: Vec<_> = guard.iter().collect();
        entries.sort_by_key(|(a, _)| *a);

        let metric_type = entries[0].1.metric.r#type();

        writer.write_str("# HELP ")?;
        encode_prefix_name(writer, prefixes, name)?;
        writer.write_str(" ")?;
        encode_help_text(writer, help)?;

        writer.write_str("# TYPE ")?;
        encode_prefix_name(writer, prefixes, name)?;
        writeln!(writer, " {}", metric_type.as_str())?;

        for (_labels, entry) in entries {
            encode_metric_value(
                writer,
                name,
                prefixes,
                registry_labels,
                &entry.encoded_labels,
                &entry.metric.value(),
            )?;
        }
        Ok(())
    }

    fn encode_schema(
        &self,
        mut schema: Option<&mut Schema>,
        values: &mut Values,
        name: &str,
        help: &str,
        prefixes: &[&str],
        registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
    ) {
        // Hold the read lock for the whole pass so the schema items and the
        // values stay aligned even when other threads call `get_or_create`.
        let guard = self.inner.read().expect("poisoned");
        let mut entries: Vec<_> = guard.iter().collect();
        entries.sort_by_key(|(a, _)| *a);

        for (_labels, entry) in entries {
            if let Some(schema) = schema.as_deref_mut() {
                let all_labels = registry_labels
                    .iter()
                    .map(|(k, v)| (k.as_ref(), v.as_ref()))
                    .chain(entry.encoded_labels.iter().map(|(k, v)| (*k, v.as_str())));
                schema.push(
                    ItemSchema::from_label_iter(name, prefixes, all_labels, entry.metric.r#type()),
                    help,
                );
            }
            values.items.push(entry.metric.value());
        }
    }

    fn is_empty(&self) -> bool {
        Family::is_empty(self)
    }

    fn attach_schema_version(&self, version: Arc<AtomicU64>) {
        let _ = self.schema_version.set(version);
    }
}

#[cfg(feature = "metrics")]
impl<L, M> fmt::Debug for Family<L, M>
where
    L: EncodeLabelSet + fmt::Debug,
    M: Metric,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let guard = self.inner.read().expect("poisoned");
        f.debug_struct("Family")
            .field("len", &guard.len())
            .field("labels", &guard.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(feature = "metrics")]
impl<L, M> Serialize for Family<L, M>
where
    L: EncodeLabelSet + Serialize + Ord,
    M: Metric,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;

        let guard = self.inner.read().expect("poisoned");
        let mut entries: Vec<_> = guard.iter().collect();
        entries.sort_by_key(|(a, _)| *a);

        let mut seq = serializer.serialize_seq(Some(entries.len()))?;
        for (labels, entry) in entries {
            seq.serialize_element(&(labels, entry.metric.value()))?;
        }
        seq.end()
    }
}

#[cfg(feature = "metrics")]
impl<'de, L, M> Deserialize<'de> for Family<L, M>
where
    L: EncodeLabelSet + Deserialize<'de>,
    M: Metric + Default + 'static,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries: Vec<(L, MetricValue)> = Vec::deserialize(deserializer)?;
        let family = Family::new();
        {
            let mut guard = family.inner.write().expect("poisoned");
            for (labels, value) in entries {
                let metric = Arc::new(M::default());
                metric.set_value(value);
                let encoded_labels = labels
                    .encode_label_pairs()
                    .into_iter()
                    .map(|(k, v)| (k, v.as_str().into_owned()))
                    .collect();
                guard.insert(
                    labels,
                    FamilyEntry {
                        metric,
                        encoded_labels,
                    },
                );
            }
        }
        Ok(family)
    }
}

#[cfg(not(feature = "metrics"))]
impl<L, M> Clone for Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric,
{
    fn clone(&self) -> Self {
        Self {
            default_metric: Arc::clone(&self.default_metric),
            _labels: std::marker::PhantomData,
        }
    }
}

#[cfg(not(feature = "metrics"))]
impl<L, M> FamilyEncoder for Family<L, M>
where
    L: EncodeLabelSet + Ord,
    M: Metric + 'static,
{
    fn encode_openmetrics(
        &self,
        _writer: &mut dyn Write,
        _name: &str,
        _help: &str,
        _prefixes: &[&str],
        _registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
    ) -> fmt::Result {
        Ok(())
    }

    fn encode_schema(
        &self,
        _schema: Option<&mut Schema>,
        _values: &mut Values,
        _name: &str,
        _help: &str,
        _prefixes: &[&str],
        _registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
    ) {
    }

    fn is_empty(&self) -> bool {
        true
    }

    fn attach_schema_version(&self, _version: Arc<AtomicU64>) {}
}

#[cfg(not(feature = "metrics"))]
impl<L, M> fmt::Debug for Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Family").field("disabled", &true).finish()
    }
}

#[cfg(not(feature = "metrics"))]
impl<L, M> Serialize for Family<L, M>
where
    L: EncodeLabelSet,
    M: Metric,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        serializer.serialize_seq(Some(0))?.end()
    }
}

#[cfg(not(feature = "metrics"))]
impl<'de, L, M> Deserialize<'de> for Family<L, M>
where
    L: EncodeLabelSet + Deserialize<'de>,
    M: Metric + Default + 'static,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Parse and discard the entries — without metrics we don't track them,
        // but we must consume the wire format so that postcard et al. can
        // resume reading the next field. `IgnoredAny` is not enough because
        // postcard's deserializer doesn't implement `deserialize_ignored_any`.
        let _: Vec<(L, crate::MetricValue)> = Vec::deserialize(deserializer)?;
        Ok(Family::new())
    }
}

#[cfg(all(test, not(feature = "metrics")))]
mod tests_no_metrics {
    use std::borrow::Cow;

    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{Counter, LabelPair};

    #[derive(Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
    struct TestLabels {
        method: String,
    }

    impl EncodeLabelSet for TestLabels {
        fn encode_label_pairs(&self) -> Vec<LabelPair<'_>> {
            vec![(
                "method",
                crate::LabelValue::Str(Cow::Borrowed(&self.method)),
            )]
        }
    }

    #[test]
    fn family_serde_roundtrip_no_metrics_feature() {
        // Serialize as empty (no entries are tracked when metrics are off),
        // deserialize back, and verify no panic / decode succeeds.
        let family: Family<TestLabels, Counter> = Family::new();
        let bytes = postcard::to_stdvec(&family).unwrap();
        let _decoded: Family<TestLabels, Counter> = postcard::from_bytes(&bytes).unwrap();
    }
}

#[cfg(all(test, feature = "metrics"))]
mod tests {
    use std::borrow::Cow;

    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{Counter, Histogram, LabelPair, NoLabels};

    #[derive(Clone, Hash, PartialEq, Eq, Debug, PartialOrd, Ord, Serialize, Deserialize)]
    struct TestLabels {
        method: String,
        status: u16,
    }

    impl EncodeLabelSet for TestLabels {
        fn encode_label_pairs(&self) -> Vec<LabelPair<'_>> {
            vec![
                (
                    "method",
                    crate::LabelValue::Str(Cow::Borrowed(&self.method)),
                ),
                ("status", crate::LabelValue::Uint(self.status as u64)),
            ]
        }
    }

    fn labels(method: &str, status: u16) -> TestLabels {
        TestLabels {
            method: method.into(),
            status,
        }
    }

    #[test]
    fn test_family_operations() {
        // get_or_create returns same metric for same labels
        let family: Family<TestLabels, Counter> = Family::new();
        let c1 = family.get_or_create(&labels("GET", 200));
        c1.inc();
        let c2 = family.get_or_create(&labels("GET", 200));
        c2.inc();
        assert_eq!(c1.get(), 2);

        // different labels get different metrics
        family.get_or_create(&labels("GET", 404)).inc_by(5);
        assert_eq!(family.get_or_create(&labels("GET", 404)).get(), 5);
        assert_eq!(family.len(), 2);

        // remove
        let removed = family.remove(&labels("GET", 200));
        assert_eq!(removed.unwrap().get(), 2);
        assert_eq!(family.len(), 1);

        // clear
        family.get_or_create(&labels("POST", 200)).inc();
        family.clear();
        assert!(family.is_empty());

        // with_constructor for custom metrics
        let hist_family: Family<NoLabels, Histogram> =
            Family::with_constructor(|| Histogram::new(vec![1.0, 5.0, 10.0]));
        hist_family.get_or_create(&NoLabels).observe(2.5);
        hist_family.get_or_create(&NoLabels).observe(7.5);
        assert_eq!(hist_family.get_or_create(&NoLabels).count(), 2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let family: Family<TestLabels, Counter> = Family::new();
        family.get_or_create(&labels("GET", 200)).inc_by(10);
        family.get_or_create(&labels("POST", 201)).inc_by(5);

        let bytes = postcard::to_stdvec(&family).unwrap();
        let decoded: Family<TestLabels, Counter> = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.get_or_create(&labels("GET", 200)).get(), 10);
        assert_eq!(decoded.get_or_create(&labels("POST", 201)).get(), 5);
    }

    #[test]
    fn test_encoding() {
        let family: Family<TestLabels, Counter> = Family::new();
        family.get_or_create(&labels("GET", 200)).inc_by(10);
        family.get_or_create(&labels("POST", 201)).inc_by(5);

        // OpenMetrics without registry labels
        let mut out = String::new();
        FamilyEncoder::encode_openmetrics(
            &family,
            &mut out,
            "requests",
            "HTTP requests",
            &["http"],
            &[],
        )
        .unwrap();
        assert!(out.contains("# HELP http_requests HTTP requests."));
        assert!(out.contains("# TYPE http_requests counter"));
        assert!(out.contains(r#"http_requests_total{method="GET",status="200"} 10"#));
        assert!(out.contains(r#"http_requests_total{method="POST",status="201"} 5"#));

        // OpenMetrics with registry labels
        let mut out = String::new();
        let registry_labels = [(Cow::Borrowed("service"), Cow::Borrowed("api"))];
        FamilyEncoder::encode_openmetrics(
            &family,
            &mut out,
            "requests",
            "HTTP requests",
            &["http"],
            &registry_labels,
        )
        .unwrap();
        assert!(out.contains(r#"http_requests_total{service="api",method="GET",status="200"} 10"#));

        // Binary schema + values
        let mut schema = Schema::default();
        let mut values = Values::default();
        FamilyEncoder::encode_schema(
            &family,
            Some(&mut schema),
            &mut values,
            "requests",
            "HTTP requests",
            &["http"],
            &[],
        );
        assert_eq!(schema.items.len(), 2);
        assert_eq!(values.items.len(), 2);
    }

    #[test]
    fn test_openmetrics_escapes_specials() {
        // Label values with `"`, `\`, `\n` and HELP text with `\`, `\n` must
        // round-trip through the OpenMetrics text format without corrupting
        // the syntax.
        let family: Family<TestLabels, Counter> = Family::new();
        family
            .get_or_create(&TestLabels {
                method: "a\"b\\c\nd".into(),
                status: 200,
            })
            .inc();

        let mut out = String::new();
        FamilyEncoder::encode_openmetrics(
            &family,
            &mut out,
            "requests",
            "Quote: \" backslash: \\ newline:\nend",
            &["http"],
            &[],
        )
        .unwrap();

        // Label value escaped.
        assert!(
            out.contains(r#"method="a\"b\\c\nd""#),
            "expected escaped label value, got: {out}",
        );
        // Help text escaped (backslash + newline). Quote left as-is in HELP.
        assert!(
            out.contains(r"backslash: \\ newline:\nend"),
            "expected escaped help, got: {out}",
        );
        assert!(
            !out.contains("newline:\nend"),
            "raw newline must be escaped: {out}",
        );
    }

    #[test]
    #[cfg(feature = "postcard")]
    fn export_after_mid_walk_insert_keeps_schema_aligned() {
        // Regression for the version-tracking race in `Encoder::export`:
        // advancing `last_schema_version` to the *post-walk* version
        // can outrun the schema we actually built. The next round then
        // skips publishing a schema while the values list has grown,
        // leaving the decoder one entry behind for every later item.
        //
        // The race is reproduced deterministically with a custom
        // `FamilyEncoder` wrapper that fires a side effect right before
        // walking its own entries — equivalent to another thread calling
        // `get_or_create` between two families' walks within a single
        // export pass.
        use std::sync::Mutex;

        use crate::{
            FamilyItem, MetricItem, MetricsGroup, MetricsSource, Registry,
            encoding::{Decoder, Encoder, Schema, Values},
            iterable::Iterable,
        };

        struct TriggerFamily {
            inner: Family<TestLabels, Counter>,
            before_walk: Mutex<Option<Box<dyn FnOnce() + Send>>>,
        }

        impl fmt::Debug for TriggerFamily {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("TriggerFamily").finish_non_exhaustive()
            }
        }

        impl FamilyEncoder for TriggerFamily {
            fn encode_openmetrics(
                &self,
                writer: &mut dyn Write,
                name: &str,
                help: &str,
                prefixes: &[&str],
                registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
            ) -> fmt::Result {
                self.inner
                    .encode_openmetrics(writer, name, help, prefixes, registry_labels)
            }
            fn encode_schema(
                &self,
                schema: Option<&mut Schema>,
                values: &mut Values,
                name: &str,
                help: &str,
                prefixes: &[&str],
                registry_labels: &[(Cow<'_, str>, Cow<'_, str>)],
            ) {
                if let Some(f) = self.before_walk.lock().unwrap().take() {
                    f();
                }
                self.inner
                    .encode_schema(schema, values, name, help, prefixes, registry_labels);
            }
            fn is_empty(&self) -> bool {
                self.inner.is_empty()
            }
            fn attach_schema_version(&self, version: Arc<AtomicU64>) {
                self.inner.attach_schema_version(version);
            }
        }

        #[derive(Debug)]
        struct M {
            a: Family<TestLabels, Counter>,
            b: TriggerFamily,
        }

        impl Iterable for M {
            fn metric_field_count(&self) -> usize {
                0
            }
            fn metric_field_ref(&self, _: usize) -> Option<MetricItem<'_>> {
                None
            }
            fn family_field_count(&self) -> usize {
                2
            }
            fn family_field_ref(&self, n: usize) -> Option<FamilyItem<'_>> {
                match n {
                    0 => Some(FamilyItem::new("a", "a help", &self.a)),
                    1 => Some(FamilyItem::new("b", "b help", &self.b)),
                    _ => None,
                }
            }
        }
        impl MetricsGroup for M {
            fn name(&self) -> &'static str {
                "race"
            }
        }

        let metrics = Arc::new(M {
            a: Family::new(),
            b: TriggerFamily {
                inner: Family::new(),
                before_walk: Mutex::new(None),
            },
        });
        metrics.a.get_or_create(&labels("GET", 200)).inc();
        metrics.b.inner.get_or_create(&labels("GET", 200)).inc();

        let mut registry = Registry::default();
        registry.register(metrics.clone());
        let registry = Arc::new(std::sync::RwLock::new(registry));

        // Round 1: initial schema + values published.
        let mut encoder = Encoder::new(registry.clone());
        let mut decoder = Decoder::default();
        decoder
            .import_bytes(&encoder.export_bytes().unwrap())
            .unwrap();
        assert_eq!(
            decoder.encode_openmetrics_to_string().unwrap(),
            registry.encode_openmetrics_to_string().unwrap(),
        );

        // Round 2: arm the trigger so a new entry lands in family A
        // *between* A's and B's walks. The schema captured this round
        // does NOT include the new entry (A was already walked) but
        // `schema_version` is bumped, so this round's update still
        // attaches the (pre-insert) schema. That is fine on its own.
        let metrics_clone = metrics.clone();
        *metrics.b.before_walk.lock().unwrap() = Some(Box::new(move || {
            metrics_clone.a.get_or_create(&labels("POST", 201)).inc();
        }));
        decoder
            .import_bytes(&encoder.export_bytes().unwrap())
            .unwrap();

        // Round 3: no further mutation. The walk now sees the POST entry
        // in A, so the values list grows by one. The fix must republish
        // the schema this round (because last round's schema captured
        // a state that did NOT include POST). With the buggy
        // `last_schema_version = end_version`, schema_version equals
        // last_seen and no schema is sent — the decoder is left aligned
        // against last round's schema while the values list has grown.
        decoder
            .import_bytes(&encoder.export_bytes().unwrap())
            .unwrap();
        assert_eq!(
            decoder.encode_openmetrics_to_string().unwrap(),
            registry.encode_openmetrics_to_string().unwrap(),
            "decoder went out of sync after a mid-walk insert: \
             schema was not re-published when needed",
        );
    }
}
