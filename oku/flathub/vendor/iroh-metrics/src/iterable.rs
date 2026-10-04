//! Traits for iterating over the fields of structs.

use std::fmt;

/// Derives [`Iterable`] for a struct.
///
/// You can use this derive instead of [`MetricsGroup`] if you want to implement `Default`
/// and `MetricsGroup` manually, but still use a derived `Iterable` impl.
///
/// [`Iterable`]: ::iroh_metrics::iterable::Iterable
/// [`MetricsGroup`]: ::iroh_metrics::MetricsGroup
pub use iroh_metrics_derive::Iterable;

use crate::{FamilyItem, MetricItem};

/// Trait for iterating over the fields of a struct.
pub trait Iterable {
    /// Returns the number of metric fields in the struct.
    fn metric_field_count(&self) -> usize;
    /// Returns the metric field at the given index.
    fn metric_field_ref(&self, n: usize) -> Option<MetricItem<'_>>;
    /// Returns the number of [`Family`](crate::Family) fields in the struct.
    fn family_field_count(&self) -> usize {
        0
    }
    /// Returns the [`Family`](crate::Family) field at the given index.
    fn family_field_ref(&self, _n: usize) -> Option<FamilyItem<'_>> {
        None
    }
}

/// Helper trait to convert from `self` to `dyn Iterable`.
pub trait IntoIterable {
    /// Returns `self` as `dyn Iterable`
    fn as_iterable(&self) -> &dyn Iterable;

    /// Returns an iterator over the metric fields of the struct.
    fn field_iter(&self) -> FieldIter<'_> {
        FieldIter::new(self.as_iterable())
    }

    /// Returns an iterator over the Family fields of the struct.
    fn family_iter(&self) -> FamilyIter<'_> {
        FamilyIter::new(self.as_iterable())
    }
}

impl<T> IntoIterable for T
where
    T: Iterable,
{
    fn as_iterable(&self) -> &dyn Iterable {
        self
    }
}

/// Iterator over the fields of a struct.
///
/// Returned from [`IntoIterable::field_iter`].
pub struct FieldIter<'a> {
    pos: usize,
    inner: &'a dyn Iterable,
}

impl fmt::Debug for FieldIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FieldIter")
    }
}

impl<'a> FieldIter<'a> {
    pub(crate) fn new(inner: &'a dyn Iterable) -> Self {
        Self { pos: 0, inner }
    }
}
impl<'a> Iterator for FieldIter<'a> {
    type Item = MetricItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.inner.metric_field_count() {
            None
        } else {
            let out = self.inner.metric_field_ref(self.pos);
            self.pos += 1;
            out
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.inner.metric_field_count() - self.pos;
        (n, Some(n))
    }
}

/// Iterator over Family fields of a struct.
pub struct FamilyIter<'a> {
    pos: usize,
    inner: &'a dyn Iterable,
}

impl fmt::Debug for FamilyIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FamilyIter")
    }
}

impl<'a> FamilyIter<'a> {
    pub(crate) fn new(inner: &'a dyn Iterable) -> Self {
        Self { pos: 0, inner }
    }
}

impl<'a> Iterator for FamilyIter<'a> {
    type Item = FamilyItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.inner.family_field_count() {
            None
        } else {
            let out = self.inner.family_field_ref(self.pos);
            self.pos += 1;
            out
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.inner.family_field_count() - self.pos;
        (n, Some(n))
    }
}
