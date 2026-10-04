use futures_timeout::TimeoutExt;
use rust_ipfs::Node;
use rust_ipfs::{Multiaddr, Protocol};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(5);

// Make sure two instances of ipfs can be connected by `Multiaddr`.
#[tokio::test]
async fn connect_two_nodes_by_addr() {
    let node_a = Node::new("a").await;
    let node_b = Node::new("b").await;

    node_a
        .connect(node_b.addrs[0].clone())
        .timeout(TIMEOUT)
        .await
        .expect("timeout")
        .expect("should have connected");
}

// Make sure two instances of ipfs can be connected by `PeerId`.
#[tokio::test]
async fn connect_two_nodes_by_peer_id() {
    let node_a = Node::new("a").await;
    let node_b = Node::new("b").await;

    node_a.add_peer(&node_b).await.unwrap();

    node_a.connect(node_b.id).await.unwrap()
}

// Ensure that duplicate connection attempts don't cause hangs.
#[tokio::test]
async fn connect_duplicate_multiaddr() {
    let node_a = Node::new("a").await;
    let node_b = Node::new("b").await;

    // test duplicate connections by address
    for _ in 0..3 {
        // final success or failure doesn't matter, there should be no timeout
        let _ = node_a
            .connect(node_b.addrs[0].clone())
            .timeout(TIMEOUT)
            .await
            .unwrap();
    }
}

// More complicated one to the above; first node will have two listening addresses and the second
// one should dial both of the addresses, resulting in two connections.
#[tokio::test]
async fn connect_two_nodes_with_two_connections_doesnt_panic() {
    let node_a = Node::new("a").await;
    let node_b = Node::new("b").await;

    node_a
        .add_listening_address(Multiaddr::empty().with(Protocol::Memory(0)))
        .await
        .unwrap();

    let addresses = node_a.listening_addresses().await.unwrap();
    assert_eq!(addresses.len(), 2);

    for mut addr in addresses.into_iter() {
        addr.push(Protocol::P2p(node_a.id));

        node_b
            .connect(addr)
            .timeout(TIMEOUT)
            .await
            .expect("timeout")
            .expect("should have connected");
    }

    // not too sure on this, since there'll be a single peer but two connections; the return
    // type is `Vec<Connection>` but it's peer with any connection.
    let mut peers = node_a.connected().await.unwrap();
    assert_eq!(peers.len(), 1);

    // sadly we are unable to currently verify that there exists two connections for the node_b
    // peer..

    node_a
        .disconnect(peers.remove(0))
        .await
        .expect("failed to disconnect peer_b at peer_a");

    let peers = node_a.connected().await.unwrap();
    assert!(
        peers.is_empty(),
        "node_b was still connected after disconnect: {peers:?}"
    );
}

#[tokio::test]
async fn connect_to_wrong_peer() {
    let a = Node::new("a").await;
    let b = Node::new("b").await;
    let c = Node::new("c").await;

    // take b's address but with c's peerid
    let mut wrong_addr = b.addrs[0].clone();
    assert!(matches!(wrong_addr.pop(), Some(Protocol::P2p(_))));
    wrong_addr.push(Protocol::P2p(c.id));

    // timeout of one is not great, but it's enough to make the connection.
    let connection_result = a.connect(wrong_addr).timeout(Duration::from_secs(1)).await;

    for &(node, name) in &[(&c, "c"), (&b, "b"), (&a, "a")] {
        let peers = node.connected().await.unwrap();
        assert!(
            peers.is_empty(),
            "{name} should have no connections, but had: {peers:?}"
        );
    }

    connection_result
        .expect("connect timed out")
        .expect_err("connection should had failed (wrong peer id)");
}
