use tracing::{debug, trace};

use crate::{
    common::{
        ErrorSpecific, Id, PutRequest, PutRequestSpecific, RequestSpecific, RequestTypeSpecific,
    },
    Node,
};

use super::socket::KrpcSocket;

/// Stores data at the closest nodes after an [super::IterativeQuery] is done,
/// or when a previous cached query is available.
///
/// Tracks successful acknowledgements and errors for the PUT query.
#[derive(Debug)]
pub struct PutQuery {
    pub target: Id,
    /// Nodes that confirmed success
    stored_at: u32,
    inflight_requests: Vec<u32>,
    pub request: PutRequestSpecific,
    errors: Vec<(u8, ErrorSpecific)>,
    extra_nodes: Box<[Node]>,
}

impl PutQuery {
    pub fn new(target: Id, request: PutRequestSpecific, extra_nodes: Option<Box<[Node]>>) -> Self {
        Self {
            target,
            stored_at: 0,
            inflight_requests: Vec::new(),
            request,
            errors: Vec::new(),
            extra_nodes: extra_nodes.unwrap_or_default(),
        }
    }

    pub fn start(
        &mut self,
        socket: &mut KrpcSocket,
        closest_nodes: &[Node],
    ) -> Result<(), PutError> {
        assert!(!self.started(), "should not call PutQuery::start() twice");

        let target = self.target;
        trace!(?target, "PutQuery start");

        if closest_nodes.is_empty() {
            Err(PutQueryError::NoClosestNodes)?;
        }

        assert!(
            closest_nodes.len() <= u8::MAX as usize,
            "should not send PUT query to more than 256 nodes"
        );

        for node in closest_nodes.iter().chain(self.extra_nodes.iter()) {
            // Set correct values to the request placeholders
            if let Some(token) = node.token() {
                let tid = socket.request(
                    node.address(),
                    RequestSpecific {
                        requester_id: Id::random(),
                        request_type: RequestTypeSpecific::Put(PutRequest {
                            token,
                            put_request_type: self.request.clone(),
                        }),
                    },
                );

                self.inflight_requests.push(tid);
            }
        }

        Ok(())
    }

    pub fn started(&self) -> bool {
        !self.inflight_requests.is_empty()
    }

    pub fn inflight(&self, tid: u32) -> bool {
        self.inflight_requests.contains(&tid)
    }

    pub fn success(&mut self) {
        debug!(target = ?self.target, "PutQuery got success response");
        self.stored_at += 1
    }

    pub fn error(&mut self, error: ErrorSpecific) {
        debug!(target = ?self.target, ?error, "PutQuery got error");

        if let Some(pos) = self
            .errors
            .iter()
            .position(|(_, err)| error.code == err.code)
        {
            // Increment the count of the existing error
            self.errors[pos].0 += 1;

            // Move the updated element to maintain the order (highest count first)
            let mut i = pos;
            while i > 0 && self.errors[i].0 > self.errors[i - 1].0 {
                self.errors.swap(i, i - 1);
                i -= 1;
            }
        } else {
            // Add the new error with a count of 1
            self.errors.push((1, error));
        }
    }

    /// Check if the query has completed, returning the PUT outcome when complete.
    pub fn poll_completion(&self, socket: &KrpcSocket) -> Result<Option<PutOutcome>, PutError> {
        if !self.started() {
            return Ok(None);
        }

        if let Some(most_common_error) = self.majority_nodes_rejected_put_mutable() {
            debug!(
                target = ?self.target,
                ?most_common_error,
                nodes_count = self.inflight_requests.len(),
                "PutQuery for MutableItem was rejected by most nodes with 3xx code."
            );

            return Err(PutError::from(most_common_error));
        }

        // And all queries got responses or timed out.
        if self.is_done(socket) {
            let target = self.target;

            if self.stored_at == 0 {
                let most_common_error = self.most_common_error();

                debug!(
                    ?target,
                    ?most_common_error,
                    nodes_count = self.inflight_requests.len(),
                    "Put Query: failed"
                );

                return Err(most_common_error
                    .map(|(_, error)| error)
                    .unwrap_or(PutQueryError::Timeout.into()));
            }

            debug!(?target, stored_at = ?self.stored_at, "PutQuery Done successfully");

            return Ok(Some(PutOutcome {
                target: self.target,
                stored_at: self.stored_at,
            }));
        }

        Ok(None)
    }

    fn is_done(&self, socket: &KrpcSocket) -> bool {
        self.inflight_requests
            .iter()
            .copied()
            .all(|transaction_id| !socket.inflight(transaction_id))
    }

    fn majority_nodes_rejected_put_mutable(&self) -> Option<ConcurrencyError> {
        if !matches!(self.request, PutRequestSpecific::PutMutable(_)) {
            return None;
        }

        let (count, error) = self.most_common_error()?;
        let half = ((self.inflight_requests.len() / 2) + 1) as u8;
        if count < half {
            return None;
        }

        match error {
            PutError::Concurrency(error) => Some(error),
            PutError::Query(_) => None,
        }
    }

    fn most_common_error(&self) -> Option<(u8, PutError)> {
        self.errors
            .first()
            .and_then(|(count, error)| match error.code {
                301 => Some((*count, PutError::from(ConcurrencyError::CasFailed))),
                302 => Some((*count, PutError::from(ConcurrencyError::NotMostRecent))),
                _ => None,
            })
    }
}

/// Result details for a successful PUT query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PutOutcome {
    /// DHT target the request was published under.
    pub target: Id,

    /// Number of DHT nodes that acknowledged storing the item.
    pub stored_at: u32,
}

/// PutQuery errors
#[derive(thiserror::Error, Debug, Clone)]
pub enum PutError {
    /// Common PutQuery errors
    #[error(transparent)]
    Query(#[from] PutQueryError),

    #[error(transparent)]
    /// PutQuery for [crate::MutableItem] errors
    Concurrency(#[from] ConcurrencyError),
}

/// Common PutQuery errors
#[derive(thiserror::Error, Debug, Clone)]
pub enum PutQueryError {
    /// Failed to find any nodes close, usually means dht node failed to bootstrap,
    /// so the routing table is empty. Check the machine's access to UDP socket,
    /// or find better bootstrapping nodes.
    #[error("Failed to find any nodes close to store value at")]
    NoClosestNodes,

    /// Either Put Query failed to store at any nodes, and most nodes responded
    /// with a non `301` nor `302` errors.
    ///
    /// Either way; contains the most common error response.
    #[error("Query Error Response")]
    ErrorResponse(ErrorSpecific),

    /// PutQuery timed out with no responses neither success or errors
    #[error("PutQuery timed out with no responses neither success or errors")]
    Timeout,
}

/// PutQuery for [crate::MutableItem] errors
#[derive(thiserror::Error, Debug, Clone)]
pub enum ConcurrencyError {
    /// Trying to PUT mutable items with the same `key`, and `salt` but different `seq`.
    ///
    /// Moreover, the more recent item does _NOT_ mention the the earlier
    /// item's `seq` in its `cas` field.
    ///
    /// This risks a [Lost Update Problem](https://en.wikipedia.org/wiki/Write-write_conflict).
    ///
    /// Try reading most recent mutable item before writing again,
    /// and make sure to set the `cas` field.
    #[error("Conflict risk, try reading most recent item before writing again.")]
    ConflictRisk,

    /// The [crate::MutableItem::seq] is less than or equal the sequence from another signed item.
    ///
    /// Try reading most recent mutable item before writing again.
    #[error("MutableItem::seq is not the most recent, try reading most recent item before writing again.")]
    NotMostRecent,

    /// The `CAS` condition does not match the `seq` of the most recent known signed item.
    #[error("CAS check failed, try reading most recent item before writing again.")]
    CasFailed,
}

#[cfg(test)]
mod tests {
    use crate::{
        common::{PutMutableRequestArguments, PutRequestSpecific},
        MutableItem, SigningKey,
    };

    use super::{ConcurrencyError, PutError, PutQuery};
    use crate::common::ErrorSpecific;
    use crate::rpc::socket::KrpcSocket;

    #[test]
    fn mutable_majority_cas_failure_wins_over_completed_success() {
        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);
        let item = MutableItem::new(signer, b"value", 1002, None);
        let request = PutRequestSpecific::PutMutable(PutMutableRequestArguments::from(
            item.clone(),
            Some(1000),
        ));
        let mut query = PutQuery::new(*item.target(), request, None);

        query.inflight_requests = vec![1, 2, 3];
        query.success();
        query.error(cas_failed());
        query.error(cas_failed());

        let socket = KrpcSocket::client().unwrap();

        assert!(matches!(
            query.poll_completion(&socket),
            Err(PutError::Concurrency(ConcurrencyError::CasFailed))
        ));
    }

    fn cas_failed() -> ErrorSpecific {
        ErrorSpecific {
            code: 301,
            description: "cas failed".to_string(),
        }
    }
}
