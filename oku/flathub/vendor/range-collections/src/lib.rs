//! A set of non-overlapping ranges, backed by `SmallVec<T>`
#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[allow(dead_code)]
mod merge_state;

mod iterators;

pub mod range_set;

pub use range_set::{RangeSet, RangeSet2, RangeSetRef};
