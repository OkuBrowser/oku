use futures::StreamExt;
use futures_timeout::TimeoutExt;
use std::time::Duration;

mod common;

#[tokio::test]
async fn send_request_to_peer() {
    let [
        (node1, node1_peer_id, _node1_addr),
        (node2, node2_peer_id, node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    node1.swarm().dial(node2_addr).await.unwrap();

    let mut node_1_st = node1
        .request_response()
        .listen_for_requests(())
        .await
        .unwrap();
    let mut node_2_st = node2
        .request_response()
        .listen_for_requests(())
        .await
        .unwrap();

    async_rt::task::dispatch({
        let node_0 = node1.clone();
        let node_1 = node2.clone();
        async move {
            loop {
                tokio::select! {
                    Some((peer_id, id, request)) = node_1_st.next() => {
                        assert_eq!(peer_id, node2_peer_id);
                        assert_eq!(request, "ping");
                        node_0.request_response().send_response(peer_id, id, "pong").await.expect("able to response");
                    },
                    Some((peer_id, id, request)) = node_2_st.next() => {
                        assert_eq!(peer_id, node1_peer_id);
                        assert_eq!(request, "ping");
                        node_1.request_response().send_response(peer_id, id, "pong").await.expect("able to response");
                    },
                }
            }
        }
    });

    let response = node1
        .request_response()
        .send_request(node2_peer_id, b"ping")
        .timeout(Duration::from_secs(5))
        .await
        .expect("respond in time")
        .expect("valid response");
    assert_eq!(response, "pong");

    let response = node2
        .request_response()
        .send_request(node1_peer_id, b"ping")
        .timeout(Duration::from_secs(5))
        .await
        .expect("respond in time")
        .expect("valid response");
    assert_eq!(response, "pong");
}

#[tokio::test]
async fn fail_to_respond_to_request() {
    let [
        (node1, _node1_peer_id, _node1_addr),
        (_node2, node2_peer_id, node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    node1.swarm().dial(node2_addr).await.unwrap();

    node1
        .request_response()
        .send_request(node2_peer_id, b"ping")
        .await
        .unwrap_err();
}
