use connexa::prelude::Protocol;
use futures::StreamExt;
use futures::future::poll_fn;
use rand::RngExt;
use std::task::Poll;

mod common;

#[tokio::test]
async fn relay_reservation() -> std::io::Result<()> {
    let [
        (_node1, _node1_peer_id, node1_addr),
        (node2, node2_peer_id, _node2_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<2>().await;

    let relay_addr = node1_addr.with(Protocol::P2pCircuit);

    node2.swarm().listen_on(relay_addr.clone()).await?;

    let addrs = node2.swarm().external_addresses().await?;

    let relay_addr = relay_addr.with(Protocol::P2p(node2_peer_id));

    assert!(addrs.contains(&relay_addr));

    Ok(())
}

#[tokio::test]
async fn relay_connection_to_peer() -> std::io::Result<()> {
    let [
        (_node1, _node1_peer_id, node1_addr),
        (node2, node2_peer_id, _node2_addr),
        (node3, node3_peer_id, _node3_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<3>().await;

    let relay_addr = node1_addr.with(Protocol::P2pCircuit);

    node2
        .swarm()
        .listen_on(relay_addr.clone())
        .await
        .expect("reservation is obtained");

    let addrs = node2.swarm().external_addresses().await?;

    let relay_addr = relay_addr.with(Protocol::P2p(node2_peer_id));

    assert!(addrs.contains(&relay_addr));

    node3.swarm().dial(relay_addr).await?;

    assert!(node3.swarm().is_connected(node2_peer_id).await?);
    assert!(node2.swarm().is_connected(node3_peer_id).await?);

    // test that we can send data between each peer

    let mut request_listener_2 = node2.request_response().listen_for_requests(()).await?;

    async_rt::task::dispatch({
        let node3 = node3.clone();
        async move {
            let data = "ping";
            let response = node3
                .request_response()
                .send_request(node2_peer_id, data)
                .await
                .unwrap();

            assert_eq!(response, "pong");
        }
    });

    if let Some((peer_id, id, request)) = request_listener_2.next().await {
        assert_eq!(request, "ping");
        assert_eq!(peer_id, node3_peer_id);
        node2
            .request_response()
            .send_response(peer_id, id, "pong")
            .await?;
    }

    Ok(())
}

#[tokio::test]
async fn relay_connection_to_peer_exceed_data_restriction() -> std::io::Result<()> {
    let [
        (_node1, _node1_peer_id, node1_addr),
        (node2, node2_peer_id, _node2_addr),
        (node3, node3_peer_id, _node3_addr),
    ] = common::spawn_connexa_nodes_with_default_keys::<3>().await;

    let relay_addr = node1_addr.with(Protocol::P2pCircuit);

    node2
        .swarm()
        .listen_on(relay_addr.clone())
        .await
        .expect("reservation is obtained");

    let addrs = node2.swarm().external_addresses().await?;

    let relay_addr = relay_addr.with(Protocol::P2p(node2_peer_id));

    assert!(addrs.contains(&relay_addr));

    node3.swarm().dial(relay_addr).await?;

    assert!(node3.swarm().is_connected(node2_peer_id).await?);
    assert!(node2.swarm().is_connected(node3_peer_id).await?);

    let mut request_listener_2 = node2.request_response().listen_for_requests(()).await?;

    let mut data = vec![0; 128 * 1024];
    rand::rng().fill(&mut data[..]);
    let err = node3
        .request_response()
        .send_request(node2_peer_id, data)
        .await
        .unwrap_err();

    assert!(matches!(err, connexa::error::Error::RequestResponse(_)));

    // test to make sure nothing was received
    let received_any_request =
        poll_fn(|cx| Poll::Ready(request_listener_2.poll_next_unpin(cx).is_ready())).await;

    assert!(!received_any_request);
    Ok(())
}
