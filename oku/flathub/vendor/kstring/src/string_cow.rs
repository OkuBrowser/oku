#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
use alloc::string::String;
use alloc::{borrow::Cow, fmt};

use crate::KStringBase;
use crate::KStringRef;
use crate::KStringRefInner;

type StdString = String;
type BoxedStr = Box<str>;

/// A reference to a UTF-8 encoded, immutable string.
#[deprecated(
    since = "2.0.4",
    note = "for alternatives, see https://github.com/rosetta-rs/string-rosetta-rs"
)]
pub type KStringCow<'s> = KStringCowBase<'s, crate::backend::DefaultStr>;

/// A reference to a UTF-8 encoded, immutable string.
#[derive(Clone)]
#[repr(transparent)]
pub struct KStringCowBase<'s, B = crate::backend::DefaultStr> {
    pub(crate) inner: KStringCowInner<'s, B>,
}

#[derive(Clone)]
pub(crate) enum KStringCowInner<'s, B> {
    Borrowed(&'s str),
    Owned(KStringBase<B>),
}

impl<B> KStringCowBase<'_, B> {
    /// Create a new empty `KStringCowBase`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self::from_static("")
    }

    /// Create a reference to a `'static` data.
    #[inline]
    #[must_use]
    pub const fn from_static(other: &'static str) -> Self {
        Self {
            inner: KStringCowInner::Owned(KStringBase::from_static(other)),
        }
    }
}

impl<'s, B: crate::backend::HeapStr> KStringCowBase<'s, B> {
    /// Create an owned `KStringCowBase`.
    #[inline]
    #[must_use]
    pub fn from_boxed(other: BoxedStr) -> Self {
        Self {
            inner: KStringCowInner::Owned(KStringBase::from_boxed(other)),
        }
    }

    /// Create an owned `KStringCowBase`.
    #[inline]
    #[must_use]
    pub fn from_string(other: StdString) -> Self {
        Self {
            inner: KStringCowInner::Owned(KStringBase::from_string(other)),
        }
    }

    /// Create a reference to a borrowed data.
    #[inline]
    #[must_use]
    pub fn from_ref(other: &'s str) -> Self {
        Self {
            inner: KStringCowInner::Borrowed(other),
        }
    }

    /// Get a reference to the `KStringBase`.
    #[inline]
    #[must_use]
    pub fn as_ref(&self) -> KStringRef<'_> {
        self.inner.as_ref()
    }

    /// Clone the data into an owned-type.
    #[inline]
    #[must_use]
    pub fn into_owned(self) -> KStringBase<B> {
        self.inner.into_owned()
    }

    /// Extracts a string slice containing the entire `KStringCowBase`.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// Convert to a mutable string type, cloning the data if necessary.
    #[inline]
    #[must_use]
    pub fn into_string(self) -> StdString {
        String::from(self.into_boxed_str())
    }

    /// Convert to a mutable string type, cloning the data if necessary.
    #[inline]
    #[must_use]
    pub fn into_boxed_str(self) -> BoxedStr {
        self.inner.into_boxed_str()
    }

    /// Convert to a Cow str
    #[inline]
    #[must_use]
    pub fn into_cow_str(self) -> Cow<'s, str> {
        self.inner.into_cow_str()
    }
}

impl<'s, B: crate::backend::HeapStr> KStringCowInner<'s, B> {
    #[inline]
    fn as_ref(&self) -> KStringRef<'_> {
        match self {
            Self::Borrowed(s) => KStringRef::from_ref(s),
            Self::Owned(s) => s.as_ref(),
        }
    }

    #[inline]
    fn into_owned(self) -> KStringBase<B> {
        match self {
            Self::Borrowed(s) => KStringBase::from_ref(s),
            Self::Owned(s) => s,
        }
    }

    #[inline]
    fn as_str(&self) -> &str {
        match self {
            Self::Borrowed(s) => s,
            Self::Owned(s) => s.as_str(),
        }
    }

    #[inline]
    fn into_boxed_str(self) -> BoxedStr {
        match self {
            Self::Borrowed(s) => BoxedStr::from(s),
            Self::Owned(s) => s.into_boxed_str(),
        }
    }

    /// Convert to a Cow str
    #[inline]
    fn into_cow_str(self) -> Cow<'s, str> {
        match self {
            Self::Borrowed(s) => Cow::Borrowed(s),
            Self::Owned(s) => s.into_cow_str(),
        }
    }
}

impl<B: crate::backend::HeapStr> core::ops::Deref for KStringCowBase<'_, B> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<B: crate::backend::HeapStr> Eq for KStringCowBase<'_, B> {}

impl<'s, B: crate::backend::HeapStr> PartialEq<Self> for KStringCowBase<'s, B> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl<B: crate::backend::HeapStr> PartialEq<str> for KStringCowBase<'_, B> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl<'s, B: crate::backend::HeapStr> PartialEq<&'s str> for KStringCowBase<'s, B> {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(self.as_str(), *other)
    }
}

impl<B: crate::backend::HeapStr> PartialEq<String> for KStringCowBase<'_, B> {
    #[inline]
    fn eq(&self, other: &StdString) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl<B: crate::backend::HeapStr> Ord for KStringCowBase<'_, B> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<B: crate::backend::HeapStr> PartialOrd for KStringCowBase<'_, B> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<B: crate::backend::HeapStr> core::hash::Hash for KStringCowBase<'_, B> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl<B: crate::backend::HeapStr> fmt::Debug for KStringCowBase<'_, B> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<B: crate::backend::HeapStr> fmt::Display for KStringCowBase<'_, B> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<B: crate::backend::HeapStr> AsRef<str> for KStringCowBase<'_, B> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<B: crate::backend::HeapStr> AsRef<[u8]> for KStringCowBase<'_, B> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg(feature = "std")]
impl<B: crate::backend::HeapStr> AsRef<std::ffi::OsStr> for KStringCowBase<'_, B> {
    #[inline]
    fn as_ref(&self) -> &std::ffi::OsStr {
        (**self).as_ref()
    }
}

#[cfg(feature = "std")]
impl<B: crate::backend::HeapStr> AsRef<std::path::Path> for KStringCowBase<'_, B> {
    #[inline]
    fn as_ref(&self) -> &std::path::Path {
        std::path::Path::new(self)
    }
}

impl<B: crate::backend::HeapStr> core::borrow::Borrow<str> for KStringCowBase<'_, B> {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<B> Default for KStringCowBase<'_, B> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<B: crate::backend::HeapStr> From<KStringBase<B>> for KStringCowBase<'_, B> {
    #[inline]
    fn from(other: KStringBase<B>) -> Self {
        let inner = KStringCowInner::Owned(other);
        Self { inner }
    }
}

impl<'s, B: crate::backend::HeapStr> From<&'s KStringBase<B>> for KStringCowBase<'s, B> {
    #[inline]
    fn from(other: &'s KStringBase<B>) -> Self {
        let other = other.as_ref();
        other.into()
    }
}

impl<'s, B: crate::backend::HeapStr> From<KStringRef<'s>> for KStringCowBase<'s, B> {
    #[inline]
    fn from(other: KStringRef<'s>) -> Self {
        match other.inner {
            KStringRefInner::Borrowed(s) => Self::from_ref(s),
            KStringRefInner::Singleton(s) => Self::from_static(s),
        }
    }
}

impl<'s, B: crate::backend::HeapStr> From<&'s KStringRef<'s>> for KStringCowBase<'s, B> {
    #[inline]
    fn from(other: &'s KStringRef<'s>) -> Self {
        match other.inner {
            KStringRefInner::Borrowed(s) => Self::from_ref(s),
            KStringRefInner::Singleton(s) => Self::from_static(s),
        }
    }
}

impl<B: crate::backend::HeapStr> From<StdString> for KStringCowBase<'_, B> {
    #[inline]
    fn from(other: StdString) -> Self {
        Self::from_string(other)
    }
}

impl<'s, B: crate::backend::HeapStr> From<&'s StdString> for KStringCowBase<'s, B> {
    #[inline]
    fn from(other: &'s StdString) -> Self {
        Self::from_ref(other.as_str())
    }
}

impl<B: crate::backend::HeapStr> From<BoxedStr> for KStringCowBase<'_, B> {
    #[inline]
    fn from(other: BoxedStr) -> Self {
        // Since the memory is already allocated, don't bother moving it into a FixedString
        Self::from_boxed(other)
    }
}

impl<'s, B: crate::backend::HeapStr> From<&'s BoxedStr> for KStringCowBase<'s, B> {
    #[inline]
    fn from(other: &'s BoxedStr) -> Self {
        Self::from_ref(other)
    }
}

impl<'s, B: crate::backend::HeapStr> From<&'s str> for KStringCowBase<'s, B> {
    #[inline]
    fn from(other: &'s str) -> Self {
        Self::from_ref(other)
    }
}

impl<B: crate::backend::HeapStr> core::str::FromStr for KStringCowBase<'_, B> {
    type Err = core::convert::Infallible;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_string(s.into()))
    }
}

#[cfg(feature = "serde")]
impl<B: crate::backend::HeapStr> serde::Serialize for KStringCowBase<'_, B> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de, B: crate::backend::HeapStr> serde::Deserialize<'de> for KStringCowBase<'_, B> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        KStringBase::deserialize(deserializer).map(|s| s.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_size() {
        println!("KStringCow: {}", size_of::<KStringCow<'static>>());
    }
}
