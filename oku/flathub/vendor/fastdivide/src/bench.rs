#![feature(test)]

extern crate fastdivide;
extern crate test;

use fastdivide::DividerU64;
use test::Bencher;

#[bench]
fn bench_normal_divide(b: &mut Bencher) {
    let q: u64 = test::black_box(112u64);
    b.iter(|| {
        let n: u64 = test::black_box(152342341u64);
        n / q
    })
}

#[bench]
fn bench_fast_divide(b: &mut Bencher) {
    let fast_divider = DividerU64::divide_by(112u64);
    b.iter(|| {
        let mut v = 0;
        {
            let n: u64 = test::black_box(152342341u64);
            v += fast_divider.divide(n)
        }
        {
            let n: u64 = test::black_box(152342341u64);
            v += fast_divider.divide(n)
        }
        {
            let n: u64 = test::black_box(152342341u64);
            v += fast_divider.divide(n)
        }
        v
    })
}
