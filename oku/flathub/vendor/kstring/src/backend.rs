#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
use alloc::string::String;

#[cfg(feature = "arc")]
pub(crate) type DefaultStr = ArcStr;
#[cfg(not(feature = "arc"))]
pub(crate) type DefaultStr = BoxedStr;

/// Fast allocations, O(n) clones
pub type BoxedStr = Box<str>;
static_assertions::assert_eq_size!(DefaultStr, BoxedStr);

/// Cross-thread, O(1) clones
pub type ArcStr = alloc::sync::Arc<str>;
static_assertions::assert_eq_size!(DefaultStr, ArcStr);

/// O(1) clones
pub type RcStr = alloc::rc::Rc<str>;
static_assertions::assert_eq_size!(DefaultStr, RcStr);

/// Abstract over different type of heap-allocated strings
pub trait HeapStr: core::fmt::Debug + Clone + private::Sealed {
    fn from_str(other: &str) -> Self;
    fn from_string(other: String) -> Self;
    fn from_boxed_str(other: BoxedStr) -> Self;
    fn as_str(&self) -> &str;
}

impl HeapStr for BoxedStr {
    #[inline]
    fn from_str(other: &str) -> Self {
        other.into()
    }

    #[inline]
    fn from_string(other: String) -> Self {
        other.into_boxed_str()
    }

    #[inline]
    fn from_boxed_str(other: BoxedStr) -> Self {
        other
    }

    #[inline]
    fn as_str(&self) -> &str {
        self
    }
}

impl HeapStr for ArcStr {
    #[inline]
    fn from_str(other: &str) -> Self {
        other.into()
    }

    #[inline]
    fn from_string(other: String) -> Self {
        other.into_boxed_str().into()
    }

    #[inline]
    fn from_boxed_str(other: BoxedStr) -> Self {
        other.into()
    }

    #[inline]
    fn as_str(&self) -> &str {
        self
    }
}

impl HeapStr for RcStr {
    #[inline]
    fn from_str(other: &str) -> Self {
        other.into()
    }

    #[inline]
    fn from_string(other: String) -> Self {
        other.into_boxed_str().into()
    }

    #[inline]
    fn from_boxed_str(other: BoxedStr) -> Self {
        other.into()
    }

    #[inline]
    fn as_str(&self) -> &str {
        self
    }
}

pub(crate) mod private {
    pub trait Sealed {}
    impl Sealed for super::BoxedStr {}
    impl Sealed for super::ArcStr {}
    impl Sealed for super::RcStr {}
}
