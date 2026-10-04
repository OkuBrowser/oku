//! Label types for metric families.

use std::{borrow::Cow, hash::Hash};

/// A label value that can be encoded as a string for OpenMetrics output.
#[derive(Debug, Clone, PartialEq)]
pub enum LabelValue<'a> {
    /// String value.
    Str(Cow<'a, str>),
    /// Signed integer.
    Int(i64),
    /// Unsigned integer.
    Uint(u64),
    /// Boolean.
    Bool(bool),
}

impl LabelValue<'_> {
    /// Converts to string representation for encoding.
    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            LabelValue::Str(s) => Cow::Borrowed(s.as_ref()),
            LabelValue::Int(v) => Cow::Owned(v.to_string()),
            LabelValue::Uint(v) => Cow::Owned(v.to_string()),
            LabelValue::Bool(v) => Cow::Borrowed(if *v { "true" } else { "false" }),
        }
    }
}

impl<'a> From<&'a str> for LabelValue<'a> {
    fn from(s: &'a str) -> Self {
        LabelValue::Str(Cow::Borrowed(s))
    }
}

impl From<String> for LabelValue<'static> {
    fn from(s: String) -> Self {
        LabelValue::Str(Cow::Owned(s))
    }
}

macro_rules! impl_from_int {
    ($($t:ty => $variant:ident),*) => {
        $(
            impl From<$t> for LabelValue<'static> {
                fn from(v: $t) -> Self {
                    LabelValue::$variant(v as _)
                }
            }
        )*
    };
}

impl_from_int!(
    i64 => Int, i32 => Int, i16 => Int, i8 => Int,
    u64 => Uint, u32 => Uint, u16 => Uint, u8 => Uint
);

impl From<bool> for LabelValue<'static> {
    fn from(v: bool) -> Self {
        LabelValue::Bool(v)
    }
}

/// A key-value label pair.
pub type LabelPair<'a> = (&'static str, LabelValue<'a>);

/// Encodes a single field as a [`LabelValue`].
///
/// Implemented for the standard label-supported types so the
/// `#[derive(EncodeLabelSet)]` macro can borrow string fields without
/// allocating on each scrape.
pub trait EncodeLabelValue {
    /// Borrows or copies `self` into a [`LabelValue`].
    fn encode_label_value(&self) -> LabelValue<'_>;
}

impl EncodeLabelValue for str {
    fn encode_label_value(&self) -> LabelValue<'_> {
        LabelValue::Str(Cow::Borrowed(self))
    }
}

impl EncodeLabelValue for String {
    fn encode_label_value(&self) -> LabelValue<'_> {
        LabelValue::Str(Cow::Borrowed(self.as_str()))
    }
}

impl<T: EncodeLabelValue + ?Sized> EncodeLabelValue for &T {
    fn encode_label_value(&self) -> LabelValue<'_> {
        T::encode_label_value(self)
    }
}

impl EncodeLabelValue for bool {
    fn encode_label_value(&self) -> LabelValue<'_> {
        LabelValue::Bool(*self)
    }
}

macro_rules! impl_encode_label_value_int {
    ($($t:ty => $variant:ident),*) => {
        $(
            impl EncodeLabelValue for $t {
                fn encode_label_value(&self) -> LabelValue<'_> {
                    LabelValue::$variant(*self as _)
                }
            }
        )*
    };
}

impl_encode_label_value_int!(
    i64 => Int, i32 => Int, i16 => Int, i8 => Int,
    u64 => Uint, u32 => Uint, u16 => Uint, u8 => Uint
);

/// Trait for types that can be encoded as a set of labels.
///
/// Implement this for label structs to use with [`Family`](crate::Family).
/// The struct must also implement `Clone + Hash + Eq + Send + Sync`.
pub trait EncodeLabelSet: Hash + Eq + Clone + Send + Sync + 'static {
    /// Returns the labels as key-value pairs.
    fn encode_label_pairs(&self) -> Vec<LabelPair<'_>>;
}

/// Empty label set for metrics that don't need labels.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct NoLabels;

impl EncodeLabelSet for NoLabels {
    fn encode_label_pairs(&self) -> Vec<LabelPair<'_>> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EncodeLabelSet;

    #[test]
    fn test_label_value_as_str() {
        assert_eq!(LabelValue::from("hello").as_str(), "hello");
        assert_eq!(LabelValue::from(42i64).as_str(), "42");
        assert_eq!(LabelValue::from(42u64).as_str(), "42");
        assert_eq!(LabelValue::from(true).as_str(), "true");
        assert_eq!(LabelValue::from(false).as_str(), "false");
    }

    #[test]
    fn test_no_labels() {
        assert!(NoLabels.encode_label_pairs().is_empty());
    }

    #[test]
    fn test_encode_label_value() {
        // Strings must borrow (no per-scrape alloc).
        let s = String::from("hello");
        assert!(matches!(
            s.encode_label_value(),
            LabelValue::Str(Cow::Borrowed("hello"))
        ));
        assert!(matches!(42u64.encode_label_value(), LabelValue::Uint(42)));
        assert!(matches!((-7i32).encode_label_value(), LabelValue::Int(-7)));
        assert!(matches!(true.encode_label_value(), LabelValue::Bool(true)));
    }

    #[test]
    fn test_derive_encode_label_set() {
        // Covers field types, `name`, `skip`, and the borrowed-string guarantee.
        #[derive(Clone, Hash, PartialEq, Eq, crate::EncodeLabelSet)]
        struct MyLabels {
            method: String,
            kind: &'static str,
            #[label(name = "status_code")]
            status: u16,
            count: i64,
            #[label(skip)]
            _internal: u64,
        }

        let labels = MyLabels {
            method: "GET".into(),
            kind: "x",
            status: 200,
            count: -3,
            _internal: 999,
        };
        let pairs = labels.encode_label_pairs();
        assert_eq!(pairs.len(), 4);
        assert_eq!(pairs[0], ("method", LabelValue::from("GET")));
        assert_eq!(pairs[1].0, "kind");
        assert_eq!(pairs[1].1.as_str(), "x");
        assert_eq!(pairs[2].0, "status_code");
        assert_eq!(pairs[2].1.as_str(), "200");
        assert_eq!(pairs[3].1.as_str(), "-3");
        // String label must be borrowed.
        assert!(matches!(&pairs[0].1, LabelValue::Str(Cow::Borrowed(_))));
    }

    #[test]
    fn test_derive_encode_label_value_enum() {
        // Default: snake_case for variants. `#[label(name = ...)]` overrides.
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, crate::EncodeLabelValue)]
        enum Kind {
            Ipv4,
            Ipv6,
            #[label(name = "loopback")]
            Local,
        }

        // Enum-level rename_all also works.
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, crate::EncodeLabelValue)]
        #[label(rename_all = "kebab-case")]
        enum Mode {
            FastPath,
            SlowPath,
        }

        assert_eq!(Kind::Ipv4.encode_label_value().as_str(), "ipv4");
        assert_eq!(Kind::Ipv6.encode_label_value().as_str(), "ipv6");
        assert_eq!(Kind::Local.encode_label_value().as_str(), "loopback");
        assert_eq!(Mode::FastPath.encode_label_value().as_str(), "fast-path");
        assert_eq!(Mode::SlowPath.encode_label_value().as_str(), "slow-path");
    }

    #[test]
    fn test_derive_rename_all() {
        // `rename_all` applies to fields without explicit `#[label(name)]`.
        #[derive(Clone, Hash, PartialEq, Eq, crate::EncodeLabelSet)]
        #[label(rename_all = "kebab-case")]
        struct Kebab {
            method_name: String,
            #[label(name = "literal")]
            status_code: u16,
        }
        #[derive(Clone, Hash, PartialEq, Eq, crate::EncodeLabelSet)]
        #[label(rename_all = "PascalCase")]
        struct Pascal {
            api_method: String,
        }

        let kebab = Kebab {
            method_name: "GET".into(),
            status_code: 200,
        };
        let k = kebab.encode_label_pairs();
        assert_eq!(k[0].0, "method-name");
        assert_eq!(k[1].0, "literal");

        let pascal = Pascal {
            api_method: "POST".into(),
        };
        let p = pascal.encode_label_pairs();
        assert_eq!(p[0].0, "ApiMethod");
    }
}
