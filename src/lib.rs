//! rspeed library API.

use std::io::{self, Write};

use clap::Parser;

pub mod backend;
pub mod buffer_pool;
mod cli;
pub mod config;
pub mod error;
pub mod metrics;
pub mod orchestrator;
pub mod result;

pub use backend::{
    Backend, BackendError, CloudflareBackend, DownloadOpts, DownloadStream, GenericHttpBackend,
    LatencyProbeOutcome, UploadOpts, UploadResult,
};
pub use config::{ColorWhen, Config, Format};
pub use error::TestError;
pub use metrics::MetricsAccumulator;
pub use orchestrator::{
    DEFAULT_DOWNLOAD_BYTES_PER_REQUEST, DEFAULT_DOWNLOAD_DEADLINE, DEFAULT_LATENCY_SAMPLES,
    DEFAULT_SNAPSHOT_INTERVAL, DEFAULT_UPLOAD_BYTES_PER_REQUEST, DEFAULT_UPLOAD_DEADLINE,
    DEFAULT_WARMUP, TestSession,
};
pub use result::{
    LatencyResult, Phase, Snapshot, TestResult, ThroughputResult, compute_latency_result,
};

/// Entry point invoked by `main`. Returns a process exit code.
pub fn run() -> anyhow::Result<i32> {
    let cli = cli::Cli::parse();
    let config = Config::from(cli);
    if let Err(e) = config.validate() {
        eprintln!("error: {e:#}");
        return Ok(e.exit_code());
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;

    runtime.block_on(async_run(config))
}

async fn async_run(config: Config) -> anyhow::Result<i32> {
    // Patch (F): backend-construction failures use TestError::Backend(_),
    // not Config(_), so TLS init failures land at exit code 3 (network/system)
    // rather than 2 (config). URL-parse failures inside select() also land here;
    // accepted granularity loss documented in spec body.
    let backend = match backend::select(&config) {
        Ok(b) => b,
        Err(e) => {
            let err = TestError::Backend(e);
            eprintln!("error: {err:#}");
            return Ok(err.exit_code());
        }
    };

    let format = config.format;
    let session = TestSession::new(backend, config);

    let result = match session.run().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e:#}");
            return Ok(e.exit_code());
        }
    };

    match format {
        Format::Json => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            serde_json::to_writer_pretty(&mut handle, &result)?;
            writeln!(handle)?;
        }
        Format::Human => {
            // STAGE-003 implements the human renderer. SPEC-012
            // falls back to JSON with a one-line warning so the user
            // gets *something* useful in the meantime.
            eprintln!("(human renderer coming in STAGE-003 — emitting JSON)");
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            serde_json::to_writer_pretty(&mut handle, &result)?;
            writeln!(handle)?;
        }
        Format::Silent => {}
    }

    Ok(0)
}
