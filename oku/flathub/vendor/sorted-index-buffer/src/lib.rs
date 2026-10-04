#[cfg(test)]
mod tests;

/// A buffer indexed by a u64 sequence number.
///
/// This behaves idential to a BTreeMap<u64, T>, but is optimized for the case where
/// the indices are mostly contiguous, with occasional gaps.
///
/// It will have large memory overhead if the indices are very sparse, so it
/// should not be used as a general-purpose sorted map.
///
/// # Internals
///
/// The underlying storage is a contiguous buffer of `Option<T>` of twice the size
/// of the key range, rounded up to the next power of two. We track min and max.
///
/// The buffer is a `Vec<Option<T>>`, which can have some additional unused capacity.
///
/// |_x_xxxx_|________| max-min=6, page size 8, buffer fits in first page
///   ^ min
///         ^ max
/// |______x_|xxxx____| max-min=6, page size 8, buffer spans two pages
///        ^ min
///               ^ max
///
/// Access is O(1). Insertion and removal is usually O(1), but will occasionally
/// move contents around to resize the buffer, which is O(n). Moving will only
/// happen every O(n.next_power_of_two()) operations, so amortized complexity is
/// still O(1).
#[derive(Debug, Clone)]
pub struct SortedIndexBuffer<T> {
    /// The underlying data buffer. Size is a power of two, 2 "pages".
    data: Vec<Option<T>>,
    /// The minimum valid index (inclusive).
    min: u64,
    /// The maximum valid index (exclusive, so we can model the empty buffer).
    max: u64,
}

impl<T> Default for SortedIndexBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SortedIndexBuffer<T> {
    /// Create a new SortedIndexBuffer with the given initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let data = Vec::with_capacity(capacity);
        Self {
            data,
            min: 0,
            max: 0,
        }
    }

    /// Create a new, empty SortedIndexBuffer.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Returns true if the buffer contains no elements.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns true if the buffer contains an element at the given index.
    pub fn contains_key(&self, index: u64) -> bool {
        self.get(index).is_some()
    }

    /// Iterate over all keys in the given index range in ascending order.
    pub fn keys_range<R: std::ops::RangeBounds<u64>>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = u64> + '_ {
        let (buf_start, buf_end, start) = self.resolve_range(range);
        self.data[buf_start..buf_end]
            .iter()
            .enumerate()
            .filter_map(move |(i, slot)| slot.as_ref().map(|_| start + i as u64))
    }

    /// Iterate over all values in the given index range in ascending order of their keys.
    pub fn values_range<R: std::ops::RangeBounds<u64>>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = &T> + '_ {
        let (buf_start, buf_end, _) = self.resolve_range(range);
        self.data[buf_start..buf_end]
            .iter()
            .filter_map(|slot| slot.as_ref())
    }

    /// Iterate over all values in the given index range in ascending order of their keys.
    pub fn values_range_mut<R: std::ops::RangeBounds<u64>>(
        &mut self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = &mut T> + '_ {
        let (buf_start, buf_end, _) = self.resolve_range(range);
        self.data[buf_start..buf_end]
            .iter_mut()
            .filter_map(|slot| slot.as_mut())
    }

    /// Iterate over all (index, value) pairs in the given index range in ascending order of their keys.
    pub fn iter_range<R: std::ops::RangeBounds<u64>>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = (u64, &T)> + '_ {
        let (buf_start, buf_end, start) = self.resolve_range(range);
        self.data[buf_start..buf_end]
            .iter()
            .enumerate()
            .filter_map(move |(i, slot)| slot.as_ref().map(|v| (start + i as u64, v)))
    }

    pub fn retain(&mut self, f: impl FnMut(u64, &mut T) -> bool) {
        if self.is_empty() {
            return;
        }
        let mut f = f;
        let base = base(self.min, self.max);
        for i in self.min..self.max {
            let offset = (i - base) as usize;
            let Some(v) = &mut self.data[offset] else {
                continue;
            };
            if !f(i, v) {
                self.data[offset] = None;
            }
        }
        // Now adjust min and max
        let start = (self.min - base) as usize;
        let end = (self.max - base) as usize;
        let min1 = self.data[start..end]
            .iter()
            .position(|slot| slot.is_some())
            .map(|p| p + start)
            .unwrap_or(end) as u64
            + base;
        let max1 = self.data[start..end]
            .iter()
            .rev()
            .position(|slot| slot.is_some())
            .map(|p| end - p)
            .unwrap_or(start + 1) as u64
            + base;
        self.resize(min1, max1);
        self.check_invariants();
    }

    /// Retain only the elements in the given index range.
    pub fn retain_range<R: std::ops::RangeBounds<u64>>(&mut self, range: R) {
        let (min1, max1) = self.clip_bounds(range);
        if min1 >= max1 {
            self.data.clear();
            self.min = 0;
            self.max = 0;
            self.check_invariants();
            return;
        }
        let base = base(self.min, self.max);
        for i in self.min..min1 {
            self.data[(i - base) as usize] = None;
        }
        for i in max1..self.max {
            self.data[(i - base) as usize] = None;
        }
        self.resize(min1, max1);
        self.check_invariants();
    }

    /// Iterate over all keys in the buffer in ascending order.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = u64> + '_ {
        self.keys_range(..)
    }

    /// Iterate over all values in the buffer in ascending order of their keys.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &T> + '_ {
        self.values_range(..)
    }

    /// Iterate over all values in the buffer in ascending order of their keys.
    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> + '_ {
        self.values_range_mut(..)
    }

    /// Iterate over all (index, value) pairs in the buffer in ascending order of their keys.
    ///
    /// Values are returned by reference.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (u64, &T)> + '_ {
        self.iter_range(..)
    }

    /// Turn into an iterator over all (index, value) pairs in the buffer in ascending order of their keys.
    ///
    /// This is an explicit method instead of implementing IntoIterator, so we can return a
    /// DoubleEndedIterator without having to name the iterator type.
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> impl DoubleEndedIterator<Item = (u64, T)> {
        let base = base(self.min, self.max);
        self.data
            .into_iter()
            .enumerate()
            .filter_map(move |(i, slot)| slot.map(|v| (base + i as u64, v)))
    }

    /// Convert range bounds into an inclusive start and exclusive end, clipped to the current
    /// bounds.
    ///
    /// The resulting range may be empty, which has to be handled by the caller.
    #[inline]
    fn clip_bounds<R: std::ops::RangeBounds<u64>>(&self, range: R) -> (u64, u64) {
        use std::ops::Bound;

        let start = match range.start_bound() {
            Bound::Included(&n) => n.max(self.min),
            Bound::Excluded(&n) => (n + 1).max(self.min),
            Bound::Unbounded => self.min,
        };
        let end = match range.end_bound() {
            Bound::Included(&n) => (n + 1).min(self.max),
            Bound::Excluded(&n) => n.min(self.max),
            Bound::Unbounded => self.max,
        };
        (start, end)
    }

    #[inline]
    fn resolve_range<R: std::ops::RangeBounds<u64>>(&self, range: R) -> (usize, usize, u64) {
        let (start, end) = self.clip_bounds(range);
        if start >= end {
            return (0, 0, start);
        }
        let base = base(self.min, self.max);
        let buf_start = (start - base) as usize;
        let buf_end = (end - base) as usize;
        (buf_start, buf_end, start)
    }

    /// Get a reference to the value at the given index, if it exists.
    pub fn get(&self, index: u64) -> Option<&T> {
        if index < self.min || index >= self.max {
            return None;
        }
        let base = base(self.min, self.max);
        let offset = (index - base) as usize;
        self.data[offset].as_ref()
    }

    /// Insert value at index.
    pub fn insert(&mut self, index: u64, value: T) {
        if self.is_empty() {
            self.min = index;
            self.max = index + 1;
            self.data.push(Some(value));
        } else {
            let (min1, max1) = (self.min.min(index), self.max.max(index + 1));
            self.resize(min1, max1);
            self.insert0(index, value);
        }
        self.check_invariants();
    }

    /// Remove value at index.
    pub fn remove(&mut self, index: u64) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let res = self.remove0(index);
        if index == self.min {
            let base = base(self.min, self.max);
            let start = (self.min - base) as usize;
            let end = (self.max - base) as usize;
            // no need to check start, since we just removed that element
            let skip = self.data[start + 1..end]
                .iter()
                .position(|slot| slot.is_some())
                .map(|p| p + 1)
                .unwrap_or(end - start);
            self.resize(self.min + skip as u64, self.max);
        } else if index + 1 == self.max {
            let base = base(self.min, self.max);
            let start = (self.min - base) as usize;
            let end = (self.max - base) as usize;
            // no need to check end-1, since we just removed that element
            let skip = self.data[start..end - 1]
                .iter()
                .rev()
                .position(|slot| slot.is_some())
                .map(|p| p + 1)
                .unwrap_or(end - start);
            self.resize(self.min, self.max - skip as u64);
        }
        self.check_invariants();
        res
    }

    /// Insert value at index, assuming the buffer already covers that index.
    ///
    /// The resulting buffer may violate the invariants.
    #[inline(always)]
    fn insert0(&mut self, index: u64, value: T) {
        let base = base(self.min, self.max);
        let offset = (index - base) as usize;
        self.buf_mut()[offset] = Some(value);
    }

    /// Remove value at index without resizing the buffer.
    ///
    /// The resulting buffer may violate the invariants.
    #[inline(always)]
    fn remove0(&mut self, index: u64) -> Option<T> {
        if index < self.min || index >= self.max {
            return None;
        }
        let base = base(self.min, self.max);
        let offset = (index - base) as usize;
        self.buf_mut()[offset].take()
    }

    /// Resize the buffer to cover the range [`min1`, `max1`), while preserving existing
    /// elements.
    #[inline(always)]
    fn resize(&mut self, min1: u64, max1: u64) {
        if min1 == self.min && max1 == self.max {
            // nothing to do
            return;
        }
        if min1 >= max1 {
            // resizing to empty buffer
            self.data.clear();
            self.min = 0;
            self.max = 0;
            return;
        }
        let len0 = self.buf().len();
        let len1 = buf_len(min1, max1);
        let base0 = base(self.min, self.max);
        let base1 = base(min1, max1);
        if len0 == len1 {
            // just need to move data around within the existing buffer
            //
            // we use rotate even though half the buffer is empty, because
            // otherwise we would require Copy on T.
            if base1 < base0 {
                let shift = (base0 - base1) as usize;
                self.buf_mut().rotate_right(shift);
            } else if base1 > base0 {
                let shift = (base1 - base0) as usize;
                self.buf_mut().rotate_left(shift);
            }
        } else if len0 < len1 {
            for _ in len0..len1 {
                self.data.push(None);
            }
            // Grow
            if len0 != 0 {
                let start0 = (self.min - base0) as usize;
                let start1 = (self.min - base1) as usize;
                let count = (self.max - self.min) as usize;
                for i in 0..count {
                    self.data.swap(start0 + i, start1 + i);
                }
            }
        } else {
            // Shrink
            let start0 = (min1 - base0) as usize;
            let start1 = (min1 - base1) as usize;
            let count = (max1 - min1) as usize;

            for i in 0..count {
                self.data.swap(start0 + i, start1 + i);
            }
            self.data.truncate(len1);
        }
        self.min = min1;
        self.max = max1;
    }

    #[inline(always)]
    fn buf(&self) -> &[Option<T>] {
        &self.data
    }

    #[inline(always)]
    fn buf_mut(&mut self) -> &mut [Option<T>] {
        &mut self.data
    }

    /// Check that the invariants of the SortedIndexBuffer hold.
    ///
    /// This should be called after each public &mut method.
    ///
    /// It is a noop in release builds.
    fn check_invariants(&self) {
        if self.is_empty() {
            // for the empty buffer, min and max must be zero
            debug_assert_eq!(self.min, 0);
            debug_assert_eq!(self.max, 0);
        } else {
            // for a non-empty buffer, elements min and max-1 must be valid
            debug_assert!(self.min < self.max);
            debug_assert!(self.get(self.min).is_some() && self.get(self.max - 1).is_some());
        }
    }

    /// Same as `check_invariants`, but also checks that the unused parts of the buffer are empty.
    ///
    /// This is more expensive, so only used in tests.
    #[cfg(test)]
    fn check_invariants_expensive(&self) {
        self.check_invariants();
        let base = base(self.min, self.max);
        let start = (self.min - base) as usize;
        let end = (self.max - base) as usize;
        for i in 0..start {
            debug_assert!(self.data[i].is_none());
        }
        for i in end..self.data.len() {
            debug_assert!(self.data[i].is_none());
        }
    }
}

/// Compute the minimum buffer length needed to cover [min, max) even in the
/// case where min..max go over a page boundary.
#[inline(always)]
fn buf_len(min: u64, max: u64) -> usize {
    let page_size = (max - min).next_power_of_two() as usize;
    page_size * 2
}

/// Compute the base index for the buffer covering [min, max).
#[inline(always)]
fn base(min: u64, max: u64) -> u64 {
    let buf_len = buf_len(min, max);
    let mask = (buf_len as u64) / 2 - 1;
    min & !mask
}
