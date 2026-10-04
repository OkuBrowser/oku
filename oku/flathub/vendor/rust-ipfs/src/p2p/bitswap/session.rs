//! Per-peer bitswap session for the peer-keyed rewrite.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use bytes::Bytes;
use connexa::error::Error;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, Stream, StreamExt};
use ipld_core::cid::Cid;
use parking_lot::Mutex;

use super::message::{BitswapMessage, BitswapRequest, BitswapResponse, RequestType};
use super::wantlist::Wantlist;
use crate::Block;
use crate::repo::{DefaultStorage, Repo};

const MAX_INFLIGHT_SERVES: usize = 32;
const SERVE_BATCH_LIMIT: usize = 1 << 20;

#[derive(Clone)]
pub struct ServeBudget {
    inner: Arc<Mutex<BudgetInner>>,
}

struct BudgetInner {
    in_flight: usize,
    limit: usize,
    waiters: Vec<Waker>,
}

impl ServeBudget {
    pub fn new(limit: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BudgetInner {
                in_flight: 0,
                limit: limit.max(1),
                waiters: Vec::new(),
            })),
        }
    }

    fn try_acquire(&self) -> bool {
        let mut inner = self.inner.lock();
        if inner.in_flight < inner.limit {
            inner.in_flight += 1;
            true
        } else {
            false
        }
    }

    fn release(&self, n: usize) {
        let mut inner = self.inner.lock();
        inner.in_flight = inner.in_flight.saturating_sub(n);
    }

    fn wake_waiters(&self) {
        let waiters = std::mem::take(&mut self.inner.lock().waiters);
        for waker in waiters {
            waker.wake();
        }
    }

    fn register_waiter(&self, waker: &Waker) {
        let mut inner = self.inner.lock();
        if !inner.waiters.iter().any(|w| w.will_wake(waker)) {
            inner.waiters.push(waker.clone());
        }
    }
}

struct QueuedServe {
    seq: u64,
    request: BitswapRequest,
}

impl PartialEq for QueuedServe {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}

impl Eq for QueuedServe {}

impl PartialOrd for QueuedServe {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedServe {
    fn cmp(&self, other: &Self) -> Ordering {
        self.request
            .priority
            .cmp(&other.request.priority)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

#[derive(Debug)]
pub enum PeerSessionEvent {
    /// Deliver this message to the peer through its handler.
    Send(BitswapMessage),
    Have(Cid),
    /// A wanted block arrived and is stored; the behaviour cancels the want.
    Stored(Cid),
    /// A wanted block arrived but could not be persisted locally.
    StoreFailed {
        cid: Cid,
        error: Error,
    },
    /// The peer does not have this cid.
    DontHave(Cid),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Ledger {
    pub blocks_sent: u64,
    pub bytes_sent: u64,
    pub blocks_recv: u64,
    pub bytes_recv: u64,
}

pub struct PeerSession {
    wantlist: Wantlist,
    repo: Repo<DefaultStorage>,
    /// wants already sent to this peer, with the request type last sent.
    sent: HashMap<Cid, RequestType>,
    /// cids this peer has requested from us, keyed to their latest request (type, priority, flags).
    peer_wants: HashMap<Cid, BitswapRequest>,
    outbound: VecDeque<PeerSessionEvent>,
    serve_backlog: BinaryHeap<QueuedServe>,
    backlog_set: HashSet<Cid>,
    serve_seq: u64,
    serving: FuturesUnordered<BoxFuture<'static, (BitswapRequest, Option<Block>)>>,
    storing: FuturesUnordered<BoxFuture<'static, (Cid, Result<(), Error>)>>,
    budget: ServeBudget,
    budget_blocked: bool,
    ledger: Ledger,
    waker: Option<Waker>,
}

impl PeerSession {
    pub fn new(wantlist: Wantlist, repo: Repo<DefaultStorage>, budget: ServeBudget) -> Self {
        let mut session = Self {
            wantlist,
            repo,
            sent: HashMap::new(),
            peer_wants: HashMap::new(),
            outbound: VecDeque::new(),
            serve_backlog: BinaryHeap::new(),
            backlog_set: HashSet::new(),
            serve_seq: 0,
            serving: FuturesUnordered::new(),
            storing: FuturesUnordered::new(),
            budget,
            budget_blocked: false,
            ledger: Ledger::default(),
            waker: None,
        };
        session.sync();
        session
    }

    /// cids this peer is currently asking us for.
    pub fn peer_wantlist(&self) -> Vec<Cid> {
        self.peer_wants.keys().copied().collect()
    }

    pub fn ledger(&self) -> Ledger {
        self.ledger
    }

    pub fn request_block(&mut self, cid: Cid) -> Option<BitswapMessage> {
        if self.sent.get(&cid) == Some(&RequestType::Block) {
            return None;
        }
        self.sent.insert(cid, RequestType::Block);
        Some(
            BitswapMessage::new(false).add_request(BitswapRequest::block(cid).send_dont_have(true)),
        )
    }

    pub fn reset_block(&mut self, cid: Cid) {
        self.sent.remove(&cid);
    }

    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    fn queue(&mut self, event: PeerSessionEvent) {
        self.outbound.push_back(event);
        self.wake();
    }

    /// Reconcile what we have sent this peer against the shared wantlist, queueing wants and cancels.
    pub fn sync(&mut self) {
        let entries = self.wantlist.entries();
        let wanted: HashSet<_> = entries.iter().map(|(cid, _)| *cid).collect();
        let mut requests = Vec::new();

        for (cid, entry) in &entries {
            if self.sent.get(cid) == Some(&RequestType::Block) {
                continue;
            }
            let desired = entry.want_type;
            if self.sent.get(cid) == Some(&desired) {
                continue;
            }
            if desired == RequestType::Have && entry.has_provider {
                continue;
            }
            let request = match desired {
                RequestType::Have => BitswapRequest::have(*cid),
                RequestType::Block => BitswapRequest::block(*cid),
            }
            .send_dont_have(true)
            .set_priority(entry.priority);
            requests.push(request);
            self.sent.insert(*cid, desired);
        }

        let canceled: Vec<Cid> = self
            .sent
            .keys()
            .filter(|cid| !wanted.contains(cid))
            .copied()
            .collect();
        for cid in canceled {
            requests.push(BitswapRequest::cancel(cid));
            self.sent.remove(&cid);
        }

        if !requests.is_empty() {
            self.queue(PeerSessionEvent::Send(
                BitswapMessage::new(false).set_requests(requests),
            ));
        }
    }

    pub fn on_message(&mut self, message: BitswapMessage) {
        if message.full {
            let keep: HashSet<_> = message
                .requests
                .iter()
                .filter(|request| !request.cancel)
                .map(|request| request.cid)
                .collect();
            self.peer_wants.retain(|cid, _| keep.contains(cid));
            self.backlog_set.retain(|cid| keep.contains(cid));
            self.serve_backlog = std::mem::take(&mut self.serve_backlog)
                .into_iter()
                .filter(|queued| keep.contains(&queued.request.cid))
                .collect();
        }

        for request in message.requests {
            let cid = request.cid;
            if request.cancel {
                self.peer_wants.remove(&cid);
                continue;
            }
            self.peer_wants.insert(cid, request);
            self.enqueue_serve(request);
        }

        for (cid, response) in message.responses {
            match response {
                BitswapResponse::Have(true) => {
                    self.wantlist.note_have(&cid);
                    if self.wantlist.contains(&cid) {
                        self.queue(PeerSessionEvent::Have(cid));
                    }
                }
                BitswapResponse::Have(false) => {
                    self.sent.remove(&cid);
                    self.queue(PeerSessionEvent::DontHave(cid));
                }
                BitswapResponse::Block(data) => {
                    self.sent.remove(&cid);
                    if !self.wantlist.contains(&cid) {
                        continue;
                    }
                    self.ledger.blocks_recv += 1;
                    self.ledger.bytes_recv += data.len() as u64;
                    match Block::new(cid, data) {
                        Ok(block) => {
                            let repo = self.repo.clone();
                            self.storing.push(
                                async move {
                                    let result = repo
                                        .put_block(&block)
                                        .await
                                        .map(|_| ())
                                        .map_err(std::io::Error::other)
                                        .map_err(Error::from);
                                    (cid, result)
                                }
                                .boxed(),
                            );
                        }
                        Err(_) => self.queue(PeerSessionEvent::DontHave(cid)),
                    }
                }
            }
        }

        self.drain_serves();
        self.wake();
    }

    fn enqueue_serve(&mut self, request: BitswapRequest) {
        if self.backlog_set.insert(request.cid) {
            let seq = self.serve_seq;
            self.serve_seq += 1;
            self.serve_backlog.push(QueuedServe { seq, request });
        }
    }

    fn drain_serves(&mut self) {
        self.budget_blocked = false;
        while self.serving.len() < MAX_INFLIGHT_SERVES {
            let Some(top) = self.serve_backlog.peek() else {
                break;
            };
            let cid = top.request.cid;
            let Some(&request) = self.peer_wants.get(&cid) else {
                self.serve_backlog.pop();
                self.backlog_set.remove(&cid);
                continue;
            };
            if !self.budget.try_acquire() {
                self.budget_blocked = true;
                break;
            }
            self.serve_backlog.pop();
            self.backlog_set.remove(&cid);
            let repo = self.repo.clone();
            self.serving.push(
                async move {
                    let block = repo.get_block_now(cid).await.ok().flatten();
                    (request, block)
                }
                .boxed(),
            );
        }
    }

    /// Re-serve cids this peer previously asked for, now that we may hold them.
    pub fn serve_wanted(&mut self, cids: &[Cid]) {
        for cid in cids {
            let Some(&request) = self.peer_wants.get(cid) else {
                continue;
            };
            self.enqueue_serve(request);
        }
        self.drain_serves();
        self.wake();
    }
}

fn serve_response(
    request: &BitswapRequest,
    block: Option<Block>,
) -> Option<(Cid, BitswapResponse)> {
    let response = match (request.ty, block) {
        (RequestType::Have, Some(_)) => BitswapResponse::Have(true),
        (RequestType::Block, Some(block)) => {
            BitswapResponse::Block(Bytes::copy_from_slice(block.data()))
        }
        (_, None) if request.send_dont_have => BitswapResponse::Have(false),
        (_, None) => return None,
    };
    Some((request.cid, response))
}

fn response_size(response: &BitswapResponse) -> usize {
    match response {
        BitswapResponse::Block(data) => data.len() + 64,
        BitswapResponse::Have(_) => 64,
    }
}

impl Stream for PeerSession {
    type Item = PeerSessionEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        this.drain_serves();
        let mut batch: Vec<(Cid, BitswapResponse)> = Vec::new();
        let mut batch_size = 0;
        let mut released = false;
        while let Poll::Ready(Some((request, block))) = this.serving.poll_next_unpin(cx) {
            this.budget.release(1);
            released = true;
            if request.ty == RequestType::Block
                && let Some(block) = &block
            {
                this.ledger.blocks_sent += 1;
                this.ledger.bytes_sent += block.data().len() as u64;
            }
            if let Some((cid, response)) = serve_response(&request, block) {
                let size = response_size(&response);
                if !batch.is_empty() && batch_size + size > SERVE_BATCH_LIMIT {
                    this.outbound.push_back(PeerSessionEvent::Send(
                        BitswapMessage::new(false).set_responses(std::mem::take(&mut batch)),
                    ));
                    batch_size = 0;
                }
                batch.push((cid, response));
                batch_size += size;
            }
            this.drain_serves();
        }
        if !batch.is_empty() {
            this.outbound.push_back(PeerSessionEvent::Send(
                BitswapMessage::new(false).set_responses(batch),
            ));
        }

        if released {
            this.budget.wake_waiters();
        }

        while let Poll::Ready(Some((cid, result))) = this.storing.poll_next_unpin(cx) {
            match result {
                Ok(()) => this.outbound.push_back(PeerSessionEvent::Stored(cid)),
                Err(error) => this
                    .outbound
                    .push_back(PeerSessionEvent::StoreFailed { cid, error }),
            }
        }

        if let Some(event) = this.outbound.pop_front() {
            return Poll::Ready(Some(event));
        }

        if this.budget_blocked && !this.serve_backlog.is_empty() {
            this.budget.register_waiter(cx.waker());
        }

        this.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for PeerSession {
    fn drop(&mut self) {
        let inflight = self.serving.len();
        if inflight > 0 {
            self.budget.release(inflight);
            self.budget.wake_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use multihash_codetable::{Code, MultihashDigest};

    fn test_cid() -> Cid {
        Cid::new_v1(0x55, Code::Sha2_256.digest(b"sync gate"))
    }

    #[tokio::test]
    async fn sync_gates_want_have_when_provider_known() {
        let repo = Repo::new_memory();
        let cid = test_cid();

        let wantlist = Wantlist::default();
        wantlist.want(cid, RequestType::Have, 1, None);
        let mut session = PeerSession::new(wantlist, repo.clone(), ServeBudget::new(1024));
        assert!(
            matches!(session.next().await, Some(PeerSessionEvent::Send(_))),
            "an unprovided want should be solicited"
        );

        let wantlist = Wantlist::default();
        wantlist.want(cid, RequestType::Have, 1, None);
        wantlist.note_have(&cid);
        let mut session = PeerSession::new(wantlist, repo, ServeBudget::new(1024));
        let polled = futures::poll!(std::pin::pin!(session.next()));
        assert!(
            polled.is_pending(),
            "a want with a known provider must not be re-solicited"
        );
    }

    #[tokio::test]
    async fn serves_coalesce_into_fewer_messages() {
        let repo = Repo::new_memory();
        let mut cids = Vec::new();
        for i in 0..8u32 {
            let data = format!("batch block {i}").into_bytes();
            let cid = Cid::new_v1(0x55, Code::Sha2_256.digest(&data));
            repo.put_block(&Block::new_unchecked(cid, data))
                .await
                .unwrap();
            cids.push(cid);
        }

        let mut session = PeerSession::new(Wantlist::default(), repo, ServeBudget::new(1024));
        let requests = cids.iter().map(|cid| BitswapRequest::block(*cid)).collect();
        session.on_message(BitswapMessage::new(false).set_requests(requests));

        let mut messages = 0;
        let mut served = 0;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(500);
        while let Ok(Some(event)) = tokio::time::timeout_at(deadline, session.next()).await {
            if let PeerSessionEvent::Send(message) = event {
                messages += 1;
                served += message.responses.len();
            }
        }
        assert_eq!(served, 8, "all requested blocks served");
        assert!(
            messages < 8,
            "serves should coalesce, got {messages} messages"
        );
    }

    #[test]
    fn serve_budget_acquire_release_and_wake() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
        use std::task::{Wake, Waker};

        struct FlagWaker(AtomicBool);
        impl Wake for FlagWaker {
            fn wake(self: Arc<Self>) {
                self.0.store(true, AtomicOrdering::SeqCst);
            }
            fn wake_by_ref(self: &Arc<Self>) {
                self.0.store(true, AtomicOrdering::SeqCst);
            }
        }

        let budget = ServeBudget::new(2);
        assert!(budget.try_acquire());
        assert!(budget.try_acquire());
        assert!(!budget.try_acquire(), "budget exhausted at the limit");

        budget.release(1);
        assert!(budget.try_acquire(), "release frees a slot");
        assert!(!budget.try_acquire());

        let flag = Arc::new(FlagWaker(AtomicBool::new(false)));
        let waker = Waker::from(flag.clone());
        budget.register_waiter(&waker);
        budget.wake_waiters();
        assert!(
            flag.0.load(AtomicOrdering::SeqCst),
            "a registered waiter is woken when budget frees"
        );
    }

    #[tokio::test]
    async fn serve_budget_serves_all_under_tiny_cap() {
        let repo = Repo::new_memory();
        let mut cids = Vec::new();
        for i in 0..8u32 {
            let data = format!("cap block {i}").into_bytes();
            let cid = Cid::new_v1(0x55, Code::Sha2_256.digest(&data));
            repo.put_block(&Block::new_unchecked(cid, data))
                .await
                .unwrap();
            cids.push(cid);
        }

        let mut session = PeerSession::new(Wantlist::default(), repo, ServeBudget::new(1));
        let requests = cids.iter().map(|cid| BitswapRequest::block(*cid)).collect();
        session.on_message(BitswapMessage::new(false).set_requests(requests));

        let mut served = HashSet::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(500);
        while served.len() < 8 {
            match tokio::time::timeout_at(deadline, session.next()).await {
                Ok(Some(PeerSessionEvent::Send(message))) => {
                    for (cid, _) in message.responses {
                        served.insert(cid);
                    }
                }
                Ok(Some(_)) => {}
                _ => break,
            }
        }
        assert_eq!(
            served.len(),
            8,
            "all blocks served even under a serve budget of 1"
        );
    }

    #[test]
    fn serve_backlog_orders_by_priority_then_fifo() {
        let cid = test_cid();
        let queued = |seq: u64, priority: i32| QueuedServe {
            seq,
            request: BitswapRequest::block(cid).set_priority(priority),
        };

        let mut heap = BinaryHeap::new();
        heap.push(queued(0, 5));
        heap.push(queued(1, 1));
        heap.push(queued(2, 10));
        heap.push(queued(3, 5));

        let popped: Vec<(i32, u64)> =
            std::iter::from_fn(|| heap.pop().map(|q| (q.request.priority, q.seq))).collect();

        assert_eq!(
            popped,
            vec![(10, 2), (5, 0), (5, 3), (1, 1)],
            "highest priority admitted first, FIFO (lowest seq) among equal priority"
        );
    }

    #[test]
    fn full_wantlist_prunes_withdrawn_wants() {
        let repo = Repo::new_memory();
        let mut session = PeerSession::new(Wantlist::default(), repo, ServeBudget::new(1024));
        let x = Cid::new_v1(0x55, Code::Sha2_256.digest(b"full x"));
        let y = Cid::new_v1(0x55, Code::Sha2_256.digest(b"full y"));

        session.on_message(
            BitswapMessage::new(false)
                .set_requests(vec![BitswapRequest::block(x), BitswapRequest::block(y)]),
        );
        let mut before = session.peer_wantlist();
        before.sort();
        let mut expected = vec![x, y];
        expected.sort();
        assert_eq!(before, expected, "both wants recorded");

        session.on_message(BitswapMessage::new(true).set_requests(vec![BitswapRequest::block(x)]));
        assert_eq!(
            session.peer_wantlist(),
            vec![x],
            "a full wantlist prunes the withdrawn want"
        );
    }
}
