#[macro_use]
extern crate criterion;

use criterion::{black_box, Criterion};
use murmurhash32::{murmurhash2, murmurhash3};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("murmurhash2 5", |b| {
        b.iter(|| murmurhash2(black_box(b"turtl")))
    });
    c.bench_function("murmurhash2 8", |b| {
        b.iter(|| murmurhash2(black_box(b"abcdefgh")))
    });
    c.bench_function("murmurhash2 10", |b| {
        b.iter(|| murmurhash2(black_box(b"liketurtles")))
    });
    c.bench_function("murmurhash2 15", |b| {
        b.iter(|| murmurhash2(black_box(b"i like turtles!")))
    });
    c.bench_function("murmurhash2 100", |b| b.iter(|| murmurhash2(black_box(b"maitre corbeau sur un arbre perche tenait dans son bec un fromage. Maitre renard par l'odeur alleche"))));
    c.bench_function("murmurhash3 5", |b| {
        b.iter(|| murmurhash3(black_box(b"turtl")))
    });
    c.bench_function("murmurhash3 8", |b| {
        b.iter(|| murmurhash3(black_box(b"abcdefgh")))
    });
    c.bench_function("murmurhash3 10", |b| {
        b.iter(|| murmurhash3(black_box(b"liketurtles")))
    });
    c.bench_function("murmurhash3 15", |b| {
        b.iter(|| murmurhash3(black_box(b"i like turtles!")))
    });
    c.bench_function("murmurhash3 100", |b| b.iter(|| murmurhash3(black_box(b"maitre corbeau sur un arbre perche tenait dans son bec un fromage. Maitre renard par l'odeur alleche"))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
