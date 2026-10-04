#![allow(clippy::needless_lifetimes)]
use crate::iterators::SliceIterator;
use binary_merge::{MergeOperation, MergeState};
use core::fmt::Debug;
use inplace_vec_builder::InPlaceSmallVecBuilder;
use smallvec::{Array, SmallVec};

/// A typical write part for the merge state
pub(crate) trait MergeStateMut: MergeState {
    // Consume 1 elements from a and b, will copy from a
    fn advance_both(&mut self, copy: bool) -> bool {
        self.advance_a(1, copy) && self.advance_b(1, false)
    }
    /// Consume n elements of a, and update ac
    fn advance_a(&mut self, n: usize, take: bool) -> bool;
    /// Consume n elements of b, and update bc
    fn advance_b(&mut self, n: usize, take: bool) -> bool;

    fn ac(&self) -> bool;

    fn bc(&self) -> bool;
}

/// An in place merge state where the rhs is an owned smallvec
pub(crate) struct InPlaceMergeState<'a, A: Array, B: Array> {
    a: InPlaceSmallVecBuilder<'a, A>,
    b: smallvec::IntoIter<B>,
    ac: bool,
    bc: bool,
}

impl<'a, A: Array, B: Array> InPlaceMergeState<'a, A, B> {
    pub fn new(a: &'a mut SmallVec<A>, b: SmallVec<B>) -> Self {
        Self {
            a: a.into(),
            b: b.into_iter(),
            ac: false,
            bc: false,
        }
    }
}

impl<'a, A: Array, B: Array> MergeState for InPlaceMergeState<'a, A, B> {
    type A = A::Item;
    type B = B::Item;
    fn a_slice(&self) -> &[A::Item] {
        self.a.source_slice()
    }
    fn b_slice(&self) -> &[B::Item] {
        self.b.as_slice()
    }
}

impl<'a, A: Array> MergeStateMut for InPlaceMergeState<'a, A, A> {
    #[inline]
    fn advance_a(&mut self, n: usize, take: bool) -> bool {
        self.ac ^= is_odd(n);
        self.a.consume(n, take);
        true
    }
    #[inline]
    fn advance_b(&mut self, n: usize, take: bool) -> bool {
        self.bc ^= is_odd(n);
        if take {
            self.a.extend_from_iter(&mut self.b, n);
        } else {
            for _ in 0..n {
                let _ = self.b.next();
            }
        }
        true
    }
    fn ac(&self) -> bool {
        self.ac
    }
    fn bc(&self) -> bool {
        self.bc
    }
}

impl<'a, A: Array, B: Array> InPlaceMergeState<'a, A, B> {
    pub fn merge<O: MergeOperation<Self>>(a: &'a mut SmallVec<A>, b: SmallVec<B>, o: O) {
        let mut state = Self::new(a, b);
        o.merge(&mut state);
    }
}

/// An in place merge state where the rhs is a reference
pub(crate) struct InPlaceMergeStateRef<'a, A: Array, B> {
    a: InPlaceSmallVecBuilder<'a, A>,
    b: SliceIterator<'a, B>,
    ac: bool,
    bc: bool,
}

impl<'a, A: Array, B> InPlaceMergeStateRef<'a, A, B> {
    pub fn new(a: &'a mut SmallVec<A>, b: &'a impl AsRef<[B]>) -> Self {
        Self {
            a: a.into(),
            b: SliceIterator(b.as_ref()),
            ac: false,
            bc: false,
        }
    }
}

impl<'a, A: Array, B> MergeState for InPlaceMergeStateRef<'a, A, B> {
    type A = A::Item;
    type B = B;
    fn a_slice(&self) -> &[A::Item] {
        self.a.source_slice()
    }
    fn b_slice(&self) -> &[B] {
        self.b.as_slice()
    }
}

impl<'a, A: Array> MergeStateMut for InPlaceMergeStateRef<'a, A, A::Item>
where
    A::Item: Clone,
{
    #[inline]
    fn advance_a(&mut self, n: usize, take: bool) -> bool {
        self.ac ^= is_odd(n);
        self.a.consume(n, take);
        true
    }
    #[inline]
    fn advance_b(&mut self, n: usize, take: bool) -> bool {
        self.bc ^= is_odd(n);
        if take {
            self.a.extend_from_iter((&mut self.b).cloned(), n);
        } else {
            for _ in 0..n {
                let _ = self.b.next();
            }
        }
        true
    }
    fn ac(&self) -> bool {
        self.ac
    }
    fn bc(&self) -> bool {
        self.bc
    }
}

impl<'a, A: Array, B: 'a> InPlaceMergeStateRef<'a, A, B> {
    pub fn merge<O: MergeOperation<Self>>(a: &'a mut SmallVec<A>, b: &'a impl AsRef<[B]>, o: O) {
        let mut state = Self::new(a, b);
        o.merge(&mut state);
    }
}

/// A merge state where we only track if elements have been produced, and abort as soon as the first element is produced
pub(crate) struct BoolOpMergeState<'a, A, B> {
    a: SliceIterator<'a, A>,
    b: SliceIterator<'a, B>,
    ac: bool,
    bc: bool,
    r: bool,
}

impl<'a, A: Debug, B: Debug> Debug for BoolOpMergeState<'a, A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a: {:?}, b: {:?} r: {}",
            self.a_slice(),
            self.b_slice(),
            self.r
        )
    }
}

impl<'a, A, B> BoolOpMergeState<'a, A, B> {
    pub fn new(a: &'a [A], b: &'a [B]) -> Self {
        Self {
            a: SliceIterator(a),
            b: SliceIterator(b),
            ac: false,
            bc: false,
            r: false,
        }
    }

    pub fn result(self) -> bool {
        self.r
    }
}

impl<'a, A, B> BoolOpMergeState<'a, A, B> {
    pub fn merge<O: MergeOperation<Self>>(a: &'a [A], b: &'a [B], o: O) -> bool {
        let mut state = Self::new(a, b);
        o.merge(&mut state);
        state.r
    }
}

impl<'a, A, B> MergeState for BoolOpMergeState<'a, A, B> {
    type A = A;
    type B = B;
    fn a_slice(&self) -> &[A] {
        self.a.as_slice()
    }
    fn b_slice(&self) -> &[B] {
        self.b.as_slice()
    }
}

impl<'a, A, B> MergeStateMut for BoolOpMergeState<'a, A, B> {
    fn advance_a(&mut self, n: usize, take: bool) -> bool {
        self.ac ^= is_odd(n);
        if take {
            self.r = true;
            false
        } else {
            self.a.drop_front(n);
            true
        }
    }

    fn advance_b(&mut self, n: usize, take: bool) -> bool {
        self.bc ^= is_odd(n);
        if take {
            self.r = true;
            false
        } else {
            self.b.drop_front(n);
            true
        }
    }
    fn ac(&self) -> bool {
        self.ac
    }
    fn bc(&self) -> bool {
        self.bc
    }
}

/// A merge state where we build into a new vector
pub(crate) struct SmallVecMergeState<'a, A, B, Arr: Array> {
    a: SliceIterator<'a, A>,
    b: SliceIterator<'a, B>,
    ac: bool,
    bc: bool,
    r: SmallVec<Arr>,
}

impl<'a, A: Debug, B: Debug, Arr: Array> Debug for SmallVecMergeState<'a, A, B, Arr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "a: {:?}, b: {:?}", self.a_slice(), self.b_slice(),)
    }
}

impl<'a, A, B, Arr: Array> SmallVecMergeState<'a, A, B, Arr> {
    pub fn new(a: &'a [A], b: &'a [B], r: SmallVec<Arr>) -> Self {
        Self {
            a: SliceIterator(a),
            b: SliceIterator(b),
            ac: false,
            bc: false,
            r,
        }
    }

    pub fn result(self) -> SmallVec<Arr> {
        self.r
    }

    pub fn merge<O: MergeOperation<Self>>(a: &'a [A], b: &'a [B], o: O) -> SmallVec<Arr> {
        let t: SmallVec<Arr> = SmallVec::new();
        let mut state = Self::new(a, b, t);
        o.merge(&mut state);
        state.result()
    }
}

impl<'a, A, B, Arr: Array> MergeState for SmallVecMergeState<'a, A, B, Arr> {
    type A = A;
    type B = B;
    fn a_slice(&self) -> &[A] {
        self.a.as_slice()
    }
    fn b_slice(&self) -> &[B] {
        self.b.as_slice()
    }
}

impl<'a, T: Clone, Arr: Array<Item = T>> MergeStateMut for SmallVecMergeState<'a, T, T, Arr> {
    fn advance_a(&mut self, n: usize, take: bool) -> bool {
        self.ac ^= is_odd(n);
        if take {
            self.r.reserve(n);
            for e in self.a.take_front(n).iter() {
                self.r.push(e.clone())
            }
        } else {
            self.a.drop_front(n);
        }
        true
    }

    fn advance_b(&mut self, n: usize, take: bool) -> bool {
        self.bc ^= is_odd(n);
        if take {
            self.r.reserve(n);
            for e in self.b.take_front(n).iter() {
                self.r.push(e.clone())
            }
        } else {
            self.b.drop_front(n);
        }
        true
    }
    fn ac(&self) -> bool {
        self.ac
    }
    fn bc(&self) -> bool {
        self.bc
    }
}

#[inline]
fn is_odd(x: usize) -> bool {
    (x & 1) != 0
}
