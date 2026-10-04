use futures::future::{AbortHandle, Abortable};
use futures::future::{Either, FutureExt, pending, select};
use ipld_core::cid::Cid;
use rust_ipfs::Node;
use tokio::{
    task,
    time::{sleep, timeout},
};

use std::{
    convert::TryFrom,
    future::Future,
    time::{Duration, Instant},
};

async fn bounded_retry<Fun, Fut, F, T>(
    timeout: Duration,
    mut future: Fun,
    check: F,
) -> Result<(), ()>
where
    Fun: FnMut() -> Fut,
    Fut: Future<Output = T>,
    F: Fn(T) -> bool,
{
    let started = Instant::now();
    let mut elapsed = Default::default();

    loop {
        if elapsed > timeout {
            return Err(());
        }
        if check(future().await) {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
        elapsed = started.elapsed();
    }
}

fn check_cid_subscriptions(ipfs: &Node, cid: &Cid, expected_count: usize) {
    let subscription_count = {
        let subs = ipfs.get_subscriptions().lock();
        if expected_count > 0 {
            assert_eq!(subs.len(), 1);
        }
        subs.get(cid).map(|l| l.len())
    };
    // treat None as 0
    assert_eq!(subscription_count.unwrap_or(0), expected_count);
}

/// Check if canceling a Cid affects the wantlist.
#[tokio::test]
#[ignore]
async fn wantlist_cancellation() {
    tracing_subscriber::fmt::init();

    // start a single node
    let ipfs = Node::new("test_node").await;

    // execute a get_block request
    let cid = Cid::try_from("QmSoLPppuBtQSGwKDZT2M73ULpjvfd3aZ6ha4oFGL1KaGa").unwrap();

    // start a get_request future
    let ipfs_clone = ipfs.clone();
    let (abort_handle1, abort_reg) = AbortHandle::new_pair();
    let abortable_req = Abortable::new(async move { ipfs_clone.get_block(cid).await }, abort_reg);
    let _get_request1 = task::spawn(abortable_req);

    // verify that the requested Cid is in the wantlist
    let wantlist_populated = bounded_retry(
        Duration::from_secs(1),
        || ipfs.bitswap_wantlist(None),
        |ret| ret.unwrap().first() == Some(&cid),
    )
    .await;

    assert!(
        wantlist_populated.is_ok(),
        "the wantlist is still empty after the request was issued"
    );

    // ensure that there is a single related subscription
    check_cid_subscriptions(&ipfs, &cid, 1);

    // fire up an additional get request, this time within the same async task...
    let ipfs_clone = ipfs.clone();
    let get_request2 = ipfs_clone.get_block(cid);
    let get_timeout = timeout(Duration::from_millis(100), pending::<()>());
    let get_request2 = match select(get_timeout.boxed(), get_request2.boxed()).await {
        Either::Left((_, fut)) => fut,
        Either::Right(_) => unreachable!(),
    };

    // ensure that there are 2 related subscriptions
    check_cid_subscriptions(&ipfs, &cid, 2);

    // ...and an additional one within the same task, for good measure
    let ipfs_clone = ipfs.clone();
    let get_request3 = ipfs_clone.get_block(cid);
    let get_timeout = timeout(Duration::from_millis(100), pending::<()>());
    let get_request3 = match select(get_timeout.boxed(), get_request3.boxed()).await {
        Either::Left((_, fut)) => fut,
        Either::Right(_) => unreachable!(),
    };

    // ensure that there are 3 related subscription
    check_cid_subscriptions(&ipfs, &cid, 3);

    // cancel the first requested Cid
    abort_handle1.abort();

    // verify that the requested Cid is still in the wantlist
    let wantlist_partially_cleared1 = bounded_retry(
        Duration::from_secs(1),
        || ipfs.bitswap_wantlist(None),
        |ret| ret.unwrap().len() == 1,
    )
    .await;

    assert!(
        wantlist_partially_cleared1.is_ok(),
        "the wantlist is empty despite there still being 2 live get requests"
    );

    // ensure that there are 2 related subscriptions
    check_cid_subscriptions(&ipfs, &cid, 2);

    // cancel the second requested Cid
    drop(get_request2);

    // verify that the requested Cid is STILL in the wantlist
    let wantlist_partially_cleared2 = bounded_retry(
        Duration::from_secs(1),
        || ipfs.bitswap_wantlist(None),
        |ret| ret.unwrap().len() == 1,
    )
    .await;

    assert!(
        wantlist_partially_cleared2.is_ok(),
        "the wantlist is empty despite there still being a live get request"
    );

    // ensure that there is a single related subscription
    check_cid_subscriptions(&ipfs, &cid, 1);

    // cancel the second requested Cid
    drop(get_request3);

    // verify that the requested Cid is no longer in the wantlist
    let wantlist_cleared = bounded_retry(
        Duration::from_secs(1),
        || ipfs.bitswap_wantlist(None),
        |ret| ret.unwrap().is_empty(),
    )
    .await;

    assert!(
        wantlist_cleared.is_ok(),
        "a block was not removed from the wantlist after all its subscriptions had died"
    );

    // ensure that there are no related subscriptions
    check_cid_subscriptions(&ipfs, &cid, 0);
}
