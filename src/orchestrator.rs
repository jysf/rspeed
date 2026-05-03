use std::time::{Duration, Instant};

use chrono::Utc;
use futures::StreamExt;
use tokio::sync::watch;

use crate::backend::{Backend, DownloadOpts, UploadOpts};
use crate::config::Config;
use crate::error::TestError;
use crate::metrics::MetricsAccumulator;
use crate::result::{Phase, Snapshot, TestResult, ThroughputResult, compute_latency_result};

pub const DEFAULT_LATENCY_SAMPLES: usize = 10;
pub const DEFAULT_DOWNLOAD_BYTES_PER_REQUEST: u64 = 1_000_000_000;
pub const DEFAULT_UPLOAD_BYTES_PER_REQUEST: u64 = 10 * 1024 * 1024;
pub const DEFAULT_SNAPSHOT_INTERVAL: Duration = Duration::from_millis(100);
pub const DEFAULT_WARMUP: Duration = Duration::from_secs(2); // DEC-005

pub struct TestSession {
    backend: Box<dyn Backend + Send + Sync>,
    config: Config,
    /// Outer broadcast seam. STAGE-003 renderers and v2 monitor
    /// dashboards subscribe via `snapshot_rx()`. Per-phase
    /// `MetricsAccumulator` snapshots are forwarded into this sender
    /// by short-lived spawned tasks.
    snapshot_tx: watch::Sender<Snapshot>,
    /// Per architect decision B-1: cached so per-phase accumulators
    /// can be constructed with non-default cadence in tests via
    /// `with_intervals(...)`. Production `new()` initialises both
    /// to the `DEFAULT_*` constants.
    snapshot_interval: Duration,
    warmup: Duration,
}

impl TestSession {
    pub fn new(backend: Box<dyn Backend + Send + Sync>, config: Config) -> Self {
        Self::with_intervals(backend, config, DEFAULT_SNAPSHOT_INTERVAL, DEFAULT_WARMUP)
    }

    /// Per architect decision B-1: extension point for tests and
    /// future bench tools that need non-default cadence/warmup.
    /// Production callers use `new()` to get the `DEFAULT_*` constants.
    pub fn with_intervals(
        backend: Box<dyn Backend + Send + Sync>,
        config: Config,
        snapshot_interval: Duration,
        warmup: Duration,
    ) -> Self {
        let (snapshot_tx, _rx) = watch::channel(Snapshot::default());
        Self {
            backend,
            config,
            snapshot_tx,
            snapshot_interval,
            warmup,
        }
    }

    pub fn snapshot_rx(&self) -> watch::Receiver<Snapshot> {
        self.snapshot_tx.subscribe()
    }

    pub async fn run(&self) -> Result<TestResult, TestError> {
        let started_at = Utc::now();
        let backend_name = self.backend.name().to_string();

        // Phase 1: latency
        let outcome = self
            .backend
            .latency_probe(DEFAULT_LATENCY_SAMPLES)
            .await
            .map_err(TestError::Latency)?;
        let latency = compute_latency_result(outcome.method, &outcome.samples);

        // Phase 2: download (if enabled)
        let mut measurement_secs = 0.0_f64;
        let download = if self.config.do_download {
            let (result, secs) = self.run_download_phase().await?;
            measurement_secs += secs;
            Some(result)
        } else {
            None
        };

        // Phase 3: upload (if enabled)
        let upload = if self.config.do_upload {
            let (result, secs) = self.run_upload_phase().await?;
            measurement_secs += secs;
            Some(result)
        } else {
            None
        };

        Ok(TestResult {
            started_at,
            backend: backend_name,
            server_url: self.config.server_url_string(),
            ip_version: self.config.ip_version_string(),
            duration_secs: measurement_secs,
            latency,
            download,
            upload,
        })
    }

    async fn run_download_phase(&self) -> Result<(ThroughputResult, f64), TestError> {
        let acc = MetricsAccumulator::new(self.snapshot_interval, self.warmup);
        acc.set_phase(Phase::Download);
        let ticker = acc.start_ticking();
        let forwarder = self.spawn_forwarder(acc.subscribe());

        let opts = DownloadOpts::new(DEFAULT_DOWNLOAD_BYTES_PER_REQUEST, self.config.connections);
        let result = async {
            let mut stream = self
                .backend
                .download(&opts)
                .await
                .map_err(TestError::Download)?;

            let phase_start = Instant::now();
            let duration = Duration::from_secs(self.config.duration_secs as u64);
            loop {
                let Some(remaining) = duration.checked_sub(phase_start.elapsed()) else {
                    break;
                };
                match tokio::time::timeout(remaining, stream.next()).await {
                    Ok(Some(Ok(chunk))) => acc.record_bytes(chunk.len() as u64),
                    Ok(Some(Err(e))) => return Err(TestError::Download(e)),
                    Ok(None) => break, // server closed early
                    Err(_) => break,   // duration reached
                }
            }
            drop(stream);

            let measurement_secs = measurement_window(phase_start.elapsed(), self.warmup);
            let throughput = acc.finish(
                self.config.connections as usize,
                self.config.connections as usize,
            );
            Ok((throughput, measurement_secs))
        }
        .await;

        // A-2: explicit abort eliminates the previous-phase-snapshot race.
        // Aborts are no-ops if the tasks have already exited naturally.
        forwarder.abort();
        ticker.abort();

        result
    }

    async fn run_upload_phase(&self) -> Result<(ThroughputResult, f64), TestError> {
        let acc = MetricsAccumulator::new(self.snapshot_interval, self.warmup);
        acc.set_phase(Phase::Upload);
        let ticker = acc.start_ticking();
        let forwarder = self.spawn_forwarder(acc.subscribe());

        let opts = UploadOpts::new(DEFAULT_UPLOAD_BYTES_PER_REQUEST, self.config.connections);

        let result = async {
            let phase_start = Instant::now();
            let duration = Duration::from_secs(self.config.duration_secs as u64);
            while phase_start.elapsed() < duration {
                let r = self
                    .backend
                    .upload(&opts)
                    .await
                    .map_err(TestError::Upload)?;
                acc.record_bytes(r.bytes_sent);
            }

            let measurement_secs = measurement_window(phase_start.elapsed(), self.warmup);
            let throughput = acc.finish(
                self.config.connections as usize,
                self.config.connections as usize,
            );
            Ok((throughput, measurement_secs))
        }
        .await;

        // A-2: explicit abort eliminates the previous-phase-snapshot race.
        forwarder.abort();
        ticker.abort();

        result
    }

    fn spawn_forwarder(&self, mut rx: watch::Receiver<Snapshot>) -> tokio::task::JoinHandle<()> {
        let outer = self.snapshot_tx.clone();
        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let snap = rx.borrow_and_update().clone();
                // Send error means no subscribers — fine, we keep forwarding.
                let _ = outer.send(snap);
            }
        })
    }
}

/// Patch (E): clarity helper for the per-phase measurement window.
fn measurement_window(elapsed: Duration, warmup: Duration) -> f64 {
    elapsed.saturating_sub(warmup).as_secs_f64()
}
