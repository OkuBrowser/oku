use futures::{StreamExt, pin_mut};
use ipld_core::cid::Cid;
use multihash_codetable::{Code, MultihashDigest};
use rust_ipfs::{Block, Multiaddr, Node, Protocol, Quorum, p2p::MultiaddrExt};

use std::time::Duration;

mod common;
use common::{Topology, spawn_nodes};
use rust_ipfs::block::BlockCodec;

fn strip_peer_id(mut addr: Multiaddr) -> Multiaddr {
    addr.extract_peer_id().expect("Peer id exist");
    addr
}

/// Check if `Ipfs::find_peer` works without DHT involvement.
#[tokio::test]
async fn find_peer_local() {
    let nodes = spawn_nodes::<2>(Topology::None).await;
    nodes[0].connect(nodes[1].addrs[0].clone()).await.unwrap();

    // while nodes[0] is connected to nodes[1], they know each
    // other's addresses and can find them without using the DHT
    let mut found_addrs = nodes[0].find_peer(nodes[1].id).await.unwrap();

    for addr in &mut found_addrs {
        addr.push(Protocol::P2p(nodes[1].id));
        assert!(nodes[1].addrs.contains(addr));
    }
}

// starts the specified number of rust IPFS nodes connected in a chain.
async fn spawn_bootstrapped_nodes<const N: usize>() -> Vec<Node> {
    let nodes = spawn_nodes::<N>(Topology::None).await;

    // register the nodes' addresses so they can bootstrap against
    // one another in a chain; node 0 is only aware of the existence
    // of node 1 and so on; bootstrap them eagerly/quickly, so that
    // they don't have a chance to form the full picture in the DHT
    for i in 0..N {
        let (next_id, next_addr) = if i < N - 1 {
            (nodes[i + 1].id, nodes[i + 1].addrs[0].clone())
        } else {
            (nodes[N - 2].id, nodes[N - 2].addrs[0].clone())
        };

        let addr = next_addr.with(Protocol::P2p(next_id));
        nodes[i].add_bootstrap(addr).await.unwrap();
        nodes[i].bootstrap().await.unwrap();
    }

    nodes
}

/// Check if `Ipfs::find_peer` works using DHT.
#[tokio::test]
#[ignore = "will be reevaluated"]
async fn dht_find_peer() {
    // works for numbers >=2, though 2 would essentially just
    // be the same as find_peer_local, so it should be higher
    const CHAIN_LEN: usize = 10;
    let nodes = spawn_bootstrapped_nodes::<CHAIN_LEN>().await;
    let last_index = CHAIN_LEN - 1;

    // node 0 now tries to find the address of the very last node in the
    // chain; the chain should be long enough for it not to automatically
    // be connected to it after the bootstrap
    let found_addrs = nodes[0].find_peer(nodes[last_index].id).await.unwrap();

    let to_be_found = strip_peer_id(nodes[last_index].addrs[0].clone());
    assert_eq!(found_addrs, vec![to_be_found]);
}

#[tokio::test]
async fn dht_get_closest_peers() {
    const CHAIN_LEN: usize = 10;
    let nodes = spawn_bootstrapped_nodes::<CHAIN_LEN>().await;

    assert_eq!(
        nodes[0].get_closest_peers(nodes[0].id).await.unwrap().len(),
        CHAIN_LEN - 1
    );
}

#[ignore = "targets an actual bootstrapper, so random failures can happen"]
#[tokio::test]
async fn dht_popular_content_discovery() {
    let peer = Node::new("a").await;

    peer.default_bootstrap().await.unwrap();

    // the Cid of the IPFS logo
    let cid: Cid = "bafkreicncneocapbypwwe3gl47bzvr3pkpxmmobzn7zr2iaz67df4kjeiq"
        .parse()
        .unwrap();

    assert!(
        peer.get_block(cid)
            .timeout(Duration::from_secs(10))
            .await
            .is_ok()
    );
}

/// Check if Ipfs::{get_providers, provide} does its job.
#[tokio::test]
async fn dht_providing() {
    const CHAIN_LEN: usize = 10;
    let nodes = spawn_bootstrapped_nodes::<CHAIN_LEN>().await;
    let last_index = CHAIN_LEN - 1;

    // the last node puts a block in order to have something to provide
    let data = b"hello block\n".to_vec();
    let cid = Cid::new_v1(BlockCodec::Raw.into(), Code::Sha2_256.digest(&data));
    nodes[last_index]
        .put_block(&Block::new(cid, data).unwrap())
        .await
        .unwrap();

    // the last node then provides the Cid
    nodes[last_index].provide(cid).await.unwrap();

    // and the first node should be able to learn that the last one provides it
    let providers = nodes[0].get_providers(cid).await.unwrap().boxed();

    assert!(
        providers
            .take(1)
            .filter_map(|result| async move { result.ok() })
            .map(futures::stream::iter)
            .flatten()
            .collect::<Vec<_>>()
            .await
            .iter()
            .any(|x| *x == nodes[last_index].id)
    );
}

#[tokio::test]
async fn bitswap_fetch_via_dht_discovery() {
    const CHAIN_LEN: usize = 50;
    let nodes = spawn_bootstrapped_nodes::<CHAIN_LEN>().await;
    let last_index = CHAIN_LEN - 1;

    // the last node holds and provides a block; node 0 is not directly connected to it.
    let data = b"phase-0 dht discovery\n".to_vec();
    let cid = Cid::new_v1(BlockCodec::Raw.into(), Code::Sha2_256.digest(&data));

    let block = Block::new(cid, data).unwrap();

    nodes[last_index].put_block(&block).await.unwrap();
    nodes[last_index].provide(cid).await.unwrap();

    // node 0 must discover the provider via the DHT and fetch through bitswap.
    let new_block = nodes[0]
        .get_block(cid)
        .timeout(Duration::from_secs(30))
        .await
        .expect("block should be fetched via DHT provider discovery");
    assert_eq!(new_block.data(), block.data());
}

/// Check if Ipfs::{get, put} does its job.
#[tokio::test]
async fn dht_get_put() {
    const CHAIN_LEN: usize = 10;
    let nodes = spawn_bootstrapped_nodes::<CHAIN_LEN>().await;
    let last_index = CHAIN_LEN - 1;

    let (key, value) = (b"key".to_vec(), b"value".to_vec());
    let quorum = Quorum::One;

    // the last node puts a key+value record
    nodes[last_index]
        .dht_put(key.clone(), value.clone(), quorum)
        .await
        .unwrap();

    // and the first node should be able to get it
    let records = nodes[0].dht_get(key).await.unwrap();
    pin_mut!(records);

    // assert_eq!(nodes[0].dht_get(key, quorum).await.unwrap(), vec![value]);
    assert!(
        records
            .by_ref()
            .take(1)
            .collect::<Vec<_>>()
            .await
            .iter()
            .any(|x| x.value == value)
    );
}
