use futures::TryFutureExt;
use rust_ipfs::Multiaddr;
use rust_ipfs::p2p::MultiaddrExt;

#[tokio::test]
async fn multiple_consecutive_ephemeral_listening_addresses() {
    let node = rust_ipfs::Node::new("test_node").await;

    let target: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().expect("valid multiaddr");

    let id = node.add_listening_address(target.clone()).await.unwrap();
    let list = node.get_listening_address(id).await.unwrap();
    assert!(!list.contains(&target));

    let id = node.add_listening_address(target.clone()).await.unwrap();
    let list = node.get_listening_address(id).await.unwrap();
    assert!(!list.contains(&target));
}

#[tokio::test]
async fn multiple_concurrent_ephemeral_listening_addresses_on_same_ip() {
    let node = rust_ipfs::Node::new("test_node").await;

    let target: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().expect("valid multiaddr");

    let first = node
        .add_listening_address(target.clone())
        .and_then(|id| node.get_listening_address(id));
    let second = node
        .add_listening_address(target)
        .and_then(|id| node.get_listening_address(id));

    let (first, second) = futures::future::join(first, second).await;

    // before we have an Swarm-alike api on the background task to make sure the two futures
    // (first and second) would attempt to modify the background task before a poll to the
    // inner swarm, this will produce one or two successes.
    //
    // with two attempts without polling the swarm in the between:
    // assert_eq!(first.is_ok(), second.is_err());
    //
    // intuitively it could seem that first will always succeed because it must get the first
    // attempt to push messages into the queue but not sure if that should be leaned on.

    assert!(
        first.is_ok() || second.is_ok(),
        "first: {first:?}, second: {second:?}"
    );
}

#[tokio::test]
#[cfg(not(target_os = "macos"))]
async fn multiple_concurrent_ephemeral_listening_addresses_on_different_ip() {
    let node = rust_ipfs::Node::new("test_node").await;

    // it doesnt work on mac os x as 127.0.0.2 is not enabled by default.
    let first =
        node.add_listening_address(libp2p::build_multiaddr!(Ip4([127, 0, 0, 1]), Tcp(0u16)));
    let second =
        node.add_listening_address(libp2p::build_multiaddr!(Ip4([127, 0, 0, 2]), Tcp(0u16)));

    let (first, second) = futures::future::join(first, second).await;

    // both should succeed
    first.unwrap();
    second.unwrap();
}

#[tokio::test]
async fn adding_unspecified_addr_resolves_with_first() {
    let node = rust_ipfs::Node::new("test_node").await;
    // there is no test in trying to match this with others as ... that would be quite
    // perilous.
    node.add_listening_address("/ip4/127.0.0.1/tcp/0".parse().expect("valid multiaddr"))
        .await
        .unwrap();
}

#[tokio::test]
async fn listening_for_multiple_unspecified_addresses() {
    let node = rust_ipfs::Node::new("test_node").await;
    // there is no test in trying to match this with others as ... that would be quite
    // perilous.
    let target: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().expect("valid multiaddr");
    let first = node.add_listening_address(target.clone());
    let second = node.add_listening_address(target);

    let (first, second) = futures::future::join(first, second).await;

    // this test is a bad one similar to multiple_concurrent_ephemeral_listening_addresses_on_same_ip
    // see also https://github.com/rs-ipfs/rust-ipfs/issues/194 for more discussion.

    // the other should be denied because there is a pending incomplete when trying to listen
    // on unspecified address
    assert!(
        first.is_ok() || second.is_ok(),
        "first: {first:?}, second: {second:?}"
    );
}

#[tokio::test]
async fn remove_listening_address() {
    let node = rust_ipfs::Node::new("test_node").await;

    let unbound: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().expect("valid multiaddr");
    let first = node.add_listening_address(unbound).await.unwrap();
    node.remove_listening_address(first).await.unwrap();
}

#[tokio::test]
async fn pre_configured_listening_addrs() {
    use rust_ipfs::Multiaddr;
    use rust_ipfs::Node;

    let addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();

    let ipfs = Node::with_options(None, Some(vec![addr.clone()])).await;

    let addrs = ipfs.identity(None).await.unwrap().listen_addrs;
    let addrs: Vec<Multiaddr> = addrs
        .into_iter()
        .map(|mut addr| {
            addr.extract_peer_id().expect("Peer id");
            addr
        })
        .collect();

    assert!(
        addrs.contains(&addr),
        "pre-configured listening addr not found; is port 4001 available to listen on?; listening addrs: {addrs:?}"
    );
}
