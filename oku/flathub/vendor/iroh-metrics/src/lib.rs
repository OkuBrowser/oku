//! Metrics library for iroh

#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![cfg_attr(iroh_docsrs, feature(doc_cfg))]

pub use self::{
    base::*,
    family::{Family, FamilyEncoder, FamilyItem},
    labels::*,
    metrics::*,
    registry::*,
};

mod base;
pub mod encoding;
mod family;
pub mod iterable;
mod labels;
mod metrics;
mod registry;
#[cfg(feature = "service")]
pub mod service;
#[cfg(feature = "static_core")]
pub mod static_core;

/// Derives [`EncodeLabelSet`] for a struct.
///
/// Each field becomes a label with the field name as the key.
/// Use `#[label(name = "custom")]` to customize the label key.
/// Use `#[label(skip)]` to exclude a field from the label set.
/// Use `#[label(rename_all = "...")]` on the struct to rename all fields by
/// case rule. Supported rules: `snake_case`, `camelCase`, `PascalCase`,
/// `SCREAMING_SNAKE_CASE`, `kebab-case`, `lowercase`, `UPPERCASE`.
///
/// Field types must implement [`EncodeLabelValue`]. Out of the box this
/// covers `String`, `&'static str`, the integer types, and `bool`.
///
/// The struct must also derive `Clone`, `Hash`, `PartialEq`, and `Eq`.
/// To use the label set with [`Family`], also derive `PartialOrd` and `Ord`
/// (the encoder produces output sorted by label set).
///
/// # Example
///
/// ```
/// use iroh_metrics::EncodeLabelSet;
///
/// #[derive(Clone, Hash, PartialEq, Eq, EncodeLabelSet)]
/// #[label(rename_all = "kebab-case")]
/// struct HttpLabels {
///     method: String,
///     #[label(name = "status_code")]
///     status: u16,
/// }
/// ```
pub use iroh_metrics_derive::EncodeLabelSet;
/// Derives [`EncodeLabelValue`] for an enum with only unit variants.
///
/// Each variant becomes a string label; default casing is `snake_case`.
/// Use `#[label(rename_all = "...")]` on the enum or `#[label(name = "...")]`
/// on a variant to customize. See [`macro@EncodeLabelSet`] for the list of
/// supported `rename_all` values.
pub use iroh_metrics_derive::EncodeLabelValue;
/// Derives [`MetricsGroup`] and [`Iterable`].
///
/// This derive macro only works on structs with named fields.
///
/// It will generate a [`Default`] impl which expects all fields to be of a type
/// that has a public `new` method taking a single `&'static str` argument.
/// The [`Default::default`] method will call each field's `new` method with the
/// first line of the field's doc comment as argument. Alternatively, you can override
/// the value passed to `new` by setting a `#[metrics(help = "my help")]`
/// attribute on the field.
///
/// It will also generate a [`MetricsGroup`] impl. By default, the struct's name,
/// converted to `camel_case` will be used as the return value of the [`MetricsGroup::name`]
/// method. The name can be customized by setting a `#[metrics(name = "my-name")]` attribute.
///
/// It will also generate a [`Iterable`] impl. Fields with the `Family<_, _>`
/// type are routed through [`Iterable::family_field_ref`] instead of
/// [`Iterable::metric_field_ref`]. Detection inspects the last segment of
/// the field type, so `iroh_metrics::Family<L, M>` is recognized but a type
/// alias is not — annotate the field with `#[metrics(family)]` in that case.
///
/// [`Iterable`]: iterable::Iterable
/// [`Iterable::metric_field_ref`]: iterable::Iterable::metric_field_ref
/// [`Iterable::family_field_ref`]: iterable::Iterable::family_field_ref
pub use iroh_metrics_derive::MetricsGroup;
/// Derives [`MetricsGroupSet`] for a struct.
///
/// All fields of the struct must be public and have a type of `Arc<SomeType>`,
/// where `SomeType` implements `MetricsGroup`.
pub use iroh_metrics_derive::MetricsGroupSet;

// This lets us use the derive metrics in the lib tests within this crate.
extern crate self as iroh_metrics;

use std::collections::HashMap;

/// Potential errors from this library.
#[n0_error::stack_error(derive, add_meta, from_sources, std_sources)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    /// Indicates that the metrics have not been enabled.
    #[error("Metrics not enabled")]
    NoMetrics,
    /// Writing the metrics to the output buffer failed.
    #[error(transparent)]
    Fmt { source: std::fmt::Error },
    /// Any IO related error.
    #[error(transparent)]
    IO { source: std::io::Error },
}

/// Parses Prometheus metrics from a string.
pub fn parse_prometheus_metrics(data: &str) -> HashMap<String, f64> {
    let mut metrics = HashMap::new();
    for line in data.lines() {
        if line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let metric = parts[0];
        let value = parts[1].parse::<f64>();
        if value.is_err() {
            continue;
        }
        metrics.insert(metric.to_string(), value.unwrap());
    }
    metrics
}
