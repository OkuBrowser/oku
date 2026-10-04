use connexa::prelude::GossipsubEvent;
use connexa::prelude::gossipsub::{IdentTopic, IntoGossipsubTopic, TopicHash};
use futures::future::pending;
use futures::stream::{BoxStream, StreamExt};
use futures::{Stream, TryFutureExt};
use futures_timeout::TimeoutExt;
use rust_ipfs::{Node, PeerId};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

mod common;
use common::{Topology, spawn_nodes};

#[tokio::test]
async fn subscribe_only_once() {
    let a = Node::new("test_node").await;
    a.pubsub_subscribe("some_topic").await.unwrap();
}

// #[tokio::test]
// async fn subscribe_multiple_times() {
//     let a = Node::new("test_node").await;
//     let _stream = a.pubsub_subscribe("some_topic").await.unwrap();
//     a.pubsub_subscribe("some_topic").await.unwrap();
// }

#[tokio::test]
async fn resubscribe_after_unsubscribe() {
    let a = Node::new("test_node").await;
    a.pubsub_subscribe("topic").await.unwrap();
    a.pubsub_unsubscribe("topic").await.unwrap();
    a.pubsub_subscribe("topic").await.unwrap();
}

// #[tokio::test]
// async fn unsubscribe_cloned_via_drop() {
//     let empty: &[&str] = &[];
//     let a = Node::new("test_node").await;

//     let msgs_1 = a.pubsub_subscribe("topic").await.unwrap();
//     let msgs_2 = a.pubsub_subscribe("topic").await.unwrap();

//     drop(msgs_1);

//     assert_ne!(a.pubsub_subscribed().await.unwrap(), empty);

//     drop(msgs_2);

//     assert_eq!(a.pubsub_subscribed().await.unwrap(), empty);
// }

// #[tokio::test]
// async fn unsubscribe_via_drop() {
//     let a = Node::new("test_node").await;
//
//     a.pubsub_subscribe("topic").await.unwrap();
//     assert_eq!(a.pubsub_subscribed().await.unwrap(), &["topic"]);
//
//     drop(msgs);
//
//     let empty: &[&str] = &[];
//     assert_eq!(a.pubsub_subscribed().await.unwrap(), empty);
// }

#[tokio::test]
async fn publish_between_two_nodes_single_topic() {
    use futures::stream::StreamExt;

    let nodes = spawn_nodes::<2>(Topology::Line).await;

    let topic = "shared".to_owned();

    let mut a_msgs = nodes[0]
        .pubsub_subscribe(&topic)
        .and_then(|_| nodes[0].pubsub_listener(&topic))
        .await
        .map(|st| PubsubStream::new(&topic, st))
        .unwrap();
    let mut b_msgs = nodes[1]
        .pubsub_subscribe(&topic)
        .and_then(|_| nodes[1].pubsub_listener(&topic))
        .await
        .map(|st| PubsubStream::new(&topic, st))
        .unwrap();

    // need to wait to see both sides so that the messages will get through
    let mut appeared = false;
    for _ in 0..100usize {
        if nodes[0]
            .pubsub_peers(&topic)
            .await
            .unwrap()
            .contains(&nodes[1].id)
            && nodes[1]
                .pubsub_peers(&topic)
                .await
                .unwrap()
                .contains(&nodes[0].id)
        {
            appeared = true;
            break;
        }
        pending::<()>()
            .timeout(Duration::from_millis(100))
            .await
            .unwrap_err();
    }

    assert!(
        appeared,
        "timed out before both nodes appeared as pubsub peers"
    );

    nodes[0]
        .pubsub_publish(&topic, b"foobar".to_vec())
        .await
        .unwrap();
    nodes[1]
        .pubsub_publish(&topic, b"barfoo".to_vec())
        .await
        .unwrap();

    let expected = [
        (
            IdentTopic::new(topic.clone()),
            Some(nodes[0].id),
            b"foobar",
            nodes[1].id,
        ),
        (
            IdentTopic::new(topic.clone()),
            Some(nodes[1].id),
            b"barfoo",
            nodes[0].id,
        ),
    ]
    .into_iter()
    .map(|(topic, sender, data, witness)| (topic.hash(), sender, data.to_vec(), witness))
    .collect::<Vec<_>>();

    let mut actual = Vec::new();

    for (st, own_peer_id) in &mut [
        (b_msgs.by_ref(), nodes[1].id),
        (a_msgs.by_ref(), nodes[0].id),
    ] {
        let received = st
            .take(1)
            .map(|msg| (msg.topic, msg.source, msg.data, *own_peer_id))
            .collect::<Vec<_>>()
            .timeout(Duration::from_secs(2))
            .await
            .unwrap();

        actual.extend(received);
    }

    // sort the received messages both in expected and actual to make sure they are comparable;
    // order of receiving is not part of the tuple and shouldn't matter.
    let mut expected = expected;
    expected.sort_unstable();
    actual.sort_unstable();

    assert_eq!(
        actual, expected,
        "sent and received messages must be present on both nodes' streams"
    );

    drop(b_msgs);

    nodes[1].pubsub_unsubscribe(&topic).await.unwrap();

    let mut disappeared = false;
    for _ in 0..100usize {
        if !nodes[0]
            .pubsub_peers(&topic)
            .await
            .unwrap()
            .contains(&nodes[1].id)
        {
            disappeared = true;
            break;
        }
        pending::<()>()
            .timeout(Duration::from_millis(100))
            .await
            .unwrap_err();
    }

    assert!(disappeared, "timed out before a saw b's unsubscription");
}

#[tokio::test]
async fn pubsub_event() {
    use futures::stream::StreamExt;

    let nodes = spawn_nodes::<2>(Topology::Line).await;
    let node_a = &nodes[0];
    let node_a_peer_id = node_a.id;
    let node_b = &nodes[1];
    let node_b_peer_id = node_b.id;

    let mut ev_a = node_a.pubsub_listener("test0".to_string()).await.unwrap();
    let mut ev_b = node_b.pubsub_listener("test0".to_string()).await.unwrap();

    node_a.pubsub_subscribe("test0").await.unwrap();
    node_b.pubsub_subscribe("test0").await.unwrap();

    let next_ev_a = ev_a.next().await.unwrap();
    let next_ev_b = ev_b.next().await.unwrap();

    assert_eq!(
        next_ev_a,
        GossipsubEvent::Subscribed {
            peer_id: node_b_peer_id,
        }
    );
    assert_eq!(
        next_ev_b,
        GossipsubEvent::Subscribed {
            peer_id: node_a_peer_id,
        }
    );
}

#[tokio::test]
async fn publish_between_two_nodes_different_topics() {
    use futures::stream::StreamExt;

    let nodes = spawn_nodes::<2>(Topology::Line).await;
    let node_a = &nodes[0];
    let node_b = &nodes[1];

    let topic_a = "shared-a".to_owned();
    let topic_b = "shared-b".to_owned();

    // Node A subscribes to Topic B
    // Node B subscribes to Topic A
    let mut a_msgs = node_a
        .pubsub_subscribe(&topic_b)
        .and_then(|_| node_a.pubsub_listener(&topic_b))
        .await
        .map(|st| PubsubStream::new(&topic_b, st))
        .unwrap();
    let mut b_msgs = node_b
        .pubsub_subscribe(&topic_a)
        .and_then(|_| node_b.pubsub_listener(&topic_a))
        .await
        .map(|st| PubsubStream::new(&topic_a, st))
        .unwrap();

    // need to wait to see both sides so that the messages will get through
    let mut appeared = false;
    for _ in 0..100usize {
        if node_a
            .pubsub_peers(&topic_a)
            .await
            .unwrap()
            .contains(&node_b.id)
            && node_b
                .pubsub_peers(&topic_b)
                .await
                .unwrap()
                .contains(&node_a.id)
        {
            appeared = true;
            break;
        }
        pending::<()>()
            .timeout(Duration::from_millis(100))
            .await
            .unwrap_err();
    }

    assert!(
        appeared,
        "timed out before both nodes appeared as pubsub peers"
    );

    // Each node publishes to their own topic
    node_a
        .pubsub_publish(topic_a.clone(), b"foobar".to_vec())
        .await
        .unwrap();
    node_b
        .pubsub_publish(topic_b.clone(), b"barfoo".to_vec())
        .await
        .unwrap();

    // the order between messages is not defined, but both should see the other's message. since we
    // receive messages first from node_b's stream we expect this order.
    //
    // in this test case the nodes are not expected to see their own message because nodes are not
    // subscribing to the streams they are sending to.
    let expected = [
        (
            IdentTopic::new(topic_a.clone()),
            Some(node_a.id),
            b"foobar",
            node_b.id,
        ),
        (
            IdentTopic::new(topic_b.clone()),
            Some(node_b.id),
            b"barfoo",
            node_a.id,
        ),
    ]
    .into_iter()
    .map(|(topic, sender, data, witness)| (topic.hash(), sender, data.to_vec(), witness))
    .collect::<Vec<_>>();

    let mut actual = Vec::new();
    for (st, own_peer_id) in &mut [(b_msgs.by_ref(), node_b.id), (a_msgs.by_ref(), node_a.id)] {
        let received = st
            .take(1)
            .map(|msg| (msg.topic, msg.source, msg.data, *own_peer_id))
            .next()
            .timeout(Duration::from_secs(2))
            .await
            .unwrap()
            .unwrap();
        actual.push(received);
    }

    // ordering is defined for expected and actual by the order of the looping above and the
    // initial expected creation.
    assert_eq!(expected, actual);

    node_b.pubsub_unsubscribe(&topic_a).await.unwrap();

    let mut disappeared = false;
    for _ in 0..100usize {
        if !node_a
            .pubsub_peers(topic_a.clone())
            .await
            .unwrap()
            .contains(&node_b.id)
        {
            disappeared = true;
            break;
        }
        pending::<()>()
            .timeout(Duration::from_millis(100))
            .await
            .unwrap_err();
    }

    assert!(disappeared, "timed out before a saw b's unsubscription");
}

struct PubsubStream {
    topic: TopicHash,
    st: BoxStream<'static, GossipsubEvent>,
}

impl PubsubStream {
    pub fn new(topic: impl IntoGossipsubTopic, st: BoxStream<'static, GossipsubEvent>) -> Self {
        let topic = topic.into_topic();
        Self { topic, st }
    }
}

pub struct PubsubMessage {
    pub source: Option<PeerId>,
    pub data: Vec<u8>,
    pub sequence_number: Option<u64>,
    pub topic: TopicHash,
}

impl Stream for PubsubStream {
    type Item = PubsubMessage;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match futures::ready!(self.st.poll_next_unpin(cx)) {
                Some(ev) => match ev {
                    GossipsubEvent::Message { message } => {
                        return Poll::Ready(Some(PubsubMessage {
                            source: message.source,
                            data: message.data.to_vec(),
                            sequence_number: message.sequence_number,
                            topic: self.topic.clone(),
                        }));
                    }
                    _ => continue,
                },
                None => return Poll::Ready(None),
            }
        }
    }
}
