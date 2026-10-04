use binary_merge::{MergeOperation, MergeState};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{any::type_name, ops::Range, rc::Rc};

struct VecMergeState<'a, T> {
    a: std::slice::Iter<'a, T>,
    b: std::slice::Iter<'a, T>,
    r: Vec<T>,
}

impl<'a, T> MergeState for VecMergeState<'a, T> {
    type A = T;

    type B = T;

    fn a_slice(&self) -> &[Self::A] {
        self.a.as_slice()
    }

    fn b_slice(&self) -> &[Self::B] {
        self.b.as_slice()
    }
}

struct BinaryMergeUnion;

impl<'a, T: Ord + Clone> MergeOperation<VecMergeState<'a, T>> for BinaryMergeUnion {
    fn from_a(&self, m: &mut VecMergeState<'a, T>, n: usize) -> bool {
        m.r.extend((&mut m.a).cloned().take(n));
        true
    }

    fn from_b(&self, m: &mut VecMergeState<'a, T>, n: usize) -> bool {
        m.r.extend((&mut m.b).cloned().take(n));
        true
    }

    fn collision(&self, m: &mut VecMergeState<'a, T>) -> bool {
        m.r.extend((&mut m.a).cloned().take(1));
        m.b.next();
        true
    }

    fn cmp(&self, a: &T, b: &T) -> std::cmp::Ordering {
        a.cmp(b)
    }
}

struct TapeMergeUnion;

impl<'a, T: Ord + Clone> MergeOperation<VecMergeState<'a, T>> for TapeMergeUnion {
    fn from_a(&self, m: &mut VecMergeState<'a, T>, n: usize) -> bool {
        m.r.extend((&mut m.a).cloned().take(n));
        true
    }

    fn from_b(&self, m: &mut VecMergeState<'a, T>, n: usize) -> bool {
        m.r.extend((&mut m.b).cloned().take(n));
        true
    }

    fn collision(&self, m: &mut VecMergeState<'a, T>) -> bool {
        m.r.extend((&mut m.a).cloned().take(1));
        m.b.next();
        true
    }

    fn cmp(&self, a: &T, b: &T) -> std::cmp::Ordering {
        a.cmp(b)
    }

    const MCM_THRESHOLD: usize = usize::MAX;
}

/// binary merge union
fn binary_merge_union<T: Ord + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut state = VecMergeState {
        a: a.iter(),
        b: b.iter(),
        r: Vec::with_capacity(a.len().max(b.len())),
    };
    BinaryMergeUnion.merge(&mut state);
    state.r
}

/// tape merge union
fn tape_merge_union<T: Ord + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut state = VecMergeState {
        a: a.iter(),
        b: b.iter(),
        r: Vec::with_capacity(a.len().max(b.len())),
    };
    TapeMergeUnion.merge(&mut state);
    state.r
}

/// handrolled version of a tape merge, just as a baseline
fn tape_merge_union_handrolled<T: Ord + Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut res = Vec::with_capacity(a.len().max(b.len()));
    let mut ai = 0;
    let mut bi = 0;
    while ai < a.len() && bi < b.len() {
        match a[ai].cmp(&b[bi]) {
            std::cmp::Ordering::Less => {
                res.push(a[ai].clone());
                ai += 1;
            }
            std::cmp::Ordering::Equal => {
                res.push(a[ai].clone());
                ai += 1;
                bi += 1;
            }
            std::cmp::Ordering::Greater => {
                res.push(b[bi].clone());
                bi += 1;
            }
        }
    }
    res.extend_from_slice(&a[ai..]);
    res.extend_from_slice(&b[bi..]);
    res
}

fn union_benches<T: Ord + Clone>(
    a: Range<usize>,
    b: Range<usize>,
    f: impl Fn(usize) -> T + Copy,
    name: &str,
    c: &mut Criterion,
) {
    let name = format!("{} T={} a={:?} b={:?}", name, type_name::<T>(), a, b);
    let ae = a.map(f).collect::<Vec<_>>();
    let be = b.map(f).collect::<Vec<_>>();
    c.bench_function(&format!("union {} binary merge", name), |bencher| {
        bencher.iter(|| binary_merge_union(black_box(&ae), black_box(&be)))
    });
    c.bench_function(&format!("union {} tape merge", name), |bencher| {
        bencher.iter(|| tape_merge_union(black_box(&ae), black_box(&be)))
    });
    c.bench_function(
        &format!("union {} tape merge, handrolled", name),
        |bencher| bencher.iter(|| tape_merge_union_handrolled(black_box(&ae), black_box(&be))),
    );
}

fn full_overlap(c: &mut Criterion) {
    union_benches(0..1000, 0..1000, |x| x, "full_overlap", c);
}

fn partial_overlap(c: &mut Criterion) {
    union_benches(0..1000, 500..1500, |x| x, "partial_overlap", c);
}

fn no_overlap(c: &mut Criterion) {
    union_benches(0..1000, 1000..2000, |x| x, "no_overlap", c);
}

fn insertion(c: &mut Criterion) {
    union_benches(0..2000, 232..233, |x| x, "insert", c);
}

fn insertion_rev_0<T: Ord + Clone>(f: impl Fn(usize) -> T + Copy, c: &mut Criterion) {
    union_benches(1234..1235, 0..2000, f, "insert", c);
}

fn insertion_rev_usize(c: &mut Criterion) {
    insertion_rev_0(|x| x, c)
}

fn insertion_rev_ratio(c: &mut Criterion) {
    insertion_rev_0(
        |x| {
            // make a thing that is cheap to copy, but expensive to compare
            let mut data = vec![0u8; 4096 - 8];
            data.extend_from_slice(&(x as u64).to_be_bytes());
            Rc::<[u8]>::from(data)
        },
        c,
    )
}

criterion_group!(
    benches,
    full_overlap,
    partial_overlap,
    no_overlap,
    insertion,
    insertion_rev_usize,
    insertion_rev_ratio,
);
criterion_main!(benches);
