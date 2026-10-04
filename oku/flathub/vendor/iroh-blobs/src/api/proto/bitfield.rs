use std::{cmp::Ordering, num::NonZeroU64};

use bao_tree::{ChunkNum, ChunkRanges};
use range_collections::range_set::RangeSetRange;
use serde::{Deserialize, Deserializer, Serialize};
use smallvec::SmallVec;

use crate::store::util::{
    observer::{Combine, CombineInPlace},
    RangeSetExt,
};

pub(crate) fn is_validated(size: NonZeroU64, ranges: &ChunkRanges) -> bool {
    let size = size.get();
    // ChunkNum::chunks will be at least 1, so this is safe.
    let last_chunk = ChunkNum::chunks(size) - 1;
    ranges.contains(&last_chunk)
}

pub fn is_complete(size: NonZeroU64, ranges: &ChunkRanges) -> bool {
    let complete = ChunkRanges::from(..ChunkNum::chunks(size.get()));
    // is_subset is a bit weirdly named. This means that complete is a subset of ranges.
    complete.is_subset(ranges)
}

/// The state of a bitfield, or an update to a bitfield
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Bitfield {
    /// Possible update to the size information. can this be just a u64?
    pub(crate) size: u64,
    /// The ranges that were added
    pub ranges: ChunkRanges,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct BitfieldState {
    pub complete: bool,
    pub validated_size: Option<u64>,
}

impl Serialize for Bitfield {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut numbers = SmallVec::<[_; 4]>::new();
        numbers.push(self.size);
        numbers.extend(self.ranges.boundaries().iter().map(|x| x.0));
        numbers.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bitfield {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut numbers: SmallVec<[u64; 4]> = SmallVec::deserialize(deserializer)?;

        // Need at least 1 u64 for size
        if numbers.is_empty() {
            return Err(serde::de::Error::custom("Bitfield needs at least size"));
        }

        // Split: first is size, rest are ranges
        let size = numbers.remove(0);
        let mut ranges = SmallVec::<[_; 2]>::new();
        for num in numbers {
            ranges.push(ChunkNum(num));
        }
        let Some(ranges) = ChunkRanges::new(ranges) else {
            return Err(serde::de::Error::custom("Boundaries are not sorted"));
        };
        Ok(Bitfield::new(ranges, size))
    }
}

impl Bitfield {
    #[cfg(feature = "fs-store")]
    pub(crate) fn new_unchecked(ranges: ChunkRanges, size: u64) -> Self {
        Self { ranges, size }
    }

    pub fn new(mut ranges: ChunkRanges, size: u64) -> Self {
        // for zero size, we have to trust the caller
        if let Some(size) = NonZeroU64::new(size) {
            let end = ChunkNum::chunks(size.get());
            if ChunkRanges::from(..end).is_subset(&ranges) {
                // complete bitfield, canonicalize to all
                ranges = ChunkRanges::all();
            } else if ranges.contains(&(end - 1)) {
                // validated bitfield, canonicalize to open end
                ranges |= ChunkRanges::from(end..);
            }
        }
        Self { ranges, size }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    /// An empty bitfield. This is the neutral element for the combine operation.
    pub fn empty() -> Self {
        Self {
            ranges: ChunkRanges::empty(),
            size: 0,
        }
    }

    /// Create a complete bitfield for the given size
    pub fn complete(size: u64) -> Self {
        Self {
            ranges: ChunkRanges::all(),
            size,
        }
    }

    /// True if the chunk corresponding to the size is included in the ranges
    pub fn is_validated(&self) -> bool {
        if let Some(size) = NonZeroU64::new(self.size) {
            is_validated(size, &self.ranges)
        } else {
            self.ranges.is_all()
        }
    }

    /// True if all chunks up to size are included in the ranges
    pub fn is_complete(&self) -> bool {
        if let Some(size) = NonZeroU64::new(self.size) {
            is_complete(size, &self.ranges)
        } else {
            self.ranges.is_all()
        }
    }

    /// Get the validated size if the bitfield is validated
    pub fn validated_size(&self) -> Option<u64> {
        if self.is_validated() {
            Some(self.size)
        } else {
            None
        }
    }

    /// Total valid bytes in this bitfield.
    pub fn total_bytes(&self) -> u64 {
        let mut total = 0;
        for range in self.ranges.iter() {
            let (start, end) = match range {
                RangeSetRange::Range(range) => {
                    (range.start.to_bytes(), range.end.to_bytes().min(self.size))
                }
                RangeSetRange::RangeFrom(range) => (range.start.to_bytes(), self.size),
            };
            total += end - start;
        }
        total
    }

    pub fn state(&self) -> BitfieldState {
        BitfieldState {
            complete: self.is_complete(),
            validated_size: self.validated_size(),
        }
    }

    /// Update the bitfield with a new value, and gives detailed information about the change.
    ///
    /// returns a tuple of (changed, Some((old, new))). If the bitfield changed at all, the flag
    /// is true. If there was a significant change, the old and new states are returned.
    pub fn update(&mut self, update: &Bitfield) -> UpdateResult {
        let s0 = self.state();
        // todo: this is very inefficient because of all the clones
        let bitfield1 = self.clone().combine(update.clone());
        if bitfield1 != *self {
            let s1 = bitfield1.state();
            *self = bitfield1;
            if s0 != s1 {
                UpdateResult::MajorChange(s0, s1)
            } else {
                UpdateResult::MinorChange(s1)
            }
        } else {
            UpdateResult::NoChange(s0)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty() && self.size == 0
    }

    /// A diff between two bitfields. This is an inverse of the combine operation.
    pub fn diff(&self, that: &Bitfield) -> Self {
        let size = choose_size(self, that);
        let ranges_diff = &self.ranges ^ &that.ranges;
        Self {
            ranges: ranges_diff,
            size,
        }
    }
}

pub fn choose_size(a: &Bitfield, b: &Bitfield) -> u64 {
    match (a.ranges.upper_bound(), b.ranges.upper_bound()) {
        (Some(ac), Some(bc)) => match ac.cmp(&bc) {
            Ordering::Less => b.size,
            Ordering::Greater => a.size,
            Ordering::Equal => a.size.max(b.size),
        },
        (Some(_), None) => b.size,
        (None, Some(_)) => a.size,
        (None, None) => a.size.max(b.size),
    }
}

impl Combine for Bitfield {
    fn combine(self, that: Self) -> Self {
        // the size of the chunk with the larger last chunk wins
        let size = choose_size(&self, &that);
        let ranges = self.ranges | that.ranges;
        Self::new(ranges, size)
    }
}

impl CombineInPlace for Bitfield {
    fn combine_with(&mut self, other: Self) -> Self {
        let new = &other.ranges - &self.ranges;
        if new.is_empty() {
            return Bitfield::empty();
        }
        self.ranges.union_with(&new);
        self.size = choose_size(self, &other);
        Bitfield {
            ranges: new,
            size: self.size,
        }
    }

    fn is_neutral(&self) -> bool {
        self.ranges.is_empty() && self.size == 0
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
pub enum UpdateResult {
    NoChange(BitfieldState),
    MinorChange(BitfieldState),
    MajorChange(BitfieldState, BitfieldState),
}

impl UpdateResult {
    pub fn new_state(&self) -> &BitfieldState {
        match self {
            UpdateResult::NoChange(new) => new,
            UpdateResult::MinorChange(new) => new,
            UpdateResult::MajorChange(_, new) => new,
        }
    }

    /// True if this change went from non-validated to validated
    pub fn was_validated(&self) -> bool {
        match self {
            UpdateResult::NoChange(_) => false,
            UpdateResult::MinorChange(_) => false,
            UpdateResult::MajorChange(old, new) => {
                new.validated_size.is_some() && old.validated_size.is_none()
            }
        }
    }

    pub fn changed(&self) -> bool {
        match self {
            UpdateResult::NoChange(_) => false,
            UpdateResult::MinorChange(_) => true,
            UpdateResult::MajorChange(_, _) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use bao_tree::{ChunkNum, ChunkRanges};
    use proptest::prelude::{prop, Strategy};
    use smallvec::SmallVec;
    use test_strategy::proptest;

    use super::Bitfield;
    use crate::store::util::observer::{Combine, CombineInPlace};

    fn gen_chunk_ranges(max: ChunkNum, k: usize) -> impl Strategy<Value = ChunkRanges> {
        prop::collection::btree_set(0..=max.0, 0..=k).prop_map(|vec| {
            let bounds = vec.into_iter().map(ChunkNum).collect::<SmallVec<[_; 2]>>();
            ChunkRanges::new(bounds).unwrap()
        })
    }

    fn gen_bitfields(size: u64, k: usize) -> impl Strategy<Value = Bitfield> {
        (0..size).prop_flat_map(move |size| {
            let chunks = ChunkNum::full_chunks(size);
            gen_chunk_ranges(chunks, k).prop_map(move |ranges| Bitfield::new(ranges, size))
        })
    }

    fn gen_non_empty_bitfields(size: u64, k: usize) -> impl Strategy<Value = Bitfield> {
        gen_bitfields(size, k).prop_filter("non-empty", |x| !x.is_neutral())
    }

    #[proptest]
    fn test_combine_empty(#[strategy(gen_non_empty_bitfields(32768, 4))] a: Bitfield) {
        assert_eq!(a.clone().combine(Bitfield::empty()), a);
        assert_eq!(Bitfield::empty().combine(a.clone()), a);
    }

    #[proptest]
    fn test_combine_order(
        #[strategy(gen_non_empty_bitfields(32768, 4))] a: Bitfield,
        #[strategy(gen_non_empty_bitfields(32768, 4))] b: Bitfield,
    ) {
        let ab = a.clone().combine(b.clone());
        let ba = b.combine(a);
        assert_eq!(ab, ba);
    }

    #[proptest]
    fn test_complete_normalized(#[strategy(gen_non_empty_bitfields(32768, 4))] a: Bitfield) {
        if a.is_complete() {
            assert_eq!(a.ranges, ChunkRanges::all());
        }
    }
}
