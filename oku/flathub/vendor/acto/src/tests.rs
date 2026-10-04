#![cfg(feature = "tokio")]

use crate::{tokio::AcTokio, ActoCell, ActoHandle, ActoInput, ActoRuntime, SupervisionRef};
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::{sync::oneshot, time::timeout};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

macro_rules! assert_timed {
    ($cond:expr $(,$($arg:tt)+)?) => {
        for _ in 0..300 {
            std::thread::sleep(Duration::from_millis(10));
            if $cond { break; }
        }
        assert!($cond $(,$($arg)*)?);
    };
}

#[test]
fn supervisor_termination() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_test_writer()
        .init();
    let sys = AcTokio::new("test", 2).unwrap();

    let probe = Arc::new(());

    let (tx, rx) = oneshot::channel();
    let probe2 = probe.clone();
    let SupervisionRef { me: r, handle: j } =
        sys.spawn_actor::<_, _, _, Arc<()>>("super", move |mut cell| async move {
            let probe3 = probe2.clone();
            let _r = cell.spawn_supervised("c1", move |mut cell: ActoCell<i32, _>| async move {
                cell.recv().await;
                probe3
            });
            tracing::debug!("message");
            let probe3 = probe2.clone();
            let _r = cell.spawn_supervised("c2", move |mut cell: ActoCell<i32, _>| async move {
                cell.recv().await;
                probe3
            });
            println!("x");
            tx.send(()).ok();
            cell.recv().await
        });
    assert_eq!(&r.name()[..11], "super(test/");

    sys.with_rt(|rt| rt.block_on(async { timeout(Duration::from_secs(1), rx).await }))
        .expect("runtime should be there")
        .expect("timed out")
        .expect("channel gone");
    assert_eq!(Arc::strong_count(&probe), 4);

    r.send(());
    let msg = sys
        .with_rt(|rt| rt.block_on(async { timeout(Duration::from_secs(1), j.join()).await }))
        .expect("runtime should be there")
        .expect("timed out")
        .expect("panicked");
    assert_eq!(msg, ActoInput::Message(()));
    assert_timed!(
        Arc::strong_count(&probe) == 1,
        " was {}",
        Arc::strong_count(&probe)
    );
}

#[test]
fn termination_info() {
    let sys = AcTokio::new("test", 2).unwrap();
    let SupervisionRef {
        me: r,
        handle: mut j,
    } = sys.spawn_actor("buh", |mut cell: ActoCell<(), _>| async move {
        loop {
            cell.recv().await;
        }
    });
    assert!(!r.is_gone());
    assert!(!j.is_finished());
    Pin::new(&mut j).abort_pinned();
    assert_timed!(j.is_finished());
    assert!(r.is_gone());
}
