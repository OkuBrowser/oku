mod common;

use connexa::prelude::{ConnexaSwarmEvent, Multiaddr, PeerId};
use futures::StreamExt;
use libp2p::swarm::dial_opts::DialOpts;
use std::time::Duration;

use crate::common::spawn_connexa_with_default_key;

#[tokio::test]
async fn test_listen_on_address() {
    let node = spawn_connexa_with_default_key().await;
    let addr: Multiaddr = "/memory/0".parse().unwrap();

    let listener_id = node.swarm().listen_on(addr).await.unwrap();

    // Get the actual listening addresses
    let addrs = node
        .swarm()
        .get_listening_addresses(listener_id)
        .await
        .unwrap();
    assert!(
        !addrs.is_empty(),
        "Should have at least one listening address"
    );
}

#[tokio::test]
async fn test_listening_addresses() {
    let node = spawn_connexa_with_default_key().await;
    let addr: Multiaddr = "/memory/0".parse().unwrap();

    // Start listening
    let listener_id = node.swarm().listen_on(addr).await.unwrap();

    // verify listening id
    node.swarm()
        .get_listening_addresses(listener_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_remove_listener() {
    let node = spawn_connexa_with_default_key().await;
    let addr: Multiaddr = "/memory/0".parse().unwrap();

    let listener_id = node.swarm().listen_on(addr).await.unwrap();

    // Verify we're listening
    let addrs = node
        .swarm()
        .get_listening_addresses(listener_id)
        .await
        .unwrap();
    assert!(!addrs.is_empty());

    // Remove the listener
    node.swarm().remove_listener(listener_id).await.unwrap();

    // Should no longer have this listener
    let result = node.swarm().get_listening_addresses(listener_id).await;
    assert!(result.is_err() || result.unwrap().is_empty());
}

#[tokio::test]
async fn test_external_addresses() {
    let node = spawn_connexa_with_default_key().await;
    let addr: Multiaddr = "/ip4/127.0.0.1/tcp/1234".parse().unwrap();

    // Initially no external addresses
    let addrs = node.swarm().external_addresses().await.unwrap();
    assert_eq!(addrs.len(), 1);

    // Add an external address
    node.swarm()
        .add_external_address(addr.clone())
        .await
        .unwrap();

    // Check external addresses
    let addrs = node.swarm().external_addresses().await.unwrap();
    assert_eq!(addrs.len(), 2);
    assert!(addrs.contains(&addr));
}

#[tokio::test]
async fn test_remove_external_address() {
    let node = spawn_connexa_with_default_key().await;
    let addr: Multiaddr = "/ip4/127.0.0.1/tcp/1234".parse().unwrap();

    // Add an external address
    node.swarm()
        .add_external_address(addr.clone())
        .await
        .unwrap();

    // Verify it was added
    let addrs = node.swarm().external_addresses().await.unwrap();
    assert_eq!(addrs.len(), 2);

    // Remove the external address
    node.swarm().remove_external_address(addr).await.unwrap();

    // Verify it was removed
    let addrs = node.swarm().external_addresses().await.unwrap();
    assert_eq!(addrs.len(), 1);
}

#[tokio::test]
async fn test_dial_and_connect() {
    let [
        (_node1, node1_peer_id, node1_addr),
        (node2, _node2_peer_id, _node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    // Node2 dials node1
    node2.swarm().dial(node1_addr).await.unwrap();

    // Check if connected
    let is_connected = node2.swarm().is_connected(node1_peer_id).await.unwrap();
    assert!(is_connected);
}

#[tokio::test]
async fn test_connected_peers() {
    let [
        (node1, _node1_peer_id, node1_addr),
        (node2, node2_peer_id, _node2_addr),
        (node3, node3_peer_id, _node3_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<3>().await;

    // Initially no connected peers
    let peers = node1.swarm().connected_peers().await.unwrap();
    assert_eq!(peers.len(), 0);

    // Node2 and Node3 connect to Node1
    node2.swarm().dial(node1_addr.clone()).await.unwrap();
    node3.swarm().dial(node1_addr).await.unwrap();

    // Node1 should have 2 connected peers
    let peers = node1.swarm().connected_peers().await.unwrap();
    assert_eq!(peers.len(), 2);

    let expected_peers = vec![node2_peer_id, node3_peer_id];

    for peer in expected_peers {
        assert!(peers.contains(&peer));
    }
}

#[tokio::test]
async fn test_disconnect_by_peer_id() {
    let [
        (_node1, node1_peer_id, node1_addr),
        (node2, _node2_peer_id, _node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    // Establish connection
    node2.swarm().dial(node1_addr).await.unwrap();

    // Verify connection
    let is_connected = node2.swarm().is_connected(node1_peer_id).await.unwrap();
    assert!(is_connected);

    // Disconnect by PeerId

    node2.swarm().disconnect(node1_peer_id).await.unwrap();

    // Verify disconnection
    let is_connected = node2.swarm().is_connected(node1_peer_id).await.unwrap();
    assert!(!is_connected);
}

#[tokio::test]
async fn test_disconnect_by_connection_id() {
    let [
        (_node1, node1_peer_id, node1_addr),
        (node2, _node2_peer_id, _node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    // Establish connection
    let connection_id = node2.swarm().dial(node1_addr).await.unwrap();

    // Verify connection
    let is_connected = node2.swarm().is_connected(node1_peer_id).await.unwrap();
    assert!(is_connected);

    // Disconnect by ConnectionId
    node2.swarm().disconnect(connection_id).await.unwrap();

    // Verify disconnection
    let is_connected = node2.swarm().is_connected(node1_peer_id).await.unwrap();
    assert!(!is_connected);
}

// TODO: Uncomment when re-adding peer store
// #[tokio::test]
// async fn test_add_peer_address() {
//     let node1 = spawn_connexa_with_default_key().await;
//     let node2 = spawn_connexa_with_default_key().await;
//
//     let peer_id = node2.keypair().public().to_peer_id();
//     let addr: Multiaddr = "/memory/1234".parse().unwrap();
//
//     // Add peer address
//     node1
//         .swarm()
//         .add_peer_address(peer_id, addr.clone())
//         .await
//         .unwrap();
//
//     // Now we should be able to dial using just the PeerId
//     let dial_opts = DialOpts::peer_id(peer_id).build();
//
//     // This would fail without the address being added first
//     let result = node1.swarm().dial(dial_opts).await;
//     // The dial might fail because the address isn't actually listening,
//     // but the important part is that the address was added to the peer store
//     assert!(result.is_ok() || result.is_err());
// }

#[tokio::test]
async fn test_dial_with_dial_opts() {
    let [
        (_node1, node1_peer_id, node1_addr),
        (node2, _node2_peer_id, _node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    // Create DialOpts with peer_id and addresses
    let dial_opts = DialOpts::peer_id(node1_peer_id)
        .addresses(vec![node1_addr])
        .build();

    // Node2 dials node1 using DialOpts
    node2.swarm().dial(dial_opts).await.unwrap();

    // Check if connected
    let is_connected = node2.swarm().is_connected(node1_peer_id).await.unwrap();
    assert!(is_connected);
}

#[tokio::test]
async fn test_connection_listener() {
    let [
        (node1, _node1_peer_id, node1_addr),
        (node2, _node2_peer_id, _node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    let mut listener = node1.swarm().listener().await.unwrap();

    // Node2 connects to node1
    node2.swarm().dial(node1_addr).await.unwrap();

    // Wait for a connection event
    tokio::time::timeout(Duration::from_secs(2), async move {
        loop {
            if let Some(ConnexaSwarmEvent::ConnectionEstablished { .. }) = listener.next().await {
                break;
            }
        }
    })
    .await
    .expect("Timeout waiting for connection event");
}

#[tokio::test]
async fn test_multiple_connections_same_peer() {
    let [
        (_node1, node1_peer_id, node1_addr),
        (node2, _node2_peer_id, _node2_addr),
        (node3, _node3_peer_id, _node3_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<3>().await;

    // Node2 connects to both addresses
    node2.swarm().dial(node1_addr.clone()).await.unwrap();
    node3.swarm().dial(node1_addr).await.unwrap();

    // Should still be connected (even with multiple connections)
    let is_connected1 = node2.swarm().is_connected(node1_peer_id).await.unwrap();
    assert!(is_connected1);
    let is_connected2 = node3.swarm().is_connected(node1_peer_id).await.unwrap();
    assert!(is_connected2)
}

#[tokio::test]
async fn test_is_connected_nonexistent_peer() {
    let node = spawn_connexa_with_default_key().await;
    let random_peer_id = PeerId::random();

    let is_connected = node.swarm().is_connected(random_peer_id).await.unwrap();
    assert!(!is_connected);
}

// FIXME: Maybe have another test to check for tcp properly, but using memory transport would make it easier to use in CI environment
// due to most restrains
#[tokio::test]
async fn test_listening_on_invalid_address() {
    let node = spawn_connexa_with_default_key().await;

    // Try to listen on an invalid address (requires specific transport)
    let addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    let result = node.swarm().listen_on(addr).await;

    // This should fail because we only enabled memory transport
    assert!(result.is_err());
}

#[tokio::test]
async fn test_dial_self() {
    let [(node, _node_peer_id, self_addr)] =
        common::spawn_connexa_nodes_with_default_keys::<1>().await;

    // Try to dial self
    let result = node.swarm().dial(self_addr).await;

    // Dialing self should fail
    assert!(result.is_err());
}

#[tokio::test]
async fn test_external_addresses_add_remove() {
    let node = spawn_connexa_with_default_key().await;

    let addr: Multiaddr = "/ip4/1.2.3.4/tcp/1234".parse().unwrap();

    // Add external address
    node.swarm()
        .add_external_address(addr.clone())
        .await
        .unwrap();

    // Check address is present
    let addrs = node.swarm().external_addresses().await.unwrap();
    assert_eq!(addrs.len(), 2);
    assert!(addrs.contains(&addr));

    // Remove address
    node.swarm()
        .remove_external_address(addr.clone())
        .await
        .unwrap();

    // Check address is removed
    let addrs = node.swarm().external_addresses().await.unwrap();
    assert_eq!(addrs.len(), 1);
}
