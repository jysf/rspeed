//! Metrics accumulator: shared, lock-protected counters and a watch-channel
//! snapshot stream. Decoupled from rendering per DEC-008 seam 1 — the
//! accumulator owns the `watch::Sender<Snapshot>` and does not know how many
//! subscribers exist or what they do with the data.
//!
//! The warm-up window from DEC-005 is enforced via the baseline-snapshot
//! pattern: on the first tick where `elapsed >= warmup`, the tick handler
//! captures `total_bytes` into `bytes_at_warmup_end`. `finish()` then derives
//! post-warm-up bytes as `total_bytes - bytes_at_warmup_end`. This eliminates
//! the race between `record_bytes` and the warm-up boundary tick — the
//! boundary is determined by elapsed time, not by `record_bytes` call order.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{Instant, MissedTickBehavior};

use crate::result::{Phase, Snapshot, ThroughputResult};

struct AccumulatorInner {
    state: Mutex<AccumulatorState>,
    tx: watch::Sender<Snapshot>,
}

#[non_exhaustive]
#[derive(Clone)]
pub struct MetricsAccumulator {
    inner: Arc<AccumulatorInner>,
    interval: Duration,
    warmup: Duration,
}

struct AccumulatorState {
    phase: Phase,
    started_at: Instant,
    interval_bytes: u64,
    total_bytes: u64,
    bytes_at_warmup_end: Option<u64>,
    samples: Vec<f64>,
}

fn lock_state(inner: &AccumulatorInner) -> MutexGuard<'_, AccumulatorState> {
    // Recover from poisoning rather than panicking. The state is just
    // counters; a poisoned lock means a previous holder panicked, but the
    // counter values themselves remain coherent for our purposes.
    inner.state.lock().unwrap_or_else(|p| p.into_inner())
}

impl MetricsAccumulator {
    pub fn new(interval: Duration, warmup: Duration) -> Self {
        let (tx, _rx) = watch::channel(Snapshot::default());
        let state = AccumulatorState {
            phase: Phase::Latency,
            started_at: Instant::now(),
            interval_bytes: 0,
            total_bytes: 0,
            bytes_at_warmup_end: None,
            samples: Vec::new(),
        };
        Self {
            inner: Arc::new(AccumulatorInner {
                state: Mutex::new(state),
                tx,
            }),
            interval,
            warmup,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<Snapshot> {
        self.inner.tx.subscribe()
    }

    pub fn record_bytes(&self, n: u64) {
        let mut state = lock_state(&self.inner);
        state.interval_bytes = state.interval_bytes.saturating_add(n);
        state.total_bytes = state.total_bytes.saturating_add(n);
    }

    pub fn set_phase(&self, phase: Phase) {
        let mut state = lock_state(&self.inner);
        state.phase = phase;
    }

    pub fn start_ticking(&self) -> tokio::task::JoinHandle<()> {
        let inner = Arc::clone(&self.inner);
        let tick_interval = self.interval;
        let warmup = self.warmup;
        let mut ticker = tokio::time::interval(tick_interval);
        // Delay (rather than the default Burst) keeps the per-tick spacing
        // honest under paused-time tests: each test `advance(period)` fires
        // one tick, not a flurry of catch-up ticks.
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        tokio::spawn(async move {
            loop {
                ticker.tick().await;
                let snapshot = {
                    let mut state = lock_state(&inner);
                    let elapsed = state.started_at.elapsed();
                    let interval_bytes = state.interval_bytes;
                    state.interval_bytes = 0;

                    let current_mbps =
                        (interval_bytes as f64 * 8.0) / tick_interval.as_secs_f64() / 1e6;

                    if state.bytes_at_warmup_end.is_none() && elapsed >= warmup {
                        state.bytes_at_warmup_end = Some(state.total_bytes);
                    }
                    if state.bytes_at_warmup_end.is_some() {
                        state.samples.push(current_mbps);
                    }

                    Snapshot {
                        elapsed,
                        phase: state.phase.clone(),
                        current_mbps,
                        bytes_so_far: state.total_bytes,
                    }
                };
                // Send errors mean no subscribers — fine, we keep ticking.
                let _ = inner.tx.send(snapshot);
            }
        })
    }

    pub fn finish(
        &self,
        connections_configured: usize,
        connections_active: usize,
    ) -> ThroughputResult {
        let (samples, bytes) = {
            let state = lock_state(&self.inner);
            let baseline = state.bytes_at_warmup_end.unwrap_or(state.total_bytes);
            let bytes = state.total_bytes.saturating_sub(baseline);
            (state.samples.clone(), bytes)
        };

        // Degenerate case: warm-up longer than test duration → no samples.
        let (mbps, mbps_p50, mbps_p95) = if samples.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let n = samples.len();
            let mean = samples.iter().copied().sum::<f64>() / n as f64;
            let mut sorted = samples;
            sorted.sort_by(|a, b| a.total_cmp(b));
            let p50 = sorted[n / 2];
            let p95 = sorted[(n * 95) / 100];
            (mean, p50, p95)
        };

        ThroughputResult {
            mbps,
            mbps_p50,
            mbps_p95,
            bytes,
            connections_configured,
            connections_active,
        }
    }
}
