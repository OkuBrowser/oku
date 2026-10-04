use core::ops::Range;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::prelude::*;
use range_collections::{RangeSet, RangeSet2};

type Elem = i32;

fn create_messages(n: usize, delay: usize) -> Vec<Range<usize>> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut msgs: Vec<Range<usize>> = Vec::new();
    let mut offset = 0;

    // create some random sized messages
    for _ in 0..n {
        let len = rng.gen::<usize>() % 10 + 1;
        msgs.push(Range {
            start: offset,
            end: offset + len,
        });
        offset += len;
    }

    // "delay" some of them by randomly swapping with the successor
    for _ in 0..delay {
        for i in 1..msgs.len() {
            if rng.gen::<bool>() {
                msgs.swap(i - 1, i);
            }
        }
    }
    msgs
}

fn union_new(a: &RangeSet2<Elem>, b: &RangeSet2<Elem>) -> RangeSet2<Elem> {
    a | b
}

fn intersection_new(a: &RangeSet2<Elem>, b: &RangeSet2<Elem>) -> RangeSet2<Elem> {
    a & b
}

fn intersects_new(a: &RangeSet2<Elem>, b: &RangeSet2<Elem>) -> bool {
    !a.is_disjoint(b)
}

fn make_on_off_profile(n: Elem, offset: Elem, stride: Elem) -> RangeSet2<Elem> {
    let mut res = RangeSet::empty();
    for i in 0..n {
        res ^= RangeSet::from((i * stride + offset)..);
    }
    res
}

pub fn interleaved(c: &mut Criterion) {
    let n = 100000;
    let a: RangeSet2<Elem> = make_on_off_profile(n, 0, 2);
    let b: RangeSet2<Elem> = make_on_off_profile(n, 1, 2);
    c.bench_function("union_interleaved_new", |bencher| {
        bencher.iter(|| union_new(black_box(&a), black_box(&b)))
    });
    c.bench_function("intersection_interleaved_new", |bencher| {
        bencher.iter(|| intersection_new(black_box(&a), black_box(&b)))
    });
    c.bench_function("intersects_interleaved_new", |bencher| {
        bencher.iter(|| intersects_new(black_box(&a), black_box(&b)))
    });
}

pub fn cutoff(c: &mut Criterion) {
    let n = 100000;
    let a: RangeSet2<Elem> = make_on_off_profile(n, 0, 2);
    let b: RangeSet2<Elem> = make_on_off_profile(n, 1, 1000);
    c.bench_function("union_cutoff_new", |bencher| {
        bencher.iter(|| union_new(black_box(&a), black_box(&b)))
    });
    c.bench_function("intersection_cutoff_new", |bencher| {
        bencher.iter(|| intersection_new(black_box(&a), black_box(&b)))
    });
    c.bench_function("intersects_cutoff_new", |bencher| {
        bencher.iter(|| intersects_new(black_box(&a), black_box(&b)))
    });
}

pub fn assemble(c: &mut Criterion) {
    let n = 10000;
    let msgs = create_messages(n, 5);
    c.bench_function("assemble", |bencher| {
        bencher.iter(|| {
            let mut buffer: RangeSet2<usize> = RangeSet::from(..0);
            for msg in msgs.iter() {
                buffer |= RangeSet::from(msg.clone());
            }
            buffer
        });
    });
}

criterion_group!(benches, interleaved, cutoff, assemble);
criterion_main!(benches);
