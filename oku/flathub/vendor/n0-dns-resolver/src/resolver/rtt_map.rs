//! Smoothed round-trip time (RTT) tracking, per nameserver.
//!
//! Each nameserver carries two estimates, because the two readers want opposite
//! treatments of a lookup's own overhead. Ordering asks what a server costs us,
//! so it measures the whole attempt: one whose UDP is black-holed pays
//! `TCP_JOIN_DELAY` every lookup and should rank below one that answers a
//! datagram in 40ms. Pacing asks how long a datagram takes to come back, so it
//! measures one exchange. Folding our own intervals into that estimate would let
//! it set its own input.

use std::sync::Mutex;

use n0_future::time::{Duration, Instant};

/// Weight of each new round-trip sample in the running average, from 0 to 1.
///
/// Every measurement updates the estimate as a blend of its previous value and
/// the new sample: `alpha * sample + (1 - alpha) * previous`. A larger weight
/// follows recent latency more closely; a smaller one smooths over more of the
/// past.
const SRTT_ALPHA: f64 = 0.3;

/// Neutral smoothed-RTT baseline in microseconds for a never-probed nameserver.
///
/// This is also the value every estimate decays back toward. Measured-fast
/// servers sit below the baseline and are preferred; failed servers sit above
/// it and are demoted. Decaying toward the baseline rather than toward zero
/// keeps a measured-fast server ahead of an idle or recovering one.
const SRTT_BASELINE_MICROS: f64 = 50_000.0;

/// Penalty in microseconds added to a nameserver's smoothed RTT on a failure.
///
/// Large enough to demote the server below the currently-healthy ones.
const SRTT_FAILURE_PENALTY_MICROS: f64 = 150_000.0;

/// Upper bound on a nameserver's smoothed RTT, in microseconds.
const SRTT_MAX_MICROS: f64 = 5_000_000.0;

/// How slowly the smoothed RTT decays back toward the baseline, in seconds.
///
/// Larger values decay more slowly. The decay is applied when the estimate is
/// read, so a demoted server gradually recovers and an idle estimate lapses
/// back to neutral. After this many seconds the gap between an estimate and the
/// baseline has shrunk to about a third of its original size.
const SRTT_DECAY_SECS: f64 = 180.0;

/// Smoothed round-trip time estimates for one nameserver.
#[derive(Debug)]
struct Srtt {
    /// What a whole attempt costs, in microseconds, as of `updated`.
    ///
    /// Orders nameservers fastest-first and demotes ones that fail. Decays back
    /// toward the baseline as it ages, so a demoted server gets re-probed and a
    /// once-fast one that has gone away does not stay preferred.
    attempt_micros: f64,
    /// When `attempt_micros` was last written.
    updated: Instant,
    /// Round trip of one datagram exchange, in microseconds.
    ///
    /// Paces retransmits. Written only when a datagram carried the answer, so
    /// neither our own intervals nor a rescuing TCP query reach it. Does not
    /// decay: its only reader clamps it to a few hundred milliseconds either
    /// way, and the next datagram corrects it.
    datagram_micros: f64,
}

impl Srtt {
    /// Creates an entry at the neutral baseline, as for an untried server.
    fn new() -> Self {
        Self {
            attempt_micros: SRTT_BASELINE_MICROS,
            updated: Instant::now(),
            datagram_micros: SRTT_BASELINE_MICROS,
        }
    }

    /// Returns the decayed estimate at `now`, used for ordering.
    ///
    /// Relaxes toward [`SRTT_BASELINE_MICROS`] as the estimate ages.
    fn decayed(&self, now: Instant) -> f64 {
        let dt = now.saturating_duration_since(self.updated).as_secs_f64();
        SRTT_BASELINE_MICROS
            + (self.attempt_micros - SRTT_BASELINE_MICROS) * (-dt / SRTT_DECAY_SECS).exp()
    }

    /// Folds a successful attempt into the estimates.
    ///
    /// `attempt` is how long the whole attempt took, `datagram` the round trip
    /// of the exchange that answered when a datagram did. They differ by the
    /// intervals the attempt waited out and by any TCP query that joined the
    /// datagrams, which is what the pacing estimate must not see.
    fn record_success(&mut self, attempt: Duration, datagram: Option<Duration>, now: Instant) {
        let base = self.decayed(now);
        let sample = attempt.as_micros() as f64;
        self.attempt_micros =
            (SRTT_ALPHA * sample + (1.0 - SRTT_ALPHA) * base).min(SRTT_MAX_MICROS);
        self.updated = now;
        if let Some(datagram) = datagram {
            let sample = datagram.as_micros() as f64;
            self.datagram_micros = (SRTT_ALPHA * sample
                + (1.0 - SRTT_ALPHA) * self.datagram_micros)
                .min(SRTT_MAX_MICROS);
        }
    }

    /// Penalizes the attempt estimate after a failed attempt.
    ///
    /// Leaves `datagram_micros` alone: a failure says the server did not answer,
    /// not that its datagrams got slower.
    fn record_failure(&mut self, now: Instant) {
        let base = self.decayed(now);
        self.attempt_micros = (base + SRTT_FAILURE_PENALTY_MICROS).min(SRTT_MAX_MICROS);
        self.updated = now;
    }
}

/// Smoothed-RTT estimates for a fixed set of nameservers.
///
/// Indexed in parallel to the resolver's nameserver list, behind a single mutex
/// so the resolver can read and update health from concurrent queries without
/// threading a lock through the call sites.
#[derive(Debug)]
pub(super) struct RttMap {
    /// One entry per nameserver, indexed as the resolver's nameserver list is.
    entries: Mutex<Vec<Srtt>>,
}

impl RttMap {
    /// Creates a map with `len` nameservers, each at the neutral baseline.
    pub(super) fn new(len: usize) -> Self {
        Self {
            entries: Mutex::new((0..len).map(|_| Srtt::new()).collect()),
        }
    }

    /// Returns the decayed attempt cost for nameserver `idx`, used for ordering.
    pub(super) fn get_decayed(&self, idx: usize) -> f64 {
        self.entries.lock().expect("poisoned")[idx].decayed(Instant::now())
    }

    /// Returns the datagram round trip for nameserver `idx`, used for pacing.
    pub(super) fn get_datagram(&self, idx: usize) -> Duration {
        Duration::from_micros(self.entries.lock().expect("poisoned")[idx].datagram_micros as u64)
    }

    /// Folds a successful attempt for nameserver `idx` into its estimates.
    ///
    /// See [`Srtt::record_success`] for the two samples.
    pub(super) fn record_success(&self, idx: usize, attempt: Duration, datagram: Option<Duration>) {
        self.entries.lock().expect("poisoned")[idx].record_success(
            attempt,
            datagram,
            Instant::now(),
        );
    }

    /// Penalizes nameserver `idx` after a failed attempt.
    pub(super) fn record_failure(&self, idx: usize) {
        self.entries.lock().expect("poisoned")[idx].record_failure(Instant::now());
    }
}
