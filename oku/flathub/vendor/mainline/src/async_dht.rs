//! AsyncDht node.

use std::{
    net::SocketAddrV4,
    pin::Pin,
    task::{Context, Poll},
};

use futures_lite::{Stream, StreamExt};

use crate::{
    common::{
        hash_immutable, most_recent_mutable_item, AnnouncePeerRequestArguments,
        FindNodeRequestArguments, GetPeersRequestArguments, GetValueRequestArguments, Id,
        MutableItem, Node, PutImmutableRequestArguments, PutMutableRequestArguments,
        PutRequestSpecific,
    },
    dht::{ActorMessage, Dht, PutMutableError, ResponseSender},
    rpc::{GetMutableOutcome, GetRequestSpecific, Info, PutError, PutOutcome, PutQueryError},
};

impl Dht {
    /// Return an async version of the Dht client.
    pub fn as_async(self) -> AsyncDht {
        AsyncDht(self)
    }
}

#[derive(Debug, Clone)]
/// Async version of the Dht node.
pub struct AsyncDht(Dht);

impl AsyncDht {
    /// Information and statistics about this [Dht] node.
    pub async fn info(&self) -> Info {
        let (tx, rx) = flume::bounded::<Info>(1);
        self.send(ActorMessage::Info(tx));

        rx.recv_async()
            .await
            .expect("actor thread unexpectedly shutdown")
    }

    /// Turn this node's routing table to a list of bootstrapping nodes.   
    pub async fn to_bootstrap(&self) -> Vec<String> {
        let (tx, rx) = flume::bounded::<Vec<String>>(1);
        self.send(ActorMessage::ToBootstrap(tx));

        rx.recv_async()
            .await
            .expect("actor thread unexpectedly shutdown")
    }

    // === Public Methods ===

    /// Await until the bootstrapping query is done.
    ///
    /// Returns true if the bootstrapping was successful.
    pub async fn bootstrapped(&self) -> bool {
        let info = self.info().await;
        let nodes = self.find_node(*info.id()).await;

        !nodes.is_empty()
    }

    // === Find nodes ===

    /// Returns the closest 20 [secure](Node::is_secure) nodes to a target [Id].
    ///
    /// Mostly useful to crawl the DHT.
    ///
    /// The returned nodes are claims by other nodes, they may be lies, or may have churned
    /// since they were last seen, but haven't been pinged yet.
    ///
    /// You might need to ping them to confirm they exist, and responsive, or if you want to
    /// learn more about them like the client they are using, or if they support a given BEP.
    ///
    /// If you are trying to find the closest nodes to a target with intent to [Self::put],
    /// a request directly to these nodes (using `extra_nodes` parameter), then you should
    /// use [Self::get_closest_nodes] instead.
    pub async fn find_node(&self, target: Id) -> Box<[Node]> {
        let (tx, rx) = flume::bounded::<Box<[Node]>>(1);
        self.send(ActorMessage::Get(
            GetRequestSpecific::FindNode(FindNodeRequestArguments { target }),
            ResponseSender::ClosestNodes(tx),
        ));

        rx.recv_async()
            .await
            .expect("Query was dropped before sending a response, please open an issue.")
    }

    // === Peers ===

    /// Get peers for a given infohash.
    ///
    /// Note: each node of the network will only return a _random_ subset (usually 20)
    /// of the total peers it has for a given infohash, so if you are getting responses
    /// from 20 nodes, you can expect up to 400 peers in total, but if there are more
    /// announced peers on that infohash, you are likely to miss some, the logic here
    /// for Bittorrent is that any peer will introduce you to more peers through "peer exchange"
    /// so if you are implementing something different from Bittorrent, you might want
    /// to implement your own logic for gossipping more peers after you discover the first ones.
    pub fn get_peers(&self, info_hash: Id) -> GetStream<Vec<SocketAddrV4>> {
        let (tx, rx) = flume::unbounded::<Vec<SocketAddrV4>>();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetPeers(GetPeersRequestArguments { info_hash }),
            ResponseSender::Peers(tx),
        ));

        GetStream(rx.into_stream())
    }

    /// Announce a peer for a given infohash.
    ///
    /// The peer will be announced on this process IP.
    /// If explicit port is passed, it will be used, otherwise the port will be implicitly
    /// assumed by remote nodes to be the same ase port they received the request from.
    pub async fn announce_peer(
        &self,
        info_hash: Id,
        port: Option<u16>,
    ) -> Result<Id, PutQueryError> {
        let (port, implied_port) = match port {
            Some(port) => (port, None),
            None => (0, Some(true)),
        };

        self.put(
            PutRequestSpecific::AnnouncePeer(AnnouncePeerRequestArguments {
                info_hash,
                port,
                implied_port,
            }),
            None,
        )
        .await
        .map(|outcome| outcome.target)
        .map_err(|error| match error {
            PutError::Query(error) => error,
            PutError::Concurrency(_) => {
                unreachable!("should not receive a concurrency error from announce peer query")
            }
        })
    }

    // === Immutable data ===

    /// Get an Immutable data by its sha1 hash.
    pub async fn get_immutable(&self, target: Id) -> Option<Box<[u8]>> {
        let (tx, rx) = flume::unbounded::<Box<[u8]>>();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                seq: None,
                salt: None,
            }),
            ResponseSender::Immutable(tx),
        ));

        rx.recv_async().await.map(Some).unwrap_or(None)
    }

    /// Put an immutable data to the DHT.
    pub async fn put_immutable(&self, value: &[u8]) -> Result<Id, PutQueryError> {
        let target: Id = hash_immutable(value).into();

        self.put(
            PutRequestSpecific::PutImmutable(PutImmutableRequestArguments {
                target,
                v: value.into(),
            }),
            None,
        )
        .await
        .map(|outcome| outcome.target)
        .map_err(|error| match error {
            PutError::Query(error) => error,
            PutError::Concurrency(_) => {
                unreachable!("should not receive a concurrency error from put immutable query")
            }
        })
    }

    // === Mutable data ===

    /// Get a mutable data by its `public_key` and optional `salt`.
    ///
    /// You can ask for items `more_recent_than` than a certain `seq`,
    /// usually one that you already have seen before, similar to `If-Modified-Since` header in HTTP.
    ///
    /// # Order
    ///
    /// The order of [MutableItem]s returned by this stream is not guaranteed to
    /// reflect their `seq` value. You should not assume that the later items are
    /// more recent than earlier ones.
    ///
    /// Consider using [Self::get_mutable_most_recent] if that is what you need.
    pub fn get_mutable(
        &self,
        public_key: &[u8; 32],
        salt: Option<&[u8]>,
        more_recent_than: Option<i64>,
    ) -> GetStream<MutableItem> {
        self.get_mutable_detailed(public_key, salt, more_recent_than)
            .items
    }

    /// Get mutable data and final aggregate diagnostics for the lookup.
    ///
    /// Valid mutable items are yielded on [GetMutableDetailed::items] as they arrive.
    /// Final counters are available by awaiting [GetMutableOutcomeReceiver::recv].
    pub fn get_mutable_detailed(
        &self,
        public_key: &[u8; 32],
        salt: Option<&[u8]>,
        more_recent_than: Option<i64>,
    ) -> GetMutableDetailed {
        let salt = salt.map(Into::into);
        let target = MutableItem::target_from_key(public_key, salt.as_deref());
        let (values_tx, values_rx) = flume::unbounded::<MutableItem>();
        let (outcome_tx, outcome_rx) = flume::bounded::<GetMutableOutcome>(1);

        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                seq: more_recent_than,
                salt,
            }),
            ResponseSender::MutableDetailed {
                values: values_tx,
                outcome: outcome_tx,
            },
        ));

        GetMutableDetailed {
            items: GetStream(values_rx.into_stream()),
            outcome: GetMutableOutcomeReceiver(outcome_rx),
        }
    }

    /// Get the most recent [MutableItem] from the network.
    pub async fn get_mutable_most_recent(
        &self,
        public_key: &[u8; 32],
        salt: Option<&[u8]>,
    ) -> Option<MutableItem> {
        self.get_mutable(public_key, salt, None)
            .fold(None::<MutableItem>, most_recent_mutable_item)
            .await
    }

    /// Put a mutable data to the DHT.
    ///
    /// Returns details about the successful PUT, including how many DHT nodes acknowledged it.
    ///
    /// # Lost Update Problem
    ///
    /// As mainline DHT is a distributed system, it is vulnerable to [Write–write conflict](https://en.wikipedia.org/wiki/Write-write_conflict).
    ///
    /// ## Read first
    ///
    /// To mitigate the risk of lost updates, you should call the [Self::get_mutable_most_recent] method
    /// then start authoring the new [MutableItem] based on the most recent as in the following example:
    ///
    ///```rust
    /// use mainline::{Dht, MutableItem, SigningKey, Testnet};
    /// use std::net::Ipv4Addr;
    ///
    /// let testnet = Testnet::builder(3).build().unwrap();
    /// let dht = Dht::builder()
    ///     .bootstrap(&testnet.bootstrap)
    ///     .bind_address(Ipv4Addr::LOCALHOST)
    ///     .build()
    ///     .unwrap()
    ///     .as_async();
    ///
    /// let signing_key = SigningKey::from_bytes(&[0; 32]);
    /// let key = signing_key.verifying_key().to_bytes();
    /// let salt = Some(b"salt".as_ref());
    ///
    /// futures::executor::block_on(async move {
    ///     let (item, cas) = if let Some(most_recent) = dht.get_mutable_most_recent(&key, salt).await {
    ///         // 1. Optionally Create a new value to take the most recent's value in consideration.
    ///         let mut new_value = most_recent.value().to_vec();
    ///         new_value.extend_from_slice(b" more data");
    ///
    ///         // 2. Increment the sequence number to be higher than the most recent's.
    ///         let most_recent_seq = most_recent.seq();
    ///         let new_seq = most_recent_seq + 1;
    ///
    ///         (
    ///             MutableItem::new(signing_key, &new_value, new_seq, salt),
    ///             // 3. Use the most recent [MutableItem::seq] as a `CAS`.
    ///             Some(most_recent_seq)
    ///         )
    ///     } else {
    ///         (MutableItem::new(signing_key, b"first value", 1, salt), None)
    ///     };
    ///
    ///     let target = *item.target();
    ///     let outcome = dht.put_mutable(item, cas).await.unwrap();
    ///     assert_eq!(outcome.target, target);
    /// });
    /// ```
    ///
    /// ## Errors
    ///
    /// In addition to the [PutQueryError] common with all PUT queries, PUT mutable item
    /// query has other [Concurrency errors][crate::rpc::ConcurrencyError], that try to detect write conflict
    /// risks or obvious conflicts.
    ///
    /// If you are lucky to get one of these errors (which is not guaranteed), then you should
    /// read the most recent item again, and repeat the steps in the previous example.
    pub async fn put_mutable(
        &self,
        item: MutableItem,
        cas: Option<i64>,
    ) -> Result<PutOutcome, PutMutableError> {
        let request = PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(item, cas));

        self.put(request, None).await.map_err(|error| match error {
            PutError::Query(err) => PutMutableError::Query(err),
            PutError::Concurrency(err) => PutMutableError::Concurrency(err),
        })
    }

    // === Raw ===

    /// Get closet nodes to a specific target, that support [BEP_0044](https://www.bittorrent.org/beps/bep_0044.html).
    ///
    /// Useful to [Self::put] a request to nodes further from the 20 closest nodes to the
    /// [PutRequestSpecific::target]. Which itself is useful to circumvent [extreme vertical sybil attacks](https://github.com/pubky/mainline/blob/main/docs/censorship-resistance.md#extreme-vertical-sybil-attacks).
    pub async fn get_closest_nodes(&self, target: Id) -> Box<[Node]> {
        let (tx, rx) = flume::unbounded::<Box<[Node]>>();
        self.send(ActorMessage::Get(
            GetRequestSpecific::GetValue(GetValueRequestArguments {
                target,
                salt: None,
                seq: None,
            }),
            ResponseSender::ClosestNodes(tx),
        ));

        rx.recv_async()
            .await
            .expect("Query was dropped before sending a response, please open an issue.")
    }

    /// Send a PUT request to the closest nodes, and optionally some extra nodes.
    ///
    /// This is useful to put data to regions of the DHT other than the closest nodes
    /// to this request's [target][PutRequestSpecific::target].
    ///
    /// You can find nodes close to other regions of the network by calling
    /// [Self::get_closest_nodes] with the target that you want to find the closest nodes to.
    ///
    /// Note: extra nodes need to have [Node::valid_token].
    ///
    /// Returns details about the successful PUT.
    pub async fn put(
        &self,
        request: PutRequestSpecific,
        extra_nodes: Option<Box<[Node]>>,
    ) -> Result<PutOutcome, PutError> {
        self.put_inner(request, extra_nodes)
            .recv_async()
            .await
            .expect("Query was dropped before sending a response, please open an issue.")
    }

    // === Private Methods ===

    pub(crate) fn put_inner(
        &self,
        request: PutRequestSpecific,
        extra_nodes: Option<Box<[Node]>>,
    ) -> flume::Receiver<Result<PutOutcome, PutError>> {
        let (tx, rx) = flume::bounded::<Result<PutOutcome, PutError>>(1);
        self.send(ActorMessage::Put(request, tx, extra_nodes));

        rx
    }

    fn send(&self, message: ActorMessage) {
        self.0.send(message)
    }
}

/// A [Stream] of incoming peers, immutable or mutable values.
pub struct GetStream<T: 'static>(flume::r#async::RecvStream<'static, T>);

impl<T> Stream for GetStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        this.0.poll_next(cx)
    }
}

/// Mutable GET values stream plus a final diagnostic outcome.
pub struct GetMutableDetailed {
    /// Valid mutable items returned by queried DHT nodes.
    pub items: GetStream<MutableItem>,
    /// Final aggregate diagnostics for the lookup.
    pub outcome: GetMutableOutcomeReceiver,
}

/// Async receiver for a detailed mutable GET outcome.
pub struct GetMutableOutcomeReceiver(flume::Receiver<GetMutableOutcome>);

impl GetMutableOutcomeReceiver {
    /// Wait for the mutable GET query to finish and return its aggregate diagnostics.
    pub async fn recv(self) -> GetMutableOutcome {
        self.0
            .recv_async()
            .await
            .expect("Query was dropped before sending a response, please open an issue.")
    }
}

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;
    use std::{str::FromStr, time::Duration};

    use ed25519_dalek::SigningKey;
    use futures::StreamExt;

    use crate::{dht::Testnet, rpc::ConcurrencyError};

    use super::*;

    fn counted_mutable_responses(outcome: &GetMutableOutcome) -> u32 {
        outcome.values
            + outcome.no_values
            + outcome.no_more_recent
            + outcome.invalid_values
            + outcome.invalid_responses
            + outcome.krpc_errors
    }

    fn counted_valid_mutable_responses(outcome: &GetMutableOutcome) -> u32 {
        outcome.values + outcome.no_values + outcome.no_more_recent
    }

    #[test]
    fn announce_get_peer() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let a = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();
            let b = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let info_hash = Id::random();

            a.announce_peer(info_hash, Some(45555))
                .await
                .expect("failed to announce");

            let peers = b.get_peers(info_hash).next().await.expect("No peers");

            assert_eq!(peers.first().unwrap().port(), 45555);
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn put_get_immutable() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let a = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();
            let b = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let value = b"Hello World!";
            let expected_target = Id::from_str("e5f96f6f38320f0f33959cb4d3d656452117aadb").unwrap();

            let target = a.put_immutable(value).await.unwrap();
            assert_eq!(target, expected_target);

            let response = b.get_immutable(target).await;
            assert_eq!(response, Some(value.to_vec().into_boxed_slice()));
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn raw_put_immutable_returns_outcome() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let dht = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let value = b"Hello World!";
            let target: Id = hash_immutable(value).into();
            let request = PutRequestSpecific::PutImmutable(PutImmutableRequestArguments {
                target,
                v: value.as_ref().into(),
            });

            let outcome = dht.put(request, None).await.unwrap();

            assert_eq!(outcome.target, target);
            assert!(outcome.stored_at > 0);
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn put_get_mutable() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let a = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();
            let b = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let seq = 1000;
            let value = b"Hello World!";

            let item = MutableItem::new(signer.clone(), value, seq, None);
            let target = *item.target();

            let outcome = a.put_mutable(item.clone(), None).await.unwrap();

            assert_eq!(outcome.target, target);
            assert!(outcome.stored_at > 0);

            let response = b
                .get_mutable(signer.verifying_key().as_bytes(), None, None)
                .next()
                .await
                .expect("No mutable values");

            assert_eq!(&response, &item);
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn get_mutable_detailed_no_values() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let dht = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let detailed = dht.get_mutable_detailed(signer.verifying_key().as_bytes(), None, None);
            let GetMutableDetailed { items, outcome } = detailed;

            let items = items.collect::<Vec<_>>().await;
            let outcome = outcome.recv().await;

            assert!(items.is_empty());
            assert!(outcome.queried > 0);
            assert_eq!(outcome.values, 0);
            assert!(outcome.no_values > 0);
            assert_eq!(
                outcome.valid_responses(),
                counted_valid_mutable_responses(&outcome)
            );
            assert_eq!(outcome.responded(), counted_mutable_responses(&outcome));
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn get_mutable_detailed_returns_values_outcome() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let a = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();
            let b = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let item = MutableItem::new(signer.clone(), b"Hello World!", 1000, None);
            a.put_mutable(item.clone(), None).await.unwrap();

            let detailed = b.get_mutable_detailed(signer.verifying_key().as_bytes(), None, None);
            let GetMutableDetailed { items, outcome } = detailed;

            let items = items.collect::<Vec<_>>().await;
            let outcome = outcome.recv().await;

            assert!(items.iter().any(|response| response == &item));
            assert!(outcome.queried > 0);
            assert_eq!(outcome.values, items.len() as u32);
            assert!(outcome.values > 0);
            assert_eq!(
                outcome.valid_responses(),
                counted_valid_mutable_responses(&outcome)
            );
            assert_eq!(outcome.responded(), counted_mutable_responses(&outcome));
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn get_mutable_detailed_no_more_recent_value() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let a = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();
            let b = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let seq = 1000;
            let item = MutableItem::new(signer.clone(), b"Hello World!", seq, None);
            a.put_mutable(item, None).await.unwrap();

            let detailed =
                b.get_mutable_detailed(signer.verifying_key().as_bytes(), None, Some(seq));
            let GetMutableDetailed { items, outcome } = detailed;

            let items = items.collect::<Vec<_>>().await;
            let outcome = outcome.recv().await;

            assert!(items.is_empty());
            assert!(outcome.queried > 0);
            assert_eq!(outcome.values, 0);
            assert!(outcome.no_more_recent > 0);
            assert_eq!(
                outcome.valid_responses(),
                counted_valid_mutable_responses(&outcome)
            );
            assert_eq!(outcome.responded(), counted_mutable_responses(&outcome));
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn put_get_mutable_no_more_recent_value() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let a = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();
            let b = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let seq = 1000;
            let value = b"Hello World!";

            let item = MutableItem::new(signer.clone(), value, seq, None);

            a.put_mutable(item.clone(), None).await.unwrap();

            let response = b
                .get_mutable(signer.verifying_key().as_bytes(), None, Some(seq))
                .next()
                .await;

            assert!(&response.is_none());
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn get_mutable_most_recent_prefers_highest_seq() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let dht = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let newer = MutableItem::new(signer.clone(), b"newer", 1001, None);
            dht.put_mutable(newer.clone(), None).await.unwrap();

            let older = MutableItem::new(signer, b"older", 1000, None);
            let (sender, _) = flume::bounded::<Result<PutOutcome, PutError>>(1);
            let request =
                PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(older, None));
            dht.0
                 .0
                .send(ActorMessage::Put(request, sender, None))
                .unwrap();

            let most_recent = dht
                .get_mutable_most_recent(newer.key(), None)
                .await
                .expect("No mutable values");

            assert_eq!(most_recent.seq(), newer.seq());
            assert_eq!(most_recent.value(), newer.value());
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn repeated_put_query() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let a = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let first = a.put_immutable(&[1, 2, 3]).await;
            let second = a.put_immutable(&[1, 2, 3]).await;

            assert_eq!(first.unwrap(), second.unwrap());
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn concurrent_get_mutable() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let a = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();
            let b = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let seq = 1000;
            let value = b"Hello World!";

            let item = MutableItem::new(signer.clone(), value, seq, None);

            a.put_mutable(item.clone(), None).await.unwrap();

            let _response_first = b
                .get_mutable(signer.verifying_key().as_bytes(), None, None)
                .next()
                .await
                .expect("No mutable values");

            let response_second = b
                .get_mutable(signer.verifying_key().as_bytes(), None, None)
                .next()
                .await
                .expect("No mutable values");

            assert_eq!(&response_second, &item);
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn concurrent_put_mutable_same() {
        let testnet = Testnet::builder(10).build().unwrap();

        let dht = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap()
            .as_async();

        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let seq = 1000;
        let value = b"Hello World!";

        let item = MutableItem::new(signer.clone(), value, seq, None);

        let mut handles = vec![];

        for _ in 0..2 {
            let dht = dht.clone();
            let item = item.clone();

            let handle = std::thread::spawn(move || {
                futures::executor::block_on(async { dht.put_mutable(item, None).await.unwrap() });
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn concurrent_put_mutable_different() {
        let testnet = Testnet::builder(10).build().unwrap();

        let dht = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap()
            .as_async();

        let mut handles = vec![];

        for i in 0..2 {
            let dht = dht.clone();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);

            let seq = 1000;

            let mut value = b"Hello World!".to_vec();
            value.push(i);

            let item = MutableItem::new(signer.clone(), &value, seq, None);

            let handle = std::thread::spawn(move || {
                futures::executor::block_on(async {
                    let result = dht.put_mutable(item, None).await;
                    if i == 0 {
                        assert!(result.is_ok())
                    } else {
                        assert!(matches!(
                            result,
                            Err(PutMutableError::Concurrency(ConcurrencyError::ConflictRisk))
                        ))
                    }
                })
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn concurrent_put_mutable_different_with_cas() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let dht = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);
            let value = b"Hello World!".to_vec();

            // First
            {
                let item = MutableItem::new(signer.clone(), &value, 1000, None);

                let (sender, _) = flume::bounded::<Result<PutOutcome, PutError>>(1);
                let request =
                    PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(item, None));
                dht.0
                     .0
                    .send(ActorMessage::Put(request, sender, None))
                    .unwrap();
            }

            std::thread::sleep(Duration::from_millis(100));

            // Second
            {
                let item = MutableItem::new(signer, &value, 1001, None);

                dht.put_mutable(item, Some(1000)).await.unwrap();
            }
        }

        futures::executor::block_on(test());
    }

    #[test]
    fn conflict_301_cas() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();

            let dht = Dht::builder()
                .bootstrap(&testnet.bootstrap)
                .bind_address(Ipv4Addr::LOCALHOST)
                .build()
                .unwrap()
                .as_async();

            let signer = SigningKey::from_bytes(&[
                56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
                228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
            ]);
            let value = b"Hello World!".to_vec();

            dht.put_mutable(MutableItem::new(signer.clone(), &value, 1001, None), None)
                .await
                .unwrap();

            assert!(matches!(
                dht.put_mutable(MutableItem::new(signer, &value, 1002, None), Some(1000))
                    .await,
                Err(PutMutableError::Concurrency(ConcurrencyError::CasFailed))
            ));
        }

        futures::executor::block_on(test());
    }
}
