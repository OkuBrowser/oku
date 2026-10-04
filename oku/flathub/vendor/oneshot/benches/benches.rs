#[cfg(not(criterion))]
pub fn main() {
    eprintln!(
        "!!!!!! WARNING
To run benches, you neet to run with RUSTFLAGS=\"--cfg criterion\"
!!!!!! WARNING
        "
    );
}

#[cfg(criterion)]
criterion::criterion_group!(benches, imp::bench);
#[cfg(criterion)]
criterion::criterion_main!(benches);

#[cfg(criterion)]
mod imp {
    macro_rules! bench_send_and_recv {
        ($c:expr, $($type:ty => $value:expr);+) => {
            // Sanity check that all $values are of $type.
            $(let _: $type = $value;)*
            {
                let mut group = $c.benchmark_group("create_channel");
                $(group.bench_function(stringify!($type), |b| {
                    b.iter(oneshot::channel::<$type>)
                });)*
                group.finish();
            }
            {
                let mut group = $c.benchmark_group("create_and_send");
                $(group.bench_function(stringify!($type), |b| {
                    b.iter(|| {
                        let (sender, _receiver) = oneshot::channel();
                        sender.send(criterion::black_box($value)).unwrap()
                    });
                });)*
                group.finish();
            }
            {
                let mut group = $c.benchmark_group("create_and_send_on_closed");
                $(group.bench_function(stringify!($type), |b| {
                    b.iter(|| {
                        let (sender, _) = oneshot::channel();
                        sender.send(criterion::black_box($value)).unwrap_err()
                    });
                });)*
                group.finish();
            }
            #[cfg(feature = "std")]
            {
                let mut group = $c.benchmark_group("create_send_and_recv");
                $(group.bench_function(stringify!($type), |b| {
                    b.iter(|| {
                        let (sender, receiver) = oneshot::channel();
                        sender.send(criterion::black_box($value)).unwrap();
                        receiver.recv().unwrap()
                    });
                });)*
                group.finish();
            }
            #[cfg(feature = "std")]
            {
                let mut group = $c.benchmark_group("create_send_and_recv_ref");
                $(group.bench_function(stringify!($type), |b| {
                    b.iter(|| {
                        let (sender, receiver) = oneshot::channel();
                        sender.send(criterion::black_box($value)).unwrap();
                        receiver.recv_ref().unwrap()
                    });
                });)*
                group.finish();
            }
        };
    }

    pub fn bench(c: &mut criterion::Criterion) {
        bench_send_and_recv!(c,
            () => ();
            u8 => 7u8;
            u128 => 1234567u128;
            [u8; 64] => [0b10101010u8; 64];
            [u8; 4096] => [0b10101010u8; 4096]
        );

        bench_try_recv(c);
        #[cfg(feature = "std")]
        bench_recv_deadline_now(c);
        #[cfg(feature = "std")]
        bench_recv_timeout_zero(c);
    }

    fn bench_try_recv(c: &mut criterion::Criterion) {
        let (sender, receiver) = oneshot::channel::<u128>();
        c.bench_function("try_recv_empty", |b| {
            b.iter(|| receiver.try_recv().unwrap_err())
        });
        drop(sender);
        c.bench_function("try_recv_empty_closed", |b| {
            b.iter(|| receiver.try_recv().unwrap_err())
        });
    }

    #[cfg(feature = "std")]
    fn bench_recv_deadline_now(c: &mut criterion::Criterion) {
        let now = std::time::Instant::now();
        {
            let (_sender, receiver) = oneshot::channel::<u128>();
            c.bench_function("recv_deadline_now", |b| {
                b.iter(|| receiver.recv_deadline(now).unwrap_err())
            });
        }
        {
            let (sender, receiver) = oneshot::channel::<u128>();
            drop(sender);
            c.bench_function("recv_deadline_now_closed", |b| {
                b.iter(|| receiver.recv_deadline(now).unwrap_err())
            });
        }
    }

    #[cfg(feature = "std")]
    fn bench_recv_timeout_zero(c: &mut criterion::Criterion) {
        let zero = std::time::Duration::from_nanos(0);
        {
            let (_sender, receiver) = oneshot::channel::<u128>();
            c.bench_function("recv_timeout_zero", |b| {
                b.iter(|| receiver.recv_timeout(zero).unwrap_err())
            });
        }
        {
            let (sender, receiver) = oneshot::channel::<u128>();
            drop(sender);
            c.bench_function("recv_timeout_zero_closed", |b| {
                b.iter(|| receiver.recv_timeout(zero).unwrap_err())
            });
        }
    }
}
