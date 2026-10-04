use criterion::{black_box, criterion_group, criterion_main, Criterion};

const QUICK_BROWN_FOX_ENC: &str =
    "8UADZCKFEOEDJOD2KC54EM-DX.CH8FSKDQ$D.OE44E5$CS44+8DK44OEC3EFGVCD2";
const QUICK_BROWN_FOX_DEC: &str = "The quick brown fox jumps over the lazy dog";
fn encode(c: &mut Criterion) {
    c.bench_function("encode long string", |b| {
        b.iter(|| base45::encode(black_box("The quick brown fox jumps over the lazy dog")))
    });
}

fn encode_from_buffer(c: &mut Criterion) {
    c.bench_function("encode long string from buffer", |b| {
        b.iter(|| {
            base45::encode(black_box(vec![
                84, 104, 101, 32, 113, 117, 105, 99, 107, 32, 98, 114, 111, 119, 110, 32, 102, 111,
                120, 32, 106, 117, 109, 112, 115, 32, 111, 118, 101, 114, 32, 116, 104, 101, 32,
                108, 97, 122, 121, 32, 100, 111, 103,
            ]))
        })
    });
}

fn decode(c: &mut Criterion) {
    c.bench_function("decode long string", |b| {
        b.iter(|| {
            String::from_utf8(
                base45::decode(black_box(
                    "8UADZCKFEOEDJOD2KC54EM-DX.CH8FSKDQ$D.OE44E5$CS44+8DK44OEC3EFGVCD2",
                ))
                .unwrap(),
            )
            .unwrap()
        })
    });
}

fn bench_encode_quick_brown_fox(c: &mut Criterion) {
    c.bench_function("encode long str 100 times", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let text = String::from(QUICK_BROWN_FOX_DEC);
                let encoded = base45::encode(&text);
                assert_eq!(encoded, QUICK_BROWN_FOX_ENC);
            }
        })
    });
}

fn rbe<const N: usize>(c: &mut Criterion) {
    use rand::*;
    let mut rng = thread_rng();
    let mut b = [0u8; N];
    rng.fill_bytes(&mut b);
    c.bench_function(&format!("encode {:#x} random bytes", N), |bench| {
        bench.iter(|| {
            let encoded = base45::encode(&b[..]);
            assert!(encoded.is_ascii());
        })
    });
}
fn bench_encode_random_0x10(b: &mut Criterion) {
    rbe::<0x10>(b);
}
fn bench_encode_random_0x100(b: &mut Criterion) {
    rbe::<0x100>(b);
}
fn bench_encode_random_0x1000(b: &mut Criterion) {
    rbe::<0x1000>(b);
}
fn bench_encode_random_0x10000(b: &mut Criterion) {
    rbe::<0x10000>(b);
}

fn bench_decode_quick_brown_fox(c: &mut Criterion) {
    c.bench_function("decode long str 100 times", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let text = String::from(QUICK_BROWN_FOX_ENC);
                if let Ok(decoded) = base45::decode(&text) {
                    assert_eq!(decoded, QUICK_BROWN_FOX_DEC.as_bytes());
                }
            }
        })
    });
}

criterion_group!(
    benches,
    encode,
    encode_from_buffer,
    decode,
    bench_encode_quick_brown_fox,
    bench_decode_quick_brown_fox,
    bench_encode_random_0x10,
    bench_encode_random_0x100,
    bench_encode_random_0x1000,
    bench_encode_random_0x10000
);
criterion_main!(benches);
