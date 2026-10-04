use std::{any::Any, sync::Arc};

use crate::{
    Metric, MetricType, MetricValue,
    encoding::EncodableMetric,
    iterable::{FieldIter, IntoIterable, Iterable},
};

/// Trait for structs containing metric items.
pub trait MetricsGroup:
    Any + Iterable + IntoIterable + std::fmt::Debug + 'static + Send + Sync
{
    /// Returns the name of this metrics group.
    fn name(&self) -> &'static str;

    /// Returns an iterator over all metric items with their values and helps.
    fn iter(&self) -> FieldIter<'_> {
        self.field_iter()
    }
}

/// A metric item with its current value.
#[derive(Debug, Clone, Copy)]
pub struct MetricItem<'a> {
    pub(crate) name: &'static str,
    pub(crate) help: &'static str,
    pub(crate) metric: &'a dyn Metric,
}

impl EncodableMetric for MetricItem<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn help(&self) -> &str {
        self.help
    }

    fn r#type(&self) -> MetricType {
        self.metric.r#type()
    }

    fn value(&self) -> MetricValue {
        self.metric.value()
    }
}

impl<'a> MetricItem<'a> {
    /// Returns a new metric item.
    pub fn new(name: &'static str, help: &'static str, metric: &'a dyn Metric) -> Self {
        Self { name, help, metric }
    }

    /// Returns the inner metric as [`Any`], for further downcasting to concrete metric types.
    pub fn as_any(&self) -> &dyn Any {
        self.metric.as_any()
    }

    /// Returns the name of this metric item.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the help of this metric item.
    pub fn help(&self) -> &'static str {
        self.help
    }

    /// Returns the [`MetricType`] for this item.
    pub fn r#type(&self) -> MetricType {
        self.metric.r#type()
    }

    /// Returns the current value of this item.
    pub fn value(&self) -> MetricValue {
        self.metric.value()
    }
}

/// Trait for a set of structs implementing [`MetricsGroup`].
pub trait MetricsGroupSet {
    /// Returns the name of this metrics group set.
    fn name(&self) -> &'static str;

    /// Returns an iterator over owned clones of the [`MetricsGroup`] in this struct.
    fn groups_cloned(&self) -> impl Iterator<Item = Arc<dyn MetricsGroup>>;

    /// Returns an iterator over references to the [`MetricsGroup`] in this struct.
    fn groups(&self) -> impl Iterator<Item = &dyn MetricsGroup>;

    /// Returns an iterator over all metrics in this metrics group set.
    ///
    /// The iterator yields tuples of `(&str, MetricItem)`. The `&str` is the group name.
    fn iter(&self) -> impl Iterator<Item = (&'static str, MetricItem<'_>)> + '_ {
        self.groups()
            .flat_map(|group| group.iter().map(|item| (group.name(), item)))
    }
}

/// Ensure metrics can be used without `metrics` feature.
/// All ops are noops then, get always returns 0.
#[cfg(all(test, not(feature = "metrics")))]
mod tests {
    use crate::Counter;

    #[test]
    fn test() {
        let counter = Counter::new();
        counter.inc();
        assert_eq!(counter.get(), 0);
    }
}

/// Tests with the `metrics` feature,
#[cfg(all(test, feature = "metrics"))]
mod tests {
    use std::panic::{RefUnwindSafe, UnwindSafe};
    #[cfg(feature = "postcard")]
    use std::sync::RwLock;

    use iroh_metrics_derive::MetricsGroup;
    use serde::{Deserialize, Serialize};

    use super::*;
    #[cfg(feature = "postcard")]
    use crate::encoding::{Decoder, Encoder};
    use crate::{
        Counter, Family, Gauge, Histogram, MetricType, MetricsGroupSet, MetricsSource, Registry,
        iterable::Iterable,
    };

    #[derive(Debug, Iterable, Serialize, Deserialize)]
    pub struct FooMetrics {
        pub metric_a: Counter,
        pub metric_b: Gauge,
    }

    impl Default for FooMetrics {
        fn default() -> Self {
            Self {
                metric_a: Counter::new(),
                metric_b: Gauge::new(),
            }
        }
    }

    impl MetricsGroup for FooMetrics {
        fn name(&self) -> &'static str {
            "foo"
        }
    }

    #[derive(Debug, Default, Iterable, Serialize, Deserialize)]
    pub struct BarMetrics {
        /// Bar Count
        pub count: Counter,
    }

    impl MetricsGroup for BarMetrics {
        fn name(&self) -> &'static str {
            "bar"
        }
    }

    #[derive(Debug, Default, Serialize, Deserialize, MetricsGroupSet)]
    #[metrics(name = "combined")]
    struct CombinedMetrics {
        foo: Arc<FooMetrics>,
        bar: Arc<BarMetrics>,
    }

    // Making sure it is reasonably possible to write the trait impl ourselves.
    #[allow(unused)]
    #[derive(Debug, Default)]
    struct CombinedMetricsManual {
        foo: Arc<FooMetrics>,
        bar: Arc<BarMetrics>,
    }

    impl MetricsGroupSet for CombinedMetricsManual {
        fn name(&self) -> &'static str {
            "combined"
        }

        fn groups_cloned(&self) -> impl Iterator<Item = Arc<dyn MetricsGroup>> {
            [
                self.foo.clone() as Arc<dyn MetricsGroup>,
                self.bar.clone() as Arc<dyn MetricsGroup>,
            ]
            .into_iter()
        }

        fn groups(&self) -> impl Iterator<Item = &dyn MetricsGroup> {
            [
                &*self.foo as &dyn MetricsGroup,
                &*self.bar as &dyn MetricsGroup,
            ]
            .into_iter()
        }
    }

    #[test]
    fn test_metric_help() -> Result<(), Box<dyn std::error::Error>> {
        let metrics = FooMetrics::default();
        let items: Vec<_> = metrics.iter().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name(), "metric_a");
        assert_eq!(items[0].help(), "metric_a");
        assert_eq!(items[0].r#type(), MetricType::Counter);
        assert_eq!(items[1].name(), "metric_b");
        assert_eq!(items[1].help(), "metric_b");
        assert_eq!(items[1].r#type(), MetricType::Gauge);

        Ok(())
    }

    #[test]
    fn test_solo_registry() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = Registry::default();
        let metrics = Arc::new(FooMetrics::default());
        registry.register(metrics.clone());

        metrics.metric_a.inc();
        metrics.metric_b.inc_by(2);
        metrics.metric_b.inc_by(3);
        assert_eq!(metrics.metric_a.get(), 1);
        assert_eq!(metrics.metric_b.get(), 5);
        metrics.metric_a.set(0);
        metrics.metric_b.set(0);
        assert_eq!(metrics.metric_a.get(), 0);
        assert_eq!(metrics.metric_b.get(), 0);
        metrics.metric_a.inc_by(5);
        metrics.metric_b.inc_by(2);
        assert_eq!(metrics.metric_a.get(), 5);
        assert_eq!(metrics.metric_b.get(), 2);

        let exp = "# HELP foo_metric_a metric_a.
# TYPE foo_metric_a counter
foo_metric_a_total 5
# HELP foo_metric_b metric_b.
# TYPE foo_metric_b gauge
foo_metric_b 2
# EOF
";
        let enc = registry.encode_openmetrics_to_string()?;
        assert_eq!(enc, exp);
        Ok(())
    }

    #[test]
    fn test_metric_sets() {
        let metrics = CombinedMetrics::default();
        metrics.foo.metric_a.inc();
        metrics.foo.metric_b.set(-42);
        metrics.bar.count.inc_by(10);

        // Using `iter` to iterate over all metrics in the group set.
        let collected = metrics
            .iter()
            .map(|(group, metric)| (group, metric.name(), metric.help(), metric.value().to_f32()));
        assert_eq!(
            collected.collect::<Vec<_>>(),
            vec![
                ("foo", "metric_a", "metric_a", 1.0),
                ("foo", "metric_b", "metric_b", -42.0),
                ("bar", "count", "Bar Count", 10.0),
            ]
        );

        // Using manual downcasting.
        let mut collected = vec![];
        for group in metrics.groups() {
            for metric in group.iter() {
                if let Some(counter) = metric.as_any().downcast_ref::<Counter>() {
                    collected.push((group.name(), metric.name(), counter.value()));
                }
                if let Some(gauge) = metric.as_any().downcast_ref::<Gauge>() {
                    collected.push((group.name(), metric.name(), gauge.value()));
                }
            }
        }
        assert_eq!(
            collected,
            vec![
                ("foo", "metric_a", MetricValue::Counter(1)),
                ("foo", "metric_b", MetricValue::Gauge(-42)),
                ("bar", "count", MetricValue::Counter(10)),
            ]
        );

        // automatic collection and encoding with a registry
        let mut registry = Registry::default();
        let sub = registry.sub_registry_with_prefix("boo");
        sub.register_all(&metrics);
        let exp = "# HELP boo_foo_metric_a metric_a.
# TYPE boo_foo_metric_a counter
boo_foo_metric_a_total 1
# HELP boo_foo_metric_b metric_b.
# TYPE boo_foo_metric_b gauge
boo_foo_metric_b -42
# HELP boo_bar_count Bar Count.
# TYPE boo_bar_count counter
boo_bar_count_total 10
# EOF
";
        assert_eq!(registry.encode_openmetrics_to_string().unwrap(), exp);

        let sub = registry.sub_registry_with_labels([("x", "y")]);
        sub.register_all_prefixed(&metrics);
        let exp = r#"# HELP boo_foo_metric_a metric_a.
# TYPE boo_foo_metric_a counter
boo_foo_metric_a_total 1
# HELP boo_foo_metric_b metric_b.
# TYPE boo_foo_metric_b gauge
boo_foo_metric_b -42
# HELP boo_bar_count Bar Count.
# TYPE boo_bar_count counter
boo_bar_count_total 10
# HELP combined_foo_metric_a metric_a.
# TYPE combined_foo_metric_a counter
combined_foo_metric_a_total{x="y"} 1
# HELP combined_foo_metric_b metric_b.
# TYPE combined_foo_metric_b gauge
combined_foo_metric_b{x="y"} -42
# HELP combined_bar_count Bar Count.
# TYPE combined_bar_count counter
combined_bar_count_total{x="y"} 10
# EOF
"#;

        assert_eq!(registry.encode_openmetrics_to_string().unwrap(), exp);
    }

    #[test]
    fn test_derive() {
        use crate::{MetricValue, MetricsGroup};

        #[derive(Debug, MetricsGroup)]
        #[metrics(default, name = "my-metrics")]
        struct Metrics {
            /// Counts foos
            ///
            /// Only the first line is used for the OpenMetrics help
            foo: Counter,
            // no help: use field name as help
            bar: Counter,
            /// This docstring is not used as prometheus help
            #[metrics(help = "Measures baz")]
            baz: Gauge,
            #[metrics(help = "foo")]
            #[default(Histogram::new(vec![0.0, 0.01, 0.05, 0.1, 0.2, 0.5, 1.0]))]
            histo: Histogram,
        }

        let metrics = Metrics::default();
        assert_eq!(metrics.name(), "my-metrics");

        metrics.foo.inc();
        metrics.bar.inc_by(2);
        metrics.baz.set(3);

        let mut values = metrics.iter();
        let foo = values.next().unwrap();
        let bar = values.next().unwrap();
        let baz = values.next().unwrap();
        assert_eq!(foo.value(), MetricValue::Counter(1));
        assert_eq!(foo.name(), "foo");
        assert_eq!(foo.help(), "Counts foos");
        assert_eq!(bar.value(), MetricValue::Counter(2));
        assert_eq!(bar.name(), "bar");
        assert_eq!(bar.help(), "bar");
        assert_eq!(baz.value(), MetricValue::Gauge(3));
        assert_eq!(baz.name(), "baz");
        assert_eq!(baz.help(), "Measures baz");

        #[derive(Debug, Default, MetricsGroup)]
        struct FooMetrics {}
        let metrics = FooMetrics::default();
        assert_eq!(metrics.name(), "foo_metrics");
        let mut values = metrics.iter();
        assert!(values.next().is_none());
    }

    #[test]
    fn test_serde() {
        let metrics = CombinedMetrics::default();
        metrics.foo.metric_a.inc();
        metrics.foo.metric_b.set(-42);
        metrics.bar.count.inc_by(10);
        let encoded = postcard::to_stdvec(&metrics).unwrap();
        let decoded: CombinedMetrics = postcard::from_bytes(&encoded).unwrap();
        assert_eq!(decoded.foo.metric_a.get(), 1);
        assert_eq!(decoded.foo.metric_b.get(), -42);
        assert_eq!(decoded.bar.count.get(), 10);
    }

    #[test]
    #[cfg(feature = "postcard")]
    fn test_encode_decode() {
        let mut registry = Registry::default();
        let metrics = Arc::new(FooMetrics::default());
        registry.register(metrics.clone());

        metrics.metric_a.inc();
        metrics.metric_b.set(-42);

        let om_from_registry = registry.encode_openmetrics_to_string().unwrap();
        println!("openmetrics len {}", om_from_registry.len());

        let registry = Arc::new(RwLock::new(registry));

        let mut encoder = Encoder::new(registry.clone());
        let update = encoder.export_bytes().unwrap();
        println!("first update len {}", update.len());

        let mut decoder = Decoder::default();
        decoder.import_bytes(&update).unwrap();

        let om_from_decoder = decoder.encode_openmetrics_to_string().unwrap();
        assert_eq!(om_from_decoder, om_from_registry);

        metrics.metric_a.inc();
        metrics.metric_b.set(99);

        let update = encoder.export_bytes().unwrap();
        println!("second update len {}", update.len());
        decoder.import_bytes(&update).unwrap();

        let om_from_registry = registry.encode_openmetrics_to_string().unwrap();
        let om_from_decoder = decoder.encode_openmetrics_to_string().unwrap();
        assert_eq!(om_from_decoder, om_from_registry);

        for item in decoder.iter() {
            assert!(item.help.is_some());
        }

        let mut encoder = Encoder::new_with_opts(
            registry.clone(),
            crate::encoding::EncoderOpts {
                include_help: false,
            },
        );
        let mut decoder = Decoder::default();
        decoder.import_bytes(&update).unwrap();
        decoder.import(encoder.export());
        for item in decoder.iter() {
            assert_eq!(item.help, None);
        }
    }

    #[test]
    fn test_histogram() {
        use crate::Histogram;

        let histogram = Histogram::new(vec![1.0, 5.0, 10.0, 50.0, 100.0, f64::INFINITY]);

        histogram.observe(0.5);
        histogram.observe(2.5);
        histogram.observe(7.5);
        histogram.observe(25.0);
        histogram.observe(75.0);
        histogram.observe(150.0);

        assert_eq!(histogram.count(), 6);
        assert_eq!(histogram.sum(), 260.5);

        let buckets = histogram.buckets();
        assert_eq!(buckets.len(), 6);
        assert_eq!(buckets[0], (1.0, 1));
        assert_eq!(buckets[1], (5.0, 2));
        assert_eq!(buckets[2], (10.0, 3));
        assert_eq!(buckets[3], (50.0, 4));
        assert_eq!(buckets[4], (100.0, 5));
        assert_eq!(buckets[5], (f64::INFINITY, 6));

        let p50 = histogram.percentile(0.5);
        assert_eq!(p50, 10.0);

        let p99 = histogram.percentile(0.99);
        assert_eq!(p99, 100.0);

        let p100 = histogram.percentile(1.0);
        assert_eq!(p100, f64::INFINITY);
    }

    #[test]
    fn test_histogram_prometheus_format() {
        use crate::Histogram;

        #[derive(Debug, Iterable)]
        pub struct HistogramMetrics {
            pub response_time: Histogram,
        }

        impl MetricsGroup for HistogramMetrics {
            fn name(&self) -> &'static str {
                "http"
            }
        }

        let metrics = HistogramMetrics {
            response_time: Histogram::new(vec![0.1, 0.5, 1.0, 5.0, f64::INFINITY]),
        };

        metrics.response_time.observe(0.05);
        metrics.response_time.observe(0.3);
        metrics.response_time.observe(0.8);
        metrics.response_time.observe(2.5);

        let mut registry = Registry::default();
        registry.register(Arc::new(metrics));

        let output = registry.encode_openmetrics_to_string().unwrap();

        let parsed = prometheus_parse::Scrape::parse(output.lines().map(|s| Ok(s.to_owned())))
            .expect("Failed to parse Prometheus output");

        assert_eq!(parsed.samples.len(), 3);

        let histogram_sample = parsed
            .samples
            .iter()
            .find(|s| s.metric == "http_response_time")
            .expect("Expected to find http_response_time histogram");

        let sum_sample = parsed
            .samples
            .iter()
            .find(|s| s.metric == "http_response_time_sum")
            .expect("Expected to find http_response_time_sum");

        let count_sample = parsed
            .samples
            .iter()
            .find(|s| s.metric == "http_response_time_count")
            .expect("Expected to find http_response_time_count");

        if let prometheus_parse::Value::Untyped(sum) = sum_sample.value {
            assert_eq!(sum, 3.65);
        } else {
            panic!("Expected sum value");
        }

        if let prometheus_parse::Value::Untyped(count) = count_sample.value {
            assert_eq!(count, 4.0);
        } else {
            panic!("Expected count value");
        }

        if let prometheus_parse::Value::Histogram(buckets) = &histogram_sample.value {
            assert_eq!(buckets.len(), 5);

            assert_eq!(buckets[0].less_than, 0.1);
            assert_eq!(buckets[0].count, 1.0);

            assert_eq!(buckets[1].less_than, 0.5);
            assert_eq!(buckets[1].count, 2.0);

            assert_eq!(buckets[2].less_than, 1.0);
            assert_eq!(buckets[2].count, 3.0);

            assert_eq!(buckets[3].less_than, 5.0);
            assert_eq!(buckets[3].count, 4.0);

            assert_eq!(buckets[4].less_than, f64::INFINITY);
            assert_eq!(buckets[4].count, 4.0);
        } else {
            panic!("Expected histogram value, got {:?}", histogram_sample.value);
        }
    }

    #[test]
    #[cfg(feature = "postcard")]
    fn test_histogram_encode_decode() {
        use std::sync::{Arc, RwLock};

        use crate::Histogram;

        #[derive(Debug, Iterable)]
        pub struct HistogramMetrics {
            pub response_time: Histogram,
        }

        impl MetricsGroup for HistogramMetrics {
            fn name(&self) -> &'static str {
                "http"
            }
        }

        let mut registry = Registry::default();
        let metrics = Arc::new(HistogramMetrics {
            response_time: Histogram::new(vec![0.1, 0.5, 1.0, 5.0, f64::INFINITY]),
        });
        registry.register(metrics.clone());

        metrics.response_time.observe(0.05);
        metrics.response_time.observe(0.3);
        metrics.response_time.observe(0.8);
        metrics.response_time.observe(2.5);

        let registry = Arc::new(RwLock::new(registry));

        let mut encoder = Encoder::new(registry.clone());
        let update = encoder.export_bytes().unwrap();

        let mut decoder = Decoder::default();
        decoder.import_bytes(&update).unwrap();

        let mut items = decoder.iter();
        let item = items.next().expect("Expected one metric");

        if let MetricValue::Histogram {
            buckets,
            sum,
            count,
        } = item.value
        {
            assert_eq!(*count, 4);
            assert_eq!(*sum, 3.65);
            assert_eq!(buckets.len(), 5);
            assert_eq!(buckets[0], (0.1, 1));
            assert_eq!(buckets[1], (0.5, 2));
            assert_eq!(buckets[2], (1.0, 3));
            assert_eq!(buckets[3], (5.0, 4));
            assert_eq!(buckets[4], (f64::INFINITY, 4));
        } else {
            panic!("Expected histogram value");
        }

        metrics.response_time.observe(0.02);
        metrics.response_time.observe(1.5);

        let update = encoder.export_bytes().unwrap();
        decoder.import_bytes(&update).unwrap();

        let mut items = decoder.iter();
        let item = items.next().expect("Expected one metric");

        if let MetricValue::Histogram {
            buckets,
            sum,
            count,
        } = item.value
        {
            assert_eq!(*count, 6);
            assert_eq!(*sum, 5.17);
            assert_eq!(buckets[0], (0.1, 2)); // 0.05, 0.02
            assert_eq!(buckets[1], (0.5, 3)); // + 0.3
            assert_eq!(buckets[2], (1.0, 4)); // + 0.8
            assert_eq!(buckets[3], (5.0, 6)); // + 2.5, 1.5
            assert_eq!(buckets[4], (f64::INFINITY, 6));
        } else {
            panic!("Expected histogram value");
        }
    }

    #[test]
    #[cfg(feature = "postcard")]
    fn test_histogram_openmetrics_from_decoder() {
        use std::sync::{Arc, RwLock};

        use crate::Histogram;

        #[derive(Debug, Iterable)]
        pub struct HistogramMetrics {
            pub response_time: Histogram,
        }

        impl MetricsGroup for HistogramMetrics {
            fn name(&self) -> &'static str {
                "http"
            }
        }

        let mut registry = Registry::default();
        let metrics = Arc::new(HistogramMetrics {
            response_time: Histogram::new(vec![0.1, 0.5, 1.0, 5.0, f64::INFINITY]),
        });
        registry.register(metrics.clone());

        metrics.response_time.observe(0.05);
        metrics.response_time.observe(0.3);
        metrics.response_time.observe(0.8);
        metrics.response_time.observe(2.5);

        let om_from_registry = registry.encode_openmetrics_to_string().unwrap();

        let registry = Arc::new(RwLock::new(registry));
        let mut encoder = Encoder::new(registry.clone());
        let update = encoder.export_bytes().unwrap();

        let mut decoder = Decoder::default();
        decoder.import_bytes(&update).unwrap();

        let om_from_decoder = decoder.encode_openmetrics_to_string().unwrap();

        assert_eq!(
            om_from_decoder, om_from_registry,
            "Decoder should produce identical OpenMetrics output to registry for histograms"
        );
    }

    #[test]
    fn test_family_in_metrics_group() {
        use std::borrow::Cow;

        use iroh_metrics_derive::MetricsGroup;

        use crate::{Family, LabelPair, LabelValue, NoLabels};

        #[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
        struct TransportLabels {
            transport: String,
        }

        impl crate::EncodeLabelSet for TransportLabels {
            fn encode_label_pairs(&self) -> Vec<LabelPair<'_>> {
                vec![("transport", LabelValue::Str(Cow::Borrowed(&self.transport)))]
            }
        }

        #[derive(Debug, MetricsGroup)]
        #[metrics(default, name = "magicsock")]
        struct Metrics {
            /// Total bytes sent
            total_bytes: Counter,
            /// Bytes by transport
            bytes_by_transport: Family<TransportLabels, Counter>,
            /// Latency histogram
            #[default(Family::with_constructor(|| crate::Histogram::new(vec![0.1, 1.0, 10.0])))]
            latency: Family<NoLabels, crate::Histogram>,
        }

        let metrics = Metrics::default();

        // Regular metric
        metrics.total_bytes.inc_by(100);

        // Family metrics
        metrics
            .bytes_by_transport
            .get_or_create(&TransportLabels {
                transport: "ipv4".into(),
            })
            .inc_by(50);
        metrics
            .bytes_by_transport
            .get_or_create(&TransportLabels {
                transport: "relay".into(),
            })
            .inc_by(30);
        metrics.latency.get_or_create(&NoLabels).observe(0.5);

        // Check regular metrics iteration
        let regular_count = metrics.iter().count();
        assert_eq!(regular_count, 1, "Should have 1 regular metric");

        // Check family iteration
        let family_count = IntoIterable::family_iter(&metrics).count();
        assert_eq!(family_count, 2, "Should have 2 family metrics");

        // Register and encode
        let mut registry = Registry::default();
        registry.register(Arc::new(metrics));

        let output = registry.encode_openmetrics_to_string().unwrap();
        assert!(output.contains("magicsock_total_bytes_total 100"));
        assert!(output.contains("magicsock_bytes_by_transport"));
        assert!(output.contains(r#"transport="ipv4""#));
        assert!(output.contains(r#"transport="relay""#));
        assert!(output.contains("magicsock_latency"));
    }

    #[test]
    fn test_unwind_safe() {
        #[derive(
            Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, iroh_metrics::EncodeLabelSet,
        )]
        struct Proto {
            proto: String,
        }

        #[derive(Debug, Default, MetricsGroup)]
        #[metrics(name = "no-family")]
        struct MetricsWithoutFamily {
            bytes: Counter,
        }

        #[derive(Debug, Default, MetricsGroup)]
        #[metrics(name = "family")]
        struct MetricsWithFamily {
            bytes: Counter,
            bytes_by_proto: Family<Proto, Counter>,
        }

        fn assert_unwind_safe<T: UnwindSafe + RefUnwindSafe>(x: T) {
            let _ = x;
        }

        let without_family = MetricsWithoutFamily::default();
        let with_family = MetricsWithFamily::default();
        assert_unwind_safe(without_family);
        assert_unwind_safe(with_family);

        // `with_constructor` must keep accepting closures that are not
        // `RefUnwindSafe`, and a family built from one must still be unwind safe.
        let inner: Arc<dyn Fn() -> Counter + Send + Sync> = Arc::new(Counter::default);
        let family: Family<Proto, Counter> = Family::with_constructor(move || inner());
        assert_unwind_safe(family);
    }

    // Shared fixtures for the family-related tests below.
    #[cfg(test)]
    mod family_tests {
        use std::sync::{Arc, RwLock};

        use iroh_metrics_derive::MetricsGroup;

        use crate::{
            Counter, Family, MetricsSource, Registry,
            encoding::{Decoder, Encoder, Update},
            iterable::IntoIterable,
        };

        #[derive(
            Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, iroh_metrics::EncodeLabelSet,
        )]
        struct Proto {
            proto: String,
        }

        fn proto(s: &str) -> Proto {
            Proto { proto: s.into() }
        }

        #[derive(Debug, Default, MetricsGroup)]
        #[metrics(name = "net")]
        struct Net {
            /// Total bytes
            bytes: Counter,
            /// Bytes per protocol
            bytes_by_proto: Family<Proto, Counter>,
        }

        fn registered() -> (Arc<Net>, Arc<RwLock<Registry>>) {
            let metrics = Arc::new(Net::default());
            let mut registry = Registry::default();
            registry.register(metrics.clone());
            (metrics, Arc::new(RwLock::new(registry)))
        }

        #[test]
        #[cfg(feature = "postcard")]
        fn encode_decode_roundtrip_matches_registry() {
            let (metrics, registry) = registered();
            metrics.bytes.inc_by(100);
            metrics
                .bytes_by_proto
                .get_or_create(&proto("tcp"))
                .inc_by(40);
            metrics
                .bytes_by_proto
                .get_or_create(&proto("udp"))
                .inc_by(60);

            let mut encoder = Encoder::new(registry.clone());
            let mut decoder = Decoder::default();
            decoder
                .import_bytes(&encoder.export_bytes().unwrap())
                .unwrap();

            let from_decoder = decoder.encode_openmetrics_to_string().unwrap();
            assert_eq!(
                from_decoder,
                registry.encode_openmetrics_to_string().unwrap()
            );
            assert!(from_decoder.contains(r#"net_bytes_by_proto_total{proto="tcp"} 40"#));
            assert!(from_decoder.contains(r#"net_bytes_by_proto_total{proto="udp"} 60"#));

            // Values-only update (no schema change) must still round-trip.
            metrics
                .bytes_by_proto
                .get_or_create(&proto("tcp"))
                .inc_by(5);
            decoder
                .import_bytes(&encoder.export_bytes().unwrap())
                .unwrap();
            assert_eq!(
                decoder.encode_openmetrics_to_string().unwrap(),
                registry.encode_openmetrics_to_string().unwrap(),
            );
        }

        #[test]
        fn new_label_combo_bumps_schema_version() {
            let (metrics, registry) = registered();
            metrics.bytes_by_proto.get_or_create(&proto("tcp")).inc();

            let mut encoder = Encoder::new(registry.clone());
            // First export publishes schema; second is cached.
            let first = encoder.export();
            assert_eq!(first.schema.expect("initial schema").items.len(), 2);
            assert!(encoder.export().schema.is_none(), "schema must be cached");

            // New label combo invalidates the cached schema.
            metrics.bytes_by_proto.get_or_create(&proto("udp")).inc();
            let third = encoder.export();
            let schema = third.schema.expect("schema re-published after new combo");
            assert_eq!(schema.items.len(), 3);

            let mut decoder = Decoder::default();
            decoder.import(Update {
                schema: Some(schema),
                values: third.values,
            });
            assert_eq!(
                decoder.encode_openmetrics_to_string().unwrap(),
                registry.encode_openmetrics_to_string().unwrap(),
            );
        }

        #[test]
        fn metrics_family_attr_overrides_alias_detection() {
            type Aliased = Family<Proto, Counter>;

            #[derive(Debug, Default, MetricsGroup)]
            #[metrics(name = "alias")]
            struct AliasMetrics {
                #[metrics(family)]
                via_alias: Aliased,
            }

            let m = AliasMetrics::default();
            assert_eq!(IntoIterable::family_iter(&m).count(), 1);
            assert_eq!(crate::MetricsGroup::iter(&m).count(), 0);
        }
    }
}
