mod common;

use common::spawn_connexa_with_default_key;
use connexa::prelude::dht::{Mode, Quorum, RecordKey};
use connexa::prelude::{DHTEvent, Multiaddr, PeerId};
use futures::StreamExt;

use crate::common::DEFAULT_TIMEOUT;

async fn create_connected_nodes() -> (
    connexa::handle::Connexa,
    connexa::handle::Connexa,
    PeerId,
    PeerId,
) {
    let [
        (node1, node1_peer_id, node1_addr),
        (node2, node2_peer_id, node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    // Node2 connects to node1 and adds it to DHT routing table
    node2.swarm().dial(node1_addr.clone()).await.unwrap();
    node1
        .dht()
        .add_address(node2_peer_id, node2_addr)
        .await
        .unwrap();
    node2
        .dht()
        .add_address(node1_peer_id, node1_addr)
        .await
        .unwrap();

    (node1, node2, node1_peer_id, node2_peer_id)
}

#[tokio::test]
async fn test_dht_mode() {
    let node = spawn_connexa_with_default_key().await;

    // Get initial mode
    let mode = node.dht().mode().await.unwrap();
    // Default mode should be either Client or Server depending on node availability
    assert_eq!(mode, Mode::Server);

    // Set to Server mode
    node.dht().set_mode(Mode::Server).await.unwrap();
    let mode = node.dht().mode().await.unwrap();
    assert_eq!(mode, Mode::Server);

    // Set to Client mode
    node.dht().set_mode(Mode::Client).await.unwrap();
    let mode = node.dht().mode().await.unwrap();
    assert_eq!(mode, Mode::Client);
}

#[tokio::test]
async fn test_dht_add_address() {
    let node = spawn_connexa_with_default_key().await;
    let peer_id = PeerId::random();
    let addr: Multiaddr = "/memory/1234".parse().unwrap();

    // Should not error when adding an address
    node.dht().add_address(peer_id, addr).await.unwrap();
}

#[tokio::test]
async fn test_dht_put_and_get() {
    let (node1, node2, peer_id1, _peer_id2) = create_connected_nodes().await;

    // Create a test key and value
    let key = "test-key";
    let value = b"test-value";

    // Node1 puts data into DHT
    node1
        .dht()
        .put(key, value.to_vec(), Quorum::One)
        .await
        .unwrap();

    // Node2 gets data from DHT
    let mut stream = node2.dht().get(key).await.unwrap();

    // Wait for at least one result
    let record = tokio::time::timeout(DEFAULT_TIMEOUT, stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert_eq!(record.record.value, value);
    assert_eq!(record.record.publisher, Some(peer_id1));
}

#[tokio::test]
async fn test_dht_provide_and_get_providers() {
    let (node1, node2, peer_id1, _peer_id2) = create_connected_nodes().await;

    let key = "provided-content";

    // Node1 announces it can provide the content
    node1.dht().provide(key).await.unwrap();

    // Node2 queries for providers
    let mut stream = node2.dht().get_providers(key).await.unwrap();

    // Wait for providers
    let providers = tokio::time::timeout(DEFAULT_TIMEOUT, stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert!(providers.contains(&peer_id1));
}

#[tokio::test]
async fn test_dht_stop_provide() {
    let node = spawn_connexa_with_default_key().await;
    let key = "stop-provide-key";

    // Start providing
    node.dht().provide(key).await.unwrap();

    // Stop providing should not error
    node.dht().stop_provide(key).await.unwrap();
}

#[tokio::test]
async fn test_dht_find_peer() {
    let (_node1, node2, peer_id1, _peer_id2) = create_connected_nodes().await;

    // Node2 tries to find Node1
    let peer_info = node2.dht().find_peer(peer_id1).await.unwrap();

    assert!(!peer_info.is_empty());

    assert!(peer_info.iter().any(|info| info.peer_id == peer_id1));
}

#[tokio::test]
async fn test_dht_listener() {
    let (node1, node2, _peer_id1, _peer_id2) = create_connected_nodes().await;

    // Listen for all DHT events (None means all keys)
    let mut listener = node2.dht().listener(()).await.unwrap();

    // Spawn a task to generate a DHT event
    let node_clone = node1.clone();
    tokio::spawn(async move {
        node_clone
            .dht()
            .put("listener-test", &b"data"[..], Quorum::One)
            .await
            .unwrap();
    });

    // Wait for an event with timeout
    let event = tokio::time::timeout(DEFAULT_TIMEOUT, listener.next())
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(event, DHTEvent::PutRecord { .. }))
}

#[tokio::test]
async fn test_dht_listener_with_specific_key() {
    let (node1, node2, _peer_id1, _peer_id2) = create_connected_nodes().await;
    let key = RecordKey::from(b"specific-key".to_vec());

    // Listen for events related to specific key
    let mut listener = node2.dht().listener(&key).await.unwrap();

    // Spawn a task to generate events
    let node_clone = node1.clone();
    let key_clone = key.clone();
    tokio::spawn(async move {
        node_clone
            .dht()
            .put(&key_clone, &b"data"[..], Quorum::One)
            .await
            .unwrap();
    });

    // Wait for an event with timeout
    let event = tokio::time::timeout(DEFAULT_TIMEOUT, listener.next())
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(event, DHTEvent::PutRecord { .. }));
}

#[tokio::test]
async fn test_dht_bootstrap_with_no_peers() {
    let node = spawn_connexa_with_default_key().await;

    let result = node.dht().bootstrap().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_dht_find_non_existent_peer() {
    let node = spawn_connexa_with_default_key().await;
    let random_peer = PeerId::random();

    // Finding a non-existent peer should return empty or error
    let result = node.dht().find_peer(random_peer).await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_dht_get_non_existent_key() {
    let node = spawn_connexa_with_default_key().await;

    let mut stream = node.dht().get("non-existent-key").await.unwrap();

    stream.next().await.unwrap().unwrap_err();
}
