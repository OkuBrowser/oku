//! A data structure for in place modification of smallvecs.
#![deny(missing_docs)]
#![allow(dead_code)]
use core::fmt::Debug;
use smallvec::{Array, SmallVec};

/// builds a SmallVec out of itself
pub struct InPlaceSmallVecBuilder<'a, A: Array> {
    /// the underlying vector, possibly containing some uninitialized values in the middle!
    v: &'a mut SmallVec<A>,
    /// the end of the target area
    t1: usize,
    /// the start of the source area
    s0: usize,
}

impl<'a, T: Debug, A: Array<Item = T>> Debug for InPlaceSmallVecBuilder<'a, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let InPlaceSmallVecBuilder { s0, t1, v } = self;
        let s1 = v.len();
        let cap = v.capacity();
        write!(
            f,
            "InPlaceSmallVecBuilder(0..{},{}..{},{})",
            t1, s0, s1, cap
        )
    }
}

/// initializes the source part of this flip buffer with the given vector.
/// The target part is initially empty.
impl<'a, A: Array> From<&'a mut SmallVec<A>> for InPlaceSmallVecBuilder<'a, A> {
    fn from(value: &'a mut SmallVec<A>) -> Self {
        InPlaceSmallVecBuilder {
            v: value,
            s0: 0,
            t1: 0,
        }
    }
}

impl<'a, A: Array> InPlaceSmallVecBuilder<'a, A> {
    /// The current target part as a slice
    pub fn target_slice(&self) -> &[A::Item] {
        &self.v[..self.t1]
    }

    /// The current source part as a slice
    pub fn source_slice(&self) -> &[A::Item] {
        &self.v[self.s0..]
    }

    /// The current source part as a slice
    pub fn source_slice_mut(&mut self) -> &mut [A::Item] {
        &mut self.v[self.s0..]
    }

    /// ensure that we have at least `capacity` space.
    #[inline]
    fn reserve(&mut self, capacity: usize) {
        // ensure we have space!
        if self.t1 + capacity > self.s0 {
            let v = &mut self.v;
            let s0 = self.s0;
            let s1 = v.len();
            let sn = s1 - s0;
            // delegate to the underlying vec for the grow logic
            v.reserve(capacity);
            // move the source to the end of the vec
            let cap = v.capacity();
            // just move source to the end without any concern about dropping
            unsafe {
                copy(v.as_mut_ptr(), s0, cap - sn, sn);
                v.set_len(cap);
            }
            // move s0
            self.s0 = cap - sn;
        }
    }

    /// Take at most `n` elements from `iter` to the target
    #[inline]
    pub fn extend_from_iter<I: Iterator<Item = A::Item>>(&mut self, mut iter: I, n: usize) {
        if n > 0 {
            self.reserve(n);
            for _ in 0..n {
                if let Some(value) = iter.next() {
                    self.push_unsafe(value)
                }
            }
        }
    }

    /// Push a single value to the target
    pub fn push(&mut self, value: A::Item) {
        // ensure we have space!
        self.reserve(1);
        self.push_unsafe(value);
    }

    fn push_unsafe(&mut self, value: A::Item) {
        unsafe { std::ptr::write(self.v.as_mut_ptr().add(self.t1), value) }
        self.t1 += 1;
    }

    /// Consume `n` elements from the source. If `take` is true they will be added to the target,
    /// else they will be dropped.
    #[inline]
    pub fn consume(&mut self, n: usize, take: bool) {
        let n = std::cmp::min(n, self.source_slice().len());
        let v = self.v.as_mut_ptr();
        if take {
            if self.t1 != self.s0 {
                unsafe {
                    copy(v, self.s0, self.t1, n);
                }
            }
            self.t1 += n;
            self.s0 += n;
        } else {
            for _ in 0..n {
                unsafe {
                    self.s0 += 1;
                    std::ptr::drop_in_place(v.add(self.s0 - 1));
                }
            }
        }
    }

    /// Skip up to `n` elements from source without adding them to the target.
    /// They will be immediately dropped!
    pub fn skip(&mut self, n: usize) {
        let n = std::cmp::min(n, self.source_slice().len());
        let v = self.v.as_mut_ptr();
        for _ in 0..n {
            unsafe {
                self.s0 += 1;
                std::ptr::drop_in_place(v.add(self.s0 - 1));
            }
        }
    }

    /// Take up to `n` elements from source to target.
    /// If n is larger than the size of the remaining source, this will only copy all remaining elements in source.
    pub fn take(&mut self, n: usize) {
        let n = std::cmp::min(n, self.source_slice().len());
        if self.t1 != self.s0 {
            unsafe {
                copy(self.v.as_mut_ptr(), self.s0, self.t1, n);
            }
        }
        self.t1 += n;
        self.s0 += n;
    }

    /// Takes the next element from the source, if it exists
    pub fn pop_front(&mut self) -> Option<A::Item> {
        if self.s0 < self.v.len() {
            self.s0 += 1;
            Some(unsafe { std::ptr::read(self.v.as_ptr().add(self.s0 - 1)) })
        } else {
            None
        }
    }

    fn drop_source(&mut self) {
        // use truncate to get rid of the source part, if any, calling drop as needed
        self.v.truncate(self.s0);
        // use set_len to get rid of the gap part between t1 and s0, not calling drop!
        unsafe {
            self.v.set_len(self.t1);
        }
        self.s0 = self.t1;
        // shorten the source part
    }
}

#[inline]
unsafe fn copy<T>(v: *mut T, from: usize, to: usize, n: usize) {
    // if to < from {
    //     for i in 0..n {
    //         std::ptr::write(v.add(to + i), std::ptr::read(v.add(from + i)));
    //     }
    // } else {
    //     for i in (0..n).rev() {
    //         std::ptr::write(v.add(to + i), std::ptr::read(v.add(from + i)));
    //     }
    // }
    std::ptr::copy(v.add(from), v.add(to), n);
}

/// the purpose of drop is to clean up and make the SmallVec that we reference into a normal
/// SmallVec again.
impl<'a, A: Array> Drop for InPlaceSmallVecBuilder<'a, A> {
    fn drop(&mut self) {
        // drop the source part.
        self.drop_source();
    }
}

#[cfg(test)]
mod tests {
    extern crate testdrop;
    use super::*;
    use testdrop::{Item, TestDrop};

    type Array<'a> = [Item<'a>; 2];

    fn everything_dropped<'a, F>(td: &'a TestDrop, n: usize, f: F)
    where
        F: Fn(SmallVec<Array<'a>>, SmallVec<Array<'a>>),
    {
        let mut a: SmallVec<Array<'a>> = SmallVec::new();
        let mut b: SmallVec<Array<'a>> = SmallVec::new();
        let mut ids: Vec<usize> = Vec::new();
        for _ in 0..n {
            let (id, item) = td.new_item();
            a.push(item);
            ids.push(id);
        }
        for _ in 0..n {
            let (id, item) = td.new_item();
            b.push(item);
            ids.push(id);
        }
        f(a, b);
        for id in ids {
            td.assert_drop(id);
        }
    }

    #[test]
    fn drop_just_source() {
        everything_dropped(&TestDrop::new(), 10, |mut a, _| {
            let _: InPlaceSmallVecBuilder<Array> = (&mut a).into();
        })
    }

    #[test]
    fn target_push_gap() {
        everything_dropped(&TestDrop::new(), 10, |mut a, b| {
            let mut res: InPlaceSmallVecBuilder<Array> = (&mut a).into();
            for x in b.into_iter() {
                res.push(x);
            }
        })
    }

    #[test]
    fn source_move_some() {
        everything_dropped(&TestDrop::new(), 10, |mut a, _| {
            let mut res: InPlaceSmallVecBuilder<Array> = (&mut a).into();
            res.take(3);
        })
    }

    #[test]
    fn source_move_all() {
        everything_dropped(&TestDrop::new(), 10, |mut a, _| {
            let mut res: InPlaceSmallVecBuilder<Array> = (&mut a).into();
            res.take(10);
        })
    }

    #[test]
    fn source_drop_some() {
        everything_dropped(&TestDrop::new(), 10, |mut a, _| {
            let mut res: InPlaceSmallVecBuilder<Array> = (&mut a).into();
            res.skip(3);
        })
    }

    #[test]
    fn source_drop_all() {
        everything_dropped(&TestDrop::new(), 10, |mut a, _| {
            let mut res: InPlaceSmallVecBuilder<Array> = (&mut a).into();
            res.skip(10);
        })
    }

    #[test]
    fn source_pop_some() {
        everything_dropped(&TestDrop::new(), 10, |mut a, _| {
            let mut res: InPlaceSmallVecBuilder<Array> = (&mut a).into();
            res.pop_front();
            res.pop_front();
            res.pop_front();
        })
    }
}
