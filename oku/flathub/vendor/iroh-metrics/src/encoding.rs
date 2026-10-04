//! Functions to encode metrics into the [OpenMetrics text format].
//!
//! [OpenMetrics text format]: https://github.com/prometheus/OpenMetrics/blob/main/specification/OpenMetrics.md

use std::{
    borrow::Cow,
    fmt::{self, Write},
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::{
    LabelValue, MetricItem, MetricType, MetricValue, MetricsGroup, MetricsSource, RwLockRegistry,
    iterable::IntoIterable,
};

/// Encodes a label value directly into the writer.
///
/// Blanket-implemented for any `AsRef<str>` so existing callers using `&str`,
/// `String`, or `Cow<str>` continue to work. The dedicated impl for
/// [`LabelValue`] avoids allocating an intermediate `String` for integer and
/// boolean variants.
pub(crate) trait EncodeLabelTo {
    fn encode_to<W: Write + ?Sized>(&self, w: &mut W) -> fmt::Result;
}

impl<T: AsRef<str> + ?Sized> EncodeLabelTo for T {
    fn encode_to<W: Write + ?Sized>(&self, w: &mut W) -> fmt::Result {
        encode_escaped(w, self.as_ref(), true)
    }
}

impl EncodeLabelTo for LabelValue<'_> {
    fn encode_to<W: Write + ?Sized>(&self, w: &mut W) -> fmt::Result {
        match self {
            // Numeric and boolean variants don't need escaping.
            LabelValue::Str(s) => encode_escaped(w, s, true),
            LabelValue::Int(v) => w.write_str(itoa::Buffer::new().format(*v)),
            LabelValue::Uint(v) => w.write_str(itoa::Buffer::new().format(*v)),
            LabelValue::Bool(v) => w.write_str(if *v { "true" } else { "false" }),
        }
    }
}

/// Writes `s` with OpenMetrics escaping: `\` → `\\` and `\n` → `\n` always,
/// plus `"` → `\"` when `escape_quote` is set (label-value rules vs HELP-text
/// rules).
fn encode_escaped<W: Write + ?Sized>(w: &mut W, s: &str, escape_quote: bool) -> fmt::Result {
    let mut start = 0;
    for (idx, c) in s.char_indices() {
        let replacement = match c {
            '\\' => Some("\\\\"),
            '\n' => Some("\\n"),
            '"' if escape_quote => Some("\\\""),
            _ => None,
        };
        if let Some(rep) = replacement {
            if start < idx {
                w.write_str(&s[start..idx])?;
            }
            w.write_str(rep)?;
            start = idx + c.len_utf8();
        }
    }
    if start < s.len() {
        w.write_str(&s[start..])?;
    }
    Ok(())
}

pub(crate) fn encode_eof(writer: &mut impl Write) -> fmt::Result {
    writer.write_str("# EOF\n")
}

/// Encodes a metric value (without HELP/TYPE headers) in OpenMetrics format.
///
/// The following suffixes are appended to the metric name per the OpenMetrics spec:
/// - Counter: `_total` (e.g. `my_counter_total`)
/// - Gauge: no suffix
/// - Histogram: `_bucket` (with `le` label), `_sum`, `_count`
pub(crate) fn encode_metric_value<W, K1, V1, K2, V2>(
    writer: &mut W,
    name: &str,
    prefixes: &[impl AsRef<str>],
    labels: &[(K1, V1)],
    extra_labels: &[(K2, V2)],
    value: &MetricValue,
) -> fmt::Result
where
    W: Write + ?Sized,
    K1: AsRef<str>,
    V1: EncodeLabelTo,
    K2: AsRef<str>,
    V2: EncodeLabelTo,
{
    match value {
        MetricValue::Counter(v) => {
            encode_prefix_name(writer, prefixes, name)?;
            // Counters require a `_total` suffix per the OpenMetrics spec.
            // Skip if the encoded name already ends with `_total`. Two cases:
            //   - `name` itself ends with `_total` (e.g. `requests_total`).
            //   - `name` is exactly `total` and there is at least one prefix,
            //     so the prefix's `_` separator turns the join into
            //     `..._total`.
            let already_total =
                name.ends_with("_total") || (name == "total" && !prefixes.is_empty());
            if !already_total {
                writer.write_str("_total")?;
            }
            encode_labels(writer, labels, extra_labels, None)?;
            writer.write_char(' ')?;
            encode_u64(writer, *v)?;
            writer.write_char('\n')?;
        }
        MetricValue::Gauge(v) => {
            encode_prefix_name(writer, prefixes, name)?;
            encode_labels(writer, labels, extra_labels, None)?;
            writer.write_char(' ')?;
            encode_i64(writer, *v)?;
            writer.write_char('\n')?;
        }
        MetricValue::Histogram {
            buckets,
            sum,
            count,
        } => {
            for (le, cnt) in buckets {
                encode_prefix_name(writer, prefixes, name)?;
                writer.write_str("_bucket")?;
                encode_labels(writer, labels, extra_labels, Some(*le))?;
                writer.write_char(' ')?;
                encode_u64(writer, *cnt)?;
                writer.write_char('\n')?;
            }

            encode_prefix_name(writer, prefixes, name)?;
            writer.write_str("_sum")?;
            encode_labels(writer, labels, extra_labels, None)?;
            writer.write_char(' ')?;
            encode_f64(writer, *sum)?;
            writer.write_char('\n')?;

            encode_prefix_name(writer, prefixes, name)?;
            writer.write_str("_count")?;
            encode_labels(writer, labels, extra_labels, None)?;
            writer.write_char(' ')?;
            encode_u64(writer, *count)?;
            writer.write_char('\n')?;
        }
    }
    Ok(())
}

fn encode_labels<W, K1, V1, K2, V2>(
    w: &mut W,
    labels: &[(K1, V1)],
    extra_labels: &[(K2, V2)],
    le: Option<f64>,
) -> fmt::Result
where
    W: Write + ?Sized,
    K1: AsRef<str>,
    V1: EncodeLabelTo,
    K2: AsRef<str>,
    V2: EncodeLabelTo,
{
    if labels.is_empty() && extra_labels.is_empty() && le.is_none() {
        return Ok(());
    }

    w.write_char('{')?;
    let mut first = true;

    for (k, v) in labels {
        if !first {
            w.write_char(',')?;
        }
        w.write_str(k.as_ref())?;
        w.write_str("=\"")?;
        v.encode_to(w)?;
        w.write_char('"')?;
        first = false;
    }

    for (k, v) in extra_labels {
        if !first {
            w.write_char(',')?;
        }
        w.write_str(k.as_ref())?;
        w.write_str("=\"")?;
        v.encode_to(w)?;
        w.write_char('"')?;
        first = false;
    }

    if let Some(le) = le {
        if !first {
            w.write_char(',')?;
        }
        if le.is_infinite() {
            w.write_str("le=\"+Inf\"")?;
        } else {
            write!(w, "le=\"{}\"", ryu::Buffer::new().format(le))?;
        }
    }

    w.write_char('}')
}

/// Writes `# EOF\n` to `writer`.
///
/// This is the expected last characters of an OpenMetrics string.
pub fn encode_openmetrics_eof(writer: &mut impl Write) -> fmt::Result {
    encode_eof(writer)
}

/// Schema information for a single metric item.
///
/// Contains metadata about a metric including its type, name, help text,
/// prefixes, and labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSchema {
    /// The type of the metric (Counter, Gauge, etc.)
    pub r#type: MetricType,
    /// The name of the metric
    pub name: String,
    /// Prefixes to prepend to the metric name
    pub prefixes: Vec<String>,
    /// Labels associated with the metric as key-value pairs
    pub labels: Vec<(String, String)>,
}

impl ItemSchema {
    /// Creates a new schema item from the given metadata.
    pub fn new<K, V>(
        name: &str,
        prefixes: &[impl AsRef<str>],
        labels: &[(K, V)],
        metric_type: MetricType,
    ) -> Self
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        Self::from_label_iter(
            name,
            prefixes,
            labels.iter().map(|(k, v)| (k.as_ref(), v.as_ref())),
            metric_type,
        )
    }

    /// Creates a new schema item from a label iterator (avoids materializing a slice).
    pub fn from_label_iter<'a, I, K, V>(
        name: &str,
        prefixes: &[impl AsRef<str>],
        labels: I,
        metric_type: MetricType,
    ) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str> + 'a,
        V: AsRef<str> + 'a,
    {
        Self {
            name: name.to_string(),
            prefixes: prefixes.iter().map(|s| s.as_ref().to_string()).collect(),
            labels: labels
                .into_iter()
                .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
                .collect(),
            r#type: metric_type,
        }
    }

    /// Returns the name prefixed with all prefixes.
    pub fn prefixed_name(&self) -> String {
        let mut out = String::new();
        for prefix in &self.prefixes {
            out.push_str(prefix);
            out.push('_');
        }
        out.push_str(&self.name);
        out
    }
}

/// A collection of metric schemas.
///
/// Contains all the schema information for a set of metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// The individual metric schemas
    pub items: Vec<ItemSchema>,
    /// Help texts (may be omitted)
    pub help: Option<Vec<String>>,
}

impl Schema {
    /// Pushes a schema item and its help text.
    pub fn push(&mut self, item: ItemSchema, help: &str) {
        self.items.push(item);
        if let Some(h) = self.help.as_mut() {
            h.push(help.to_string());
        }
    }

    /// Creates a new [`Schema`] that does not track help text.
    pub fn new_without_help() -> Self {
        Self {
            items: Default::default(),
            help: None,
        }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            help: Some(Vec::new()),
        }
    }
}

/// Histogram data wrapper
///
/// Contains the bucket counts, sum, and total count for a histogram metric.
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct HistogramData {
    /// Bucket upper bounds and cumulative counts
    pub buckets: Vec<(f64, u64)>,
    /// Sum of all observed values
    pub sum: f64,
    /// Total count of observations
    pub count: u64,
}

/// A collection of metric values.
///
/// Contains the actual values for a set of metrics.
#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct Values {
    /// The individual metric values
    pub items: Vec<MetricValue>,
}

/// An update containing schema and/or values for metrics.
///
/// Used to transfer metric information between encoders and decoders.
/// The schema is optional and only included when it has changed.
#[derive(Debug, Serialize, Clone, Deserialize, Default)]
pub struct Update {
    /// Optional schema information (included when schema changes)
    pub schema: Option<Schema>,
    /// The metric values
    pub values: Values,
}

/// A metric item combining schema and value information.
///
/// Provides a unified view of a metric's metadata and current value.
#[derive(Debug)]
pub struct Item<'a> {
    /// Reference to the metric's schema information
    pub schema: &'a ItemSchema,
    /// Reference to the metric's current value
    pub value: &'a MetricValue,
    /// Help text, if available
    pub help: Option<&'a String>,
}

impl EncodableMetric for Item<'_> {
    fn name(&self) -> &str {
        &self.schema.name
    }

    fn help(&self) -> &str {
        self.help.map(|x| x.as_str()).unwrap_or_default()
    }

    fn r#type(&self) -> MetricType {
        self.schema.r#type
    }

    fn value(&self) -> MetricValue {
        self.value.clone()
    }
}

impl Item<'_> {
    /// Encodes this metric item to OpenMetrics format.
    ///
    /// Writes the metric in OpenMetrics text format to the provided writer.
    /// Always includes `# HELP` and `# TYPE` headers — if you are iterating
    /// items yourself across a family of metrics, prefer
    /// [`MetricsSource::encode_openmetrics`]
    /// on the [`Decoder`], which deduplicates headers across same-named items.
    pub fn encode_openmetrics(
        &self,
        writer: &mut impl std::fmt::Write,
    ) -> Result<(), crate::Error> {
        EncodableMetric::encode_openmetrics(
            self,
            writer,
            self.schema.prefixes.as_slice(),
            self.schema
                .labels
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str())),
        )?;
        Ok(())
    }

    /// Writes only the `# HELP` and `# TYPE` headers for this item.
    pub(crate) fn encode_openmetrics_header(
        &self,
        writer: &mut impl std::fmt::Write,
    ) -> fmt::Result {
        writer.write_str("# HELP ")?;
        encode_prefix_name(writer, &self.schema.prefixes, &self.schema.name)?;
        writer.write_str(" ")?;
        encode_help_text(writer, self.help.map(|x| x.as_str()).unwrap_or_default())?;

        writer.write_str("# TYPE ")?;
        encode_prefix_name(writer, &self.schema.prefixes, &self.schema.name)?;
        writer.write_str(" ")?;
        writer.write_str(self.schema.r#type.as_str())?;
        writer.write_str("\n")
    }

    /// Writes only the value line(s) for this item, without `# HELP`/`# TYPE`.
    pub(crate) fn encode_openmetrics_value(
        &self,
        writer: &mut impl std::fmt::Write,
    ) -> fmt::Result {
        let labels: Vec<_> = self
            .schema
            .labels
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let empty: &[(&str, &str)] = &[];
        encode_metric_value(
            writer,
            &self.schema.name,
            &self.schema.prefixes,
            &labels,
            empty,
            self.value,
        )
    }
}

/// Decoder for metrics received from an [`Encoder`]
///
/// Implements [`MetricsSource`] to export the decoded metrics to OpenMetrics.
#[derive(Debug, Clone, Default)]
pub struct Decoder {
    schema: Option<Schema>,
    values: Values,
}

impl Decoder {
    /// Imports a metric update.
    ///
    /// Updates the decoder's schema (if provided) and values with the given update.
    pub fn import(&mut self, update: Update) {
        if let Some(schema) = update.schema {
            self.schema = Some(schema);
        }
        self.values = update.values;
    }

    /// Imports a metric update from serialized bytes.
    ///
    /// Deserializes the bytes using postcard and imports the resulting update.
    #[cfg(feature = "postcard")]
    pub fn import_bytes(&mut self, data: &[u8]) -> Result<(), postcard::Error> {
        let update = postcard::from_bytes(data)?;
        self.import(update);
        Ok(())
    }

    /// Creates an iterator over the decoded metric items.
    ///
    /// Returns an iterator that yields [`Item`] instances combining schema and value data.
    pub fn iter(&self) -> DecoderIter<'_> {
        DecoderIter {
            pos: 0,
            inner: self,
        }
    }
}

/// Iterator over decoded metric items.
///
/// Iterates through the metrics in a [`Decoder`], yielding [`Item`] instances.
#[derive(Debug)]
pub struct DecoderIter<'a> {
    /// Current position in the iteration
    pos: usize,
    /// Reference to the decoder being iterated
    inner: &'a Decoder,
}

impl<'a> Iterator for DecoderIter<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let schema = self.inner.schema.as_ref()?.items.get(self.pos)?;
        let value = self.inner.values.items.get(self.pos)?;
        let help = self
            .inner
            .schema
            .as_ref()?
            .help
            .as_ref()
            .and_then(|help| help.get(self.pos));
        self.pos += 1;
        Some(Item {
            schema,
            value,
            help,
        })
    }
}

impl MetricsSource for Decoder {
    fn encode_openmetrics(&self, writer: &mut impl std::fmt::Write) -> Result<(), crate::Error> {
        // Family entries share name and type but appear as separate items in
        // the schema. Emit `# HELP` / `# TYPE` only when the prefixed name
        // changes between consecutive items, matching what the registry path
        // produces.
        let mut prev_key: Option<(Vec<String>, String)> = None;
        for item in self.iter() {
            let key = (item.schema.prefixes.clone(), item.schema.name.clone());
            if prev_key.as_ref() != Some(&key) {
                item.encode_openmetrics_header(writer)?;
                prev_key = Some(key);
            }
            item.encode_openmetrics_value(writer)?;
        }
        encode_eof(writer)?;
        Ok(())
    }
}

impl MetricsSource for Arc<RwLock<Decoder>> {
    fn encode_openmetrics(&self, writer: &mut impl std::fmt::Write) -> Result<(), crate::Error> {
        self.read().expect("poisoned").encode_openmetrics(writer)
    }
}

/// Encoder for converting metrics from a registry into serializable updates.
///
/// Tracks schema changes and generates [`Update`] objects that can be
/// transmitted to a [`Decoder`].
#[derive(Debug)]
pub struct Encoder {
    /// The metrics registry to encode from
    registry: RwLockRegistry,
    /// Version of the last schema that was exported
    last_schema_version: u64,
    opts: EncoderOpts,
}

/// Options for an [`Encoder`]
#[derive(Debug)]
#[non_exhaustive]
pub struct EncoderOpts {
    /// Whether to include the metric help text in the transmitted schema.
    pub include_help: bool,
}

impl Default for EncoderOpts {
    fn default() -> Self {
        Self { include_help: true }
    }
}

impl Encoder {
    /// Creates a new encoder for the given registry.
    ///
    /// The encoder will track schema changes and only include schema
    /// information in updates when it has changed.
    pub fn new(registry: RwLockRegistry) -> Self {
        Self::new_with_opts(registry, Default::default())
    }

    /// Creates a new encoder for the given registry with custom options.
    pub fn new_with_opts(registry: RwLockRegistry, opts: EncoderOpts) -> Self {
        Self {
            registry,
            last_schema_version: 0,
            opts,
        }
    }

    /// Exports the current state of the registry as an update.
    ///
    /// Returns an [`Update`] containing the current metric values and
    /// optionally the schema (if it has changed since the last export).
    ///
    /// Each family's schema items and values are pushed under a single read
    /// lock per family (see `FamilyEncoder::encode_schema`), so the two flat
    /// slices are always internally consistent. We capture the version
    /// *before* the walk and advance `last_schema_version` only that far —
    /// any insert that races between two families' walks bumps the version
    /// past `start_version`, forcing the *next* export to re-publish a
    /// schema that matches the values it sends. Advancing past the captured
    /// version would let the next round skip publishing while the values
    /// list has already grown, leaving the decoder one entry behind for
    /// every later item.
    pub fn export(&mut self) -> Update {
        let registry = self.registry.read().expect("poisoned");
        let last_seen = self.last_schema_version;
        let start_version = registry.schema_version();
        let mut schema = if self.opts.include_help {
            Schema::default()
        } else {
            Schema::new_without_help()
        };
        let mut values = Values::default();
        registry.encode_schema(Some(&mut schema), &mut values);

        let end_version = registry.schema_version();
        self.last_schema_version = start_version;
        let schema = (end_version != last_seen).then_some(schema);
        Update { schema, values }
    }

    /// Exports the current state of the registry as serialized bytes.
    ///
    /// Returns the serialized bytes of an [`Update`] using postcard encoding.
    #[cfg(feature = "postcard")]
    pub fn export_bytes(&mut self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_stdvec(&self.export())
    }
}

impl dyn MetricsGroup {
    pub(crate) fn encode_schema<'a>(
        &self,
        mut schema: Option<&mut Schema>,
        values: &mut Values,
        prefix: Option<&'a str>,
        labels: &[(Cow<'a, str>, Cow<'a, str>)],
    ) {
        let name = self.name();
        let prefixes = if let Some(prefix) = prefix {
            &[prefix, name][..]
        } else {
            &[name]
        };
        for metric in self.iter() {
            if let Some(schema) = schema.as_deref_mut() {
                let lbls = labels.iter().map(|(k, v)| (k.as_ref(), v.as_ref()));
                metric.encode_schema(schema, prefixes, lbls);
            }
            metric.encode_value(values);
        }
        for family in IntoIterable::family_iter(self) {
            family.encode_schema(schema.as_deref_mut(), values, prefixes, labels);
        }
    }

    pub(crate) fn encode_openmetrics<'a>(
        &self,
        writer: &'a mut impl Write,
        prefix: Option<&'a str>,
        labels: &[(Cow<'a, str>, Cow<'a, str>)],
    ) -> fmt::Result {
        let name = self.name();
        let prefixes = if let Some(prefix) = prefix {
            &[prefix, name] as &[&str]
        } else {
            &[name]
        };
        for metric in self.iter() {
            let labels = labels.iter().map(|(k, v)| (k.as_ref(), v.as_ref()));
            metric.encode_openmetrics(writer, prefixes, labels)?;
        }
        for family in IntoIterable::family_iter(self) {
            family.encode_openmetrics(writer, prefixes, labels)?;
        }
        Ok(())
    }
}

/// Trait for types that can provide metric encoding information.
pub(crate) trait EncodableMetric {
    /// Returns the name of this metric item.
    fn name(&self) -> &str;

    /// Returns the help of this metric item.
    fn help(&self) -> &str;

    /// Returns the [`MetricType`] for this item.
    fn r#type(&self) -> MetricType;

    /// Returns the current value of this item.
    fn value(&self) -> MetricValue;

    /// Encode the metrics item in the OpenMetrics text format.
    fn encode_openmetrics<'a>(
        &self,
        writer: &mut impl Write,
        prefixes: &[impl AsRef<str>],
        labels: impl Iterator<Item = (&'a str, &'a str)> + 'a,
    ) -> fmt::Result {
        writer.write_str("# HELP ")?;
        encode_prefix_name(writer, prefixes, self.name())?;
        writer.write_str(" ")?;
        encode_help_text(writer, self.help())?;

        writer.write_str("# TYPE ")?;
        encode_prefix_name(writer, prefixes, self.name())?;
        writer.write_str(" ")?;
        writer.write_str(self.r#type().as_str())?;
        writer.write_str("\n")?;

        let labels_vec: Vec<_> = labels.collect();
        let empty: &[(&str, &str)] = &[];
        encode_metric_value(
            writer,
            self.name(),
            prefixes,
            &labels_vec,
            empty,
            &self.value(),
        )?;
        Ok(())
    }
}

impl MetricItem<'_> {
    pub(crate) fn encode_schema<'a>(
        &self,
        schema: &mut Schema,
        prefixes: &[&str],
        labels: impl Iterator<Item = (&'a str, &'a str)> + 'a,
    ) {
        schema.push(
            ItemSchema::from_label_iter(self.name(), prefixes, labels, self.r#type()),
            self.help(),
        );
    }

    fn encode_value(&self, values: &mut Values) {
        values.items.push(self.value());
    }

    pub(crate) fn encode_openmetrics<'a>(
        &self,
        writer: &mut impl Write,
        prefixes: &[impl AsRef<str>],
        labels: impl Iterator<Item = (&'a str, &'a str)> + 'a,
    ) -> fmt::Result {
        EncodableMetric::encode_openmetrics(self, writer, prefixes, labels)
    }
}

pub(crate) fn encode_u64(writer: &mut (impl Write + ?Sized), v: u64) -> fmt::Result {
    writer.write_str(itoa::Buffer::new().format(v))?;
    Ok(())
}

pub(crate) fn encode_i64(writer: &mut (impl Write + ?Sized), v: i64) -> fmt::Result {
    writer.write_str(itoa::Buffer::new().format(v))?;
    Ok(())
}

pub(crate) fn encode_f64(writer: &mut (impl Write + ?Sized), v: f64) -> fmt::Result {
    writer.write_str(ryu::Buffer::new().format(v))?;
    Ok(())
}

pub(crate) fn encode_prefix_name(
    writer: &mut (impl Write + ?Sized),
    prefixes: &[impl AsRef<str>],
    name: &str,
) -> fmt::Result {
    for prefix in prefixes {
        writer.write_str(prefix.as_ref())?;
        writer.write_str("_")?;
    }
    writer.write_str(name)?;
    Ok(())
}

/// Writes `help` followed by `".\n"` — but skips the period if the text
/// already ends with `.`, `?` or `!` (ignoring trailing newlines, which the
/// OpenMetrics escape turns into a literal `\n` token). Per the spec, `\`
/// and `\n` inside `help` are escaped to `\\` and `\n`.
pub(crate) fn encode_help_text(writer: &mut (impl Write + ?Sized), help: &str) -> fmt::Result {
    encode_escaped(writer, help, false)?;
    let trimmed = help.trim_end_matches('\n');
    if !matches!(trimmed.chars().last(), Some('.') | Some('?') | Some('!')) {
        writer.write_str(".")?;
    }
    writer.write_str("\n")
}
