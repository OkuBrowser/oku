use std::collections::BTreeMap;
use std::net::SocketAddrV4;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct InflightRequest {
    pub to: SocketAddrV4,
    pub sent_at: Instant,
}

impl InflightRequest {
    pub fn does_match(&self, socket: &SocketAddrV4) -> bool {
        if self.to.port() != socket.port() {
            return false;
        }

        if self.to.ip().is_unspecified() {
            return true;
        }

        self.to.ip() == socket.ip()
    }
}

#[derive(Debug)]
pub struct InflightRequests {
    // BTreeMap provides O(log n) lookup, insertion, and deletion keyed by transaction_id.
    requests: BTreeMap<u32, InflightRequest>,
    timeout: Duration,
}

impl InflightRequests {
    pub fn new(timeout: Duration) -> Self {
        Self {
            requests: BTreeMap::new(),
            timeout,
        }
    }

    /// Add a new inflight request O(log n)
    pub fn add(&mut self, transaction_id: u32, to: SocketAddrV4) {
        self.requests.insert(
            transaction_id,
            InflightRequest {
                to,
                sent_at: Instant::now(),
            },
        );
    }

    /// Check if a transaction_id is still inflight and not expired O(log n)
    pub fn contains(&self, transaction_id: u32) -> bool {
        if let Some(request) = self.requests.get(&transaction_id) {
            return request.sent_at.elapsed() < self.timeout;
        }
        false
    }

    /// Remove inflight request by transaction_id if it exists and matches the address
    /// O(log n)
    pub fn remove(&mut self, transaction_id: u32, from: &SocketAddrV4) -> Option<InflightRequest> {
        let request = self.requests.get(&transaction_id)?;

        // Drop immediately if expired; avoid accepting late responses
        if request.sent_at.elapsed() >= self.timeout {
            self.requests.remove(&transaction_id);
            return None;
        }

        if !request.does_match(from) {
            return None;
        }

        self.requests.remove(&transaction_id)
    }

    /// Cleanup expired requests based on timeout
    /// O(n) scans all requests to remove expired ones
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        let cutoff = now - self.timeout;

        // Remove expired requests in a single pass using retain
        self.requests.retain(|_, request| request.sent_at > cutoff);
    }
}
