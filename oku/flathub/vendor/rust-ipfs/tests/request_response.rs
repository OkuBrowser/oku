mod common;
use std::time::Duration;

use bytes::Bytes;
use common::{Topology, spawn_nodes};
use futures::StreamExt;
use futures_timeout::TimeoutExt;

#[tokio::test]
async fn send_request_to_peer() {
    let nodes = spawn_nodes::<2>(Topology::Line).await;

    let mut node_0_st = nodes[0].requests_subscribe(()).await.unwrap();
    let mut node_1_st = nodes[1].requests_subscribe(()).await.unwrap();

    let node_0_id = nodes[0].id;
    let node_1_id = nodes[1].id;

    tokio::spawn({
        let node_0 = nodes[0].clone();
        let node_1 = nodes[1].clone();
        async move {
            loop {
                tokio::select! {
                    Some((peer_id, id, request)) = node_0_st.next() => {
                        assert_eq!(peer_id, node_1_id);
                        assert_eq!(&request[..], &b"ping"[..]);
                        let res = Bytes::copy_from_slice(b"pong");
                        node_0.send_response(peer_id, id, res).await.expect("able to response");
                    },
                    Some((peer_id, id, request)) = node_1_st.next() => {
                        assert_eq!(peer_id, node_0_id);
                        assert_eq!(&request[..], &b"ping"[..]);
                        let res = Bytes::copy_from_slice(b"pong");
                        node_1.send_response(peer_id, id, res).await.expect("able to response");
                    },
                }
            }
        }
    });

    let response = nodes[0]
        .send_request(node_1_id, b"ping")
        .timeout(Duration::from_secs(5))
        .await
        .expect("respond in time")
        .expect("valid response");
    assert_eq!(&response[..], &b"pong"[..]);

    let response = nodes[1]
        .send_request(node_0_id, b"ping")
        .timeout(Duration::from_secs(5))
        .await
        .expect("respond in time")
        .expect("valid response");
    assert_eq!(&response[..], &b"pong"[..]);
}

#[tokio::test]
async fn fail_to_respond_to_request() {
    let nodes = spawn_nodes::<2>(Topology::Line).await;

    let node_1_id = nodes[1].id;

    let response = nodes[0]
        .send_request(node_1_id, b"ping")
        .timeout(Duration::from_secs(5))
        .await
        .expect("respond in time");

    assert!(response.is_err());
}
