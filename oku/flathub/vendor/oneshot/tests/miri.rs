//! Run these tests with Miri in this way:
//! `MIRIFLAGS="-Zmiri-many-seeds=0..512 cargo +nightly miri test --test miri -- [<name of test function>]`
#![cfg(not(oneshot_loom))]

#[cfg(any(feature = "async", feature = "std"))]
use std::hint::spin_loop;
#[cfg(feature = "std")]
use std::time::Duration;
#[cfg(feature = "async")]
use std::{
    future::Future,
    pin::pin,
    task::{self, Poll, Waker},
};

// Exhaustively testing all combinations of what can be done with the sender and receiver
// TX:
// - send
// - drop
// RX:
// - drop
// - recv
// - recv_ref
// - recv_timeout
// - try_recv until completion
// - try_recv then drop
// - poll until completion
// - poll then drop

// ==== TX SEND =====

#[test]
fn tx_send_rx_drop() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        drop(rx);
    });

    let tx_thread = spawn_named("tx_thread", move || {
        let _ = tx.send(999);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_send_rx_recv_done() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        assert_eq!(rx.recv(), Ok(999));
    });

    let tx_thread = spawn_named("tx_thread", move || {
        let _ = tx.send(999);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_send_rx_recv_ref() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        assert_eq!(rx.recv_ref(), Ok(999));
    });

    let tx_thread = spawn_named("tx_thread", move || {
        let _ = tx.send(999);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_send_rx_recv_timeout() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        assert_eq!(rx.recv_timeout(Duration::from_millis(1000)), Ok(999));
    });

    let tx_thread = spawn_named("tx_thread", move || {
        let _ = tx.send(999);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_send_rx_try_recv_to_completion() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || loop {
        match rx.try_recv() {
            Ok(999) => break,
            Ok(val) => panic!("Unexpected Ok({val})"),
            Err(oneshot::TryRecvError::Empty) => spin_loop(),
            Err(oneshot::TryRecvError::Disconnected) => panic!("Unexpected disconnect"),
        }
    });

    let tx_thread = spawn_named("tx_thread", move || {
        let _ = tx.send(999);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_send_rx_try_recv_then_drop() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || match rx.try_recv() {
        Ok(999) => (),
        Ok(val) => panic!("Unexpected Ok({val})"),
        Err(oneshot::TryRecvError::Empty) => (),
        Err(oneshot::TryRecvError::Disconnected) => panic!("Unexpected disconnect"),
    });

    let tx_thread = spawn_named("tx_thread", move || {
        let _ = tx.send(999);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[cfg(feature = "async")]
#[test]
fn tx_send_rx_poll_to_completion() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        let mut rx = pin!(rx);
        let mut cx = task::Context::from_waker(Waker::noop());
        loop {
            match rx.as_mut().poll(&mut cx) {
                Poll::Ready(Ok(999)) => break,
                Poll::Ready(result) => panic!("Unexpected result: {:?}", result),
                Poll::Pending => spin_loop(),
            }
        }
    });

    let tx_thread = spawn_named("tx_thread", move || {
        tx.send(999).unwrap();
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[cfg(feature = "async")]
#[test]
fn tx_send_rx_poll_then_drop() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        let mut rx = pin!(rx);
        let mut cx = task::Context::from_waker(Waker::noop());
        match rx.as_mut().poll(&mut cx) {
            Poll::Ready(Ok(999)) => (),
            Poll::Ready(result) => panic!("Unexpected result: {:?}", result),
            Poll::Pending => (),
        }
    });

    let tx_thread = spawn_named("tx_thread", move || {
        let _ = tx.send(999);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

// ==== TX DROP =====

#[test]
fn tx_drop_rx_drop() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        drop(rx);
    });

    let tx_thread = spawn_named("tx_thread", move || {
        drop(tx);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_drop_rx_recv_done() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        assert!(rx.recv().is_err());
    });

    let tx_thread = spawn_named("tx_thread", move || {
        drop(tx);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_drop_rx_recv_ref() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        assert!(rx.recv_ref().is_err());
    });

    let tx_thread = spawn_named("tx_thread", move || {
        drop(tx);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_drop_rx_recv_timeout() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        assert_eq!(
            rx.recv_timeout(Duration::from_millis(1000)),
            Err(oneshot::RecvTimeoutError::Disconnected)
        );
    });

    let tx_thread = spawn_named("tx_thread", move || {
        drop(tx);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_drop_rx_try_recv_to_completion() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || loop {
        match rx.try_recv() {
            Ok(val) => panic!("Unexpected Ok({val})"),
            Err(oneshot::TryRecvError::Empty) => spin_loop(),
            Err(oneshot::TryRecvError::Disconnected) => break,
        }
    });

    let tx_thread = spawn_named("tx_thread", move || {
        drop(tx);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[test]
#[cfg(feature = "std")]
fn tx_drop_rx_try_recv_then_drop() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || match rx.try_recv() {
        Ok(val) => panic!("Unexpected Ok({val})"),
        Err(oneshot::TryRecvError::Empty) => (),
        Err(oneshot::TryRecvError::Disconnected) => (),
    });

    let tx_thread = spawn_named("tx_thread", move || {
        drop(tx);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[cfg(feature = "async")]
#[test]
fn tx_drop_rx_poll_to_completion() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        let mut rx = pin!(rx);
        let mut cx = task::Context::from_waker(Waker::noop());
        loop {
            match rx.as_mut().poll(&mut cx) {
                Poll::Ready(Err(oneshot::RecvError)) => break,
                Poll::Ready(result) => panic!("Unexpected result: {:?}", result),
                Poll::Pending => spin_loop(),
            }
        }
    });

    let tx_thread = spawn_named("tx_thread", move || {
        drop(tx);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[cfg(feature = "async")]
#[test]
fn tx_drop_rx_poll_then_drop() {
    let (tx, rx) = oneshot::channel::<i32>();

    let rx_thread = spawn_named("rx_thread", move || {
        let mut rx = pin!(rx);
        let mut cx = task::Context::from_waker(Waker::noop());
        match rx.as_mut().poll(&mut cx) {
            Poll::Ready(Ok(val)) => panic!("Unexpected Ok({}", val),
            Poll::Ready(Err(_)) => (),
            Poll::Pending => (),
        }
    });

    let tx_thread = spawn_named("tx_thread", move || {
        drop(tx);
    });

    rx_thread.join().unwrap();
    tx_thread.join().unwrap();
}

#[inline]
#[track_caller]
fn spawn_named<F>(name: &str, f: F) -> std::thread::JoinHandle<()>
where
    F: FnOnce() + Send + 'static,
{
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(f)
        .unwrap()
}
