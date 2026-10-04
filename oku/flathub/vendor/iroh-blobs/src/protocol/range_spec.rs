//! Specifications for ranges selection in blobs and sequences of blobs.
//!
//! The [`ChunkRanges`] allows specifying which BAO chunks inside a single blob should be
//! selected.
//!
//! The [`ChunkRangesSeq`] builds on top of this to select blob chunks in an entire
//! collection.
use std::{fmt, sync::OnceLock};

pub use bao_tree::ChunkRanges;
use bao_tree::{ChunkNum, ChunkRangesRef};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use crate::protocol::ChunkRangesExt;

static CHUNK_RANGES_EMPTY: OnceLock<ChunkRanges> = OnceLock::new();

fn chunk_ranges_empty() -> &'static ChunkRanges {
    CHUNK_RANGES_EMPTY.get_or_init(ChunkRanges::empty)
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(from = "wire::RangeSpecSeq", into = "wire::RangeSpecSeq")]
pub struct ChunkRangesSeq(pub(crate) SmallVec<[(u64, ChunkRanges); 2]>);

impl std::hash::Hash for ChunkRangesSeq {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (i, r) in &self.0 {
            i.hash(state);
            r.boundaries().hash(state);
        }
    }
}

impl std::ops::Index<u64> for ChunkRangesSeq {
    type Output = ChunkRanges;

    fn index(&self, index: u64) -> &Self::Output {
        match self.0.binary_search_by(|(o, _)| o.cmp(&index)) {
            Ok(i) => &self.0[i].1,
            Err(i) => {
                if i == 0 {
                    chunk_ranges_empty()
                } else {
                    &self.0[i - 1].1
                }
            }
        }
    }
}

impl ChunkRangesSeq {
    pub const fn empty() -> Self {
        Self(SmallVec::new_const())
    }

    /// Request just the first blob.
    pub fn root() -> Self {
        let mut inner = SmallVec::new();
        inner.push((0, ChunkRanges::all()));
        inner.push((1, ChunkRanges::empty()));
        Self(inner)
    }

    /// A [`ChunkRangesSeq`] containing all chunks from all blobs.
    ///
    /// [`ChunkRangesSeq::iter`], will return a full range forever.
    pub fn all() -> Self {
        let mut inner = SmallVec::new();
        inner.push((0, ChunkRanges::all()));
        Self(inner)
    }

    /// A [`ChunkRangesSeq`] getting the verified size for the first blob.
    pub fn verified_size() -> Self {
        let mut inner = SmallVec::new();
        inner.push((0, ChunkRanges::last_chunk()));
        inner.push((1, ChunkRanges::empty()));
        Self(inner)
    }

    /// A [`ChunkRangesSeq`] getting the entire first blob and verified sizes for all others.
    pub fn verified_child_sizes() -> Self {
        let mut inner = SmallVec::new();
        inner.push((0, ChunkRanges::all()));
        inner.push((1, ChunkRanges::last_chunk()));
        Self(inner)
    }

    /// Checks if this [`ChunkRangesSeq`] does not select any chunks in the blob.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Checks if this [`ChunkRangesSeq`] selects all chunks in the blob.
    pub fn is_all(&self) -> bool {
        if self.0.len() != 1 {
            return false;
        }
        let Some((_, ranges)) = self.0.iter().next() else {
            return false;
        };
        ranges.is_all()
    }

    /// If this range seq describes a range for a single item, returns the offset
    /// and range spec for that item
    pub fn as_single(&self) -> Option<(u64, &ChunkRanges)> {
        // we got two elements,
        // the first element starts at offset 0,
        // and the second element is empty
        if self.0.len() != 2 {
            return None;
        }
        let (o1, v1) = self.0.iter().next().unwrap();
        let (o2, v2) = self.0.iter().next_back().unwrap();
        if *o1 == (o2 - 1) && v2.is_empty() {
            Some((*o1, v1))
        } else {
            None
        }
    }

    pub fn is_blob(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self.as_single() {
            Some((0, _)) => true,
            _ => false,
        }
    }

    /// Convenience function to create a [`ChunkRangesSeq`] from an iterator of
    /// chunk ranges. If the last element is non-empty, it will be repeated
    /// forever.
    pub fn from_ranges_infinite(ranges: impl IntoIterator<Item = ChunkRanges>) -> Self {
        let (ranges, _) = from_ranges_inner(ranges);
        Self(ranges)
    }

    /// Convenience function to create a [`ChunkRangesSeq`] from an iterator of
    /// chunk ranges. If the last element is non-empty, an empty range will be
    /// added immediately after it to terminate the sequence.
    pub fn from_ranges(ranges: impl IntoIterator<Item = ChunkRanges>) -> Self {
        let (mut res, next) = from_ranges_inner(ranges);
        if let Some((_, r)) = res.iter().next_back() {
            if !r.is_empty() {
                res.push((next, ChunkRanges::empty()));
            }
        }
        Self(res)
    }

    /// An iterator over blobs in the sequence with a non-empty range spec.
    ///
    /// This iterator will only yield items for blobs which have at least one chunk
    /// selected.
    ///
    /// This iterator is infinite if the [`ChunkRangesSeq`] ends on a non-empty [`ChunkRanges`],
    /// that is all further blobs have selected chunks spans.
    pub fn iter_non_empty_infinite(&self) -> NonEmptyRequestRangeSpecIter<'_> {
        NonEmptyRequestRangeSpecIter::new(self.iter_infinite())
    }

    /// True if this range spec sequence repeats the last range spec forever.
    pub fn is_infinite(&self) -> bool {
        self.0
            .iter()
            .next_back()
            .map(|(_, v)| !v.is_empty())
            .unwrap_or_default()
    }

    pub fn iter_infinite(&self) -> ChunkRangesSeqIterInfinite<'_> {
        ChunkRangesSeqIterInfinite {
            current: chunk_ranges_empty(),
            offset: 0,
            remaining: self.0.iter().peekable(),
        }
    }

    pub fn iter(&self) -> ChunkRangesSeqIter<'_> {
        ChunkRangesSeqIter {
            current: chunk_ranges_empty(),
            offset: 0,
            remaining: self.0.iter().peekable(),
        }
    }
}

fn from_ranges_inner(
    ranges: impl IntoIterator<Item = ChunkRanges>,
) -> (SmallVec<[(u64, ChunkRanges); 2]>, u64) {
    let mut res = SmallVec::new();
    let mut i = 0;
    for range in ranges.into_iter() {
        if range
            != res
                .iter()
                .next_back()
                .map(|(_, v)| v)
                .unwrap_or(&ChunkRanges::empty())
        {
            res.push((i, range));
        }
        i += 1;
    }
    (res, i)
}

/// An infinite iterator yielding [`RangeSpec`]s for each blob in a sequence.
///
/// The first item yielded is the [`RangeSpec`] for the first blob in the sequence, the
/// next item is the [`RangeSpec`] for the next blob, etc.
#[derive(Debug)]
pub struct ChunkRangesSeqIterInfinite<'a> {
    /// current value
    current: &'a ChunkRanges,
    /// current offset
    offset: u64,
    /// remaining ranges
    remaining: std::iter::Peekable<std::slice::Iter<'a, (u64, ChunkRanges)>>,
}

impl<'a> ChunkRangesSeqIterInfinite<'a> {
    /// True if we are at the end of the iterator.
    ///
    /// This does not mean that the iterator is terminated, it just means that
    /// it will repeat the same value forever.
    pub fn is_at_end(&mut self) -> bool {
        self.remaining.peek().is_none()
    }
}

impl<'a> Iterator for ChunkRangesSeqIterInfinite<'a> {
    type Item = &'a ChunkRanges;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.remaining.peek() {
                Some((offset, _)) if self.offset < *offset => {
                    // emit current value until we reach the next offset
                    self.offset += 1;
                    return Some(self.current);
                }
                None => {
                    // no more values, just repeat current forever
                    self.offset += 1;
                    return Some(self.current);
                }
                Some((_, ranges)) => {
                    // get next current value, new count, and set remaining
                    self.current = ranges;
                    self.remaining.next();
                }
            }
        }
    }
}

/// An infinite iterator yielding [`RangeSpec`]s for each blob in a sequence.
///
/// The first item yielded is the [`RangeSpec`] for the first blob in the sequence, the
/// next item is the [`RangeSpec`] for the next blob, etc.
#[derive(Debug)]
pub struct ChunkRangesSeqIter<'a> {
    /// current value
    current: &'a ChunkRanges,
    /// current offset
    offset: u64,
    /// remaining ranges
    remaining: std::iter::Peekable<std::slice::Iter<'a, (u64, ChunkRanges)>>,
}

impl<'a> Iterator for ChunkRangesSeqIter<'a> {
    type Item = &'a ChunkRanges;

    fn next(&mut self) -> Option<Self::Item> {
        match self.remaining.peek()? {
            (offset, _) if self.offset < *offset => {
                // emit current value until we reach the next offset
                self.offset += 1;
                Some(self.current)
            }
            (_, ranges) => {
                // get next current value, new count, and set remaining
                self.current = ranges;
                self.remaining.next();
                self.offset += 1;
                Some(self.current)
            }
        }
    }
}

/// An iterator over blobs in the sequence with a non-empty range specs.
///
/// default is what to use if the children of this RequestRangeSpec are empty.
#[derive(Debug)]
pub struct NonEmptyRequestRangeSpecIter<'a> {
    inner: ChunkRangesSeqIterInfinite<'a>,
    count: u64,
}

impl<'a> NonEmptyRequestRangeSpecIter<'a> {
    fn new(inner: ChunkRangesSeqIterInfinite<'a>) -> Self {
        Self { inner, count: 0 }
    }

    pub(crate) fn offset(&self) -> u64 {
        self.count
    }

    pub fn is_at_end(&mut self) -> bool {
        self.inner.is_at_end()
    }
}

impl<'a> Iterator for NonEmptyRequestRangeSpecIter<'a> {
    type Item = (u64, &'a ChunkRanges);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // unwrapping is safe because we know that the inner iterator will never terminate
            let curr = self.inner.next().unwrap();
            let count = self.count;
            // increase count in any case until we are at the end of possible u64 values
            // we are unlikely to ever reach this limit.
            self.count = self.count.checked_add(1)?;
            // yield only if the current value is non-empty
            if !curr.is_empty() {
                break Some((count, curr));
            } else if self.inner.is_at_end() {
                // terminate instead of looping until we run out of u64 values
                break None;
            }
        }
    }
}

/// A chunk range specification as a sequence of chunk offsets.
///
/// This is just the wire encoding of a [`ChunkRanges`]. You should rarely have to
/// interact with this directly.
///
/// Offsets encode alternating spans starting on 0, where the first span is always
/// deselected.
///
/// ## Examples:
///
/// - `[2, 5, 3, 1]` encodes five spans, of which two are selected:
///   - `[0, 0+2) = [0, 2)` is not selected.
///   - `[2, 2+5) = [2, 7)` is selected.
///   - `[7, 7+3) = [7, 10)` is not selected.
///   - `[10, 10+1) = [10, 11)` is selected.
///   - `[11, inf)` is deselected.
///
///   Such a [`RangeSpec`] can be converted to a [`ChunkRanges`] using containing just the
///   selected ranges: `ChunkRanges{2..7, 10..11}` using [`RangeSpec::to_chunk_ranges`].
///
/// - An empty range selects no spans, encoded as `[]`. This means nothing of the blob is
///   selected.
///
/// - To select an entire blob create a single half-open span starting at the first chunk:
///   `[0]`.
///
/// - To select the tail of a blob, create a single half-open span: `[15]`.
///
/// This is a SmallVec so we can avoid allocations for the very common case of a single
/// chunk range.
#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Hash)]
#[repr(transparent)]
pub struct RangeSpec(SmallVec<[u64; 2]>);

impl RangeSpec {
    /// Creates a new [`RangeSpec`] from a range set.
    pub fn new(ranges: impl AsRef<ChunkRangesRef>) -> Self {
        let ranges = ranges.as_ref().boundaries();
        let mut res = SmallVec::new();
        if let Some((start, rest)) = ranges.split_first() {
            let mut prev = start.0;
            res.push(prev);
            for v in rest {
                res.push(v.0 - prev);
                prev = v.0;
            }
        }
        Self(res)
    }

    /// A [`RangeSpec`] selecting nothing from the blob.
    ///
    /// This is called "empty" because the representation is an empty set.
    pub const EMPTY: Self = Self(SmallVec::new_const());

    /// Creates a [`RangeSpec`] selecting the entire blob.
    pub fn all() -> Self {
        Self(smallvec![0])
    }

    /// Creates a [`RangeSpec`] selecting the last chunk, which is also a size proof.
    pub fn verified_size() -> Self {
        Self(smallvec![u64::MAX])
    }

    /// Checks if this [`RangeSpec`] does not select any chunks in the blob.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Checks if this [`RangeSpec`] selects all chunks in the blob.
    pub fn is_all(&self) -> bool {
        self.0.len() == 1 && self.0[0] == 0
    }

    /// Returns the number of chunks selected by this [`RangeSpec`], as a tuple
    /// with the minimum and maximum number of chunks.
    pub fn chunks(&self) -> (u64, Option<u64>) {
        let mut min = 0;
        for i in 0..self.0.len() / 2 {
            min += self.0[2 * i + 1];
        }
        let max = if !self.0.len().is_multiple_of(2) {
            // spec is open ended
            None
        } else {
            Some(min)
        };
        (min, max)
    }

    /// Creates a [`ChunkRanges`] from this [`RangeSpec`].
    pub fn to_chunk_ranges(&self) -> ChunkRanges {
        // this is zero allocation for single ranges
        // todo: optimize this in range collections
        let mut ranges = ChunkRanges::empty();
        let mut current = ChunkNum(0);
        let mut on = false;
        for &width in self.0.iter() {
            let next = current + width;
            if on {
                ranges |= ChunkRanges::from(current..next);
            }
            current = next;
            on = !on;
        }
        if on {
            ranges |= ChunkRanges::from(current..);
        }
        ranges
    }
}

impl fmt::Debug for RangeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_all() {
            write!(f, "all")
        } else if self.is_empty() {
            write!(f, "empty")
        } else if !f.alternate() {
            f.debug_list()
                .entries(self.to_chunk_ranges().iter())
                .finish()
        } else {
            f.debug_list().entries(self.0.iter()).finish()
        }
    }
}

mod wire {

    use serde::{Deserialize, Serialize};
    use smallvec::SmallVec;

    use super::{ChunkRangesSeq, RangeSpec};

    #[derive(Deserialize, Serialize)]
    pub struct RangeSpecSeq(SmallVec<[(u64, RangeSpec); 2]>);

    impl From<RangeSpecSeq> for ChunkRangesSeq {
        fn from(wire: RangeSpecSeq) -> Self {
            let mut offset = 0;
            let mut res = SmallVec::new();
            for (delta, spec) in wire.0.iter() {
                offset += *delta;
                res.push((offset, spec.to_chunk_ranges()));
            }
            Self(res)
        }
    }

    impl From<ChunkRangesSeq> for RangeSpecSeq {
        fn from(value: ChunkRangesSeq) -> Self {
            let mut res = SmallVec::new();
            let mut offset = 0;
            for (i, r) in value.0.iter() {
                let delta = *i - offset;
                res.push((delta, RangeSpec::new(r)));
                offset = *i;
            }
            Self(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use iroh_test::{assert_eq_hex, hexdump::parse_hexdump};
    use proptest::prelude::*;

    use super::*;
    use crate::protocol::ChunkRangesExt;

    fn ranges(value_range: Range<u64>) -> impl Strategy<Value = ChunkRanges> {
        prop::collection::vec((value_range.clone(), value_range), 0..16).prop_map(|v| {
            let mut res = ChunkRanges::empty();
            for (a, b) in v {
                let start = a.min(b);
                let end = a.max(b);
                res |= ChunkRanges::chunks(start..end);
            }
            res
        })
    }

    fn range_spec_seq_roundtrip_impl(ranges: &[ChunkRanges]) -> Vec<ChunkRanges> {
        let spec = ChunkRangesSeq::from_ranges(ranges.iter().cloned());
        spec.iter_infinite()
            .take(ranges.len())
            .cloned()
            .collect::<Vec<_>>()
    }

    fn range_spec_seq_bytes_roundtrip_impl(ranges: &[ChunkRanges]) -> Vec<ChunkRanges> {
        let spec = ChunkRangesSeq::from_ranges(ranges.iter().cloned());
        let bytes = postcard::to_allocvec(&spec).unwrap();
        let spec2: ChunkRangesSeq = postcard::from_bytes(&bytes).unwrap();
        spec2
            .iter_infinite()
            .take(ranges.len())
            .cloned()
            .collect::<Vec<_>>()
    }

    fn mk_case(case: Vec<Range<u64>>) -> Vec<ChunkRanges> {
        case.iter()
            .map(|x| ChunkRanges::chunks(x.start..x.end))
            .collect::<Vec<_>>()
    }

    #[test]
    fn range_spec_wire_format() {
        // a list of commented hex dumps and the corresponding range spec
        let cases = [
            (RangeSpec::EMPTY, "00"),
            (
                RangeSpec::all(),
                r"
                    01 # length prefix - 1 element
                    00 # span width - 0. everything stating from 0 is included
                ",
            ),
            (
                RangeSpec::new(ChunkRanges::chunks(64..)),
                r"
                    01 # length prefix - 1 element
                    40 # span width - 64. everything starting from 64 is included
                ",
            ),
            (
                RangeSpec::new(ChunkRanges::chunks(10000..)),
                r"
                    01 # length prefix - 1 element
                    904E # span width - 10000, 904E in postcard varint encoding. everything starting from 10000 is included
                ",
            ),
            (
                RangeSpec::new(ChunkRanges::chunks(..64)),
                r"
                    02 # length prefix - 2 elements
                    00 # span width - 0. everything stating from 0 is included
                    40 # span width - 64. everything starting from 64 is excluded
                ",
            ),
            (
                RangeSpec::new(&ChunkRanges::chunks(1..3) | &ChunkRanges::chunks(9..13)),
                r"
                    04 # length prefix - 4 elements
                    01 # span width - 1
                    02 # span width - 2 (3 - 1)
                    06 # span width - 6 (9 - 3)
                    04 # span width - 4 (13 - 9)
                ",
            ),
        ];
        for (case, expected_hex) in cases {
            let expected = parse_hexdump(expected_hex).unwrap();
            assert_eq_hex!(expected, postcard::to_stdvec(&case).unwrap());
        }
    }

    #[test]
    fn range_spec_seq_wire_format() {
        let cases = [
            (ChunkRangesSeq::empty(), "00"),
            (
                ChunkRangesSeq::all(),
                r"
                    01 # 1 tuple in total
                    # first tuple
                    00 # span 0 until start
                    0100 # 1 element, RangeSpec::all()
            ",
            ),
            (
                ChunkRangesSeq::from_ranges([
                    ChunkRanges::chunks(1..3),
                    ChunkRanges::chunks(7..13),
                ]),
                r"
                    03 # 3 tuples in total
                    # first tuple
                    00 # span 0 until start
                    020102 # range 1..3
                    # second tuple
                    01 # span 1 until next
                    020706 # range 7..13
                    # third tuple
                    01 # span 1 until next
                    00 # empty range forever from now
                ",
            ),
            (
                ChunkRangesSeq::from_ranges_infinite([
                    ChunkRanges::empty(),
                    ChunkRanges::empty(),
                    ChunkRanges::empty(),
                    ChunkRanges::chunks(7..),
                    ChunkRanges::all(),
                ]),
                r"
                    02 # 2 tuples in total
                    # first tuple
                    03 # span 3 until start (first 3 elements are empty)
                    01 07 # range 7..
                    # second tuple
                    01 # span 1 until next (1 element is 7..)
                    01 00 # ChunkRanges::all() forever from now
                ",
            ),
        ];
        for (case, expected_hex) in cases {
            let expected = parse_hexdump(expected_hex).unwrap();
            assert_eq_hex!(expected, postcard::to_stdvec(&case).unwrap());
        }
    }

    /// Test that the roundtrip from [`Vec<ChunkRanges>`] via [`RangeSpec`] to [`RangeSpecSeq`]  and back works.
    #[test]
    fn range_spec_seq_roundtrip_cases() {
        for case in [
            vec![0..1, 0..0],
            vec![1..2, 1..2, 1..2],
            vec![1..2, 1..2, 2..3, 2..3],
        ] {
            let case = mk_case(case);
            let expected = case.clone();
            let actual = range_spec_seq_roundtrip_impl(&case);
            assert_eq!(expected, actual);
        }
    }

    /// Test that the creation of a [`RangeSpecSeq`] from a sequence of [`ChunkRanges`]s canonicalizes the result.
    #[test]
    fn range_spec_seq_canonical() {
        for (case, expected_count) in [
            (vec![0..1, 0..0], 2),
            (vec![1..2, 1..2, 1..2], 2),
            (vec![1..2, 1..2, 2..3, 2..3], 3),
        ] {
            let case = mk_case(case);
            let spec = ChunkRangesSeq::from_ranges(case);
            assert_eq!(spec.0.len(), expected_count);
        }
    }

    proptest! {

        #[test]
        fn range_spec_roundtrip(ranges in ranges(0..1000)) {
            let spec = RangeSpec::new(&ranges);
            let ranges2 = spec.to_chunk_ranges();
            prop_assert_eq!(ranges, ranges2);
        }

        #[test]
        fn range_spec_seq_roundtrip(ranges in proptest::collection::vec(ranges(0..100), 0..10)) {
            let expected = ranges.clone();
            let actual = range_spec_seq_roundtrip_impl(&ranges);
            prop_assert_eq!(expected, actual);
        }

        #[test]
        fn range_spec_seq_bytes_roundtrip(ranges in proptest::collection::vec(ranges(0..100), 0..10)) {
            let expected = ranges.clone();
            let actual = range_spec_seq_bytes_roundtrip_impl(&ranges);
            prop_assert_eq!(expected, actual);
        }
    }
}
