//! Backend abstraction. The `Backend` trait is the seam between
//! "what we measure" (downloader/uploader/latency probe in STAGE-002)
//! and "where we measure against" (Cloudflare default; user-supplied
//! HTTP server). See DEC-003.

mod cloudflare;
mod generic;
mod select;

pub use cloudflare::CloudflareBackend;
pub use generic::GenericHttpBackend;
pub use select::select;

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use std::time::Duration;

/// The seam between measurement code (STAGE-002) and backend-specific
/// transport. Provisional shape — STAGE-002 may evolve.
#[async_trait]
pub trait Backend: Send + Sync {
    fn name(&self) -> &'static str;

    async fn latency_probe(&self, samples: usize) -> Result<Vec<Duration>, BackendError>;

    async fn download(&self, opts: &DownloadOpts) -> Result<DownloadStream, BackendError>;

    async fn upload(&self, opts: &UploadOpts) -> Result<UploadResult, BackendError>;
}

pub type DownloadStream = BoxStream<'static, Result<Bytes, BackendError>>;

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct DownloadOpts {
    pub bytes_per_request: u64,
    pub connections: u8,
}

impl DownloadOpts {
    pub fn new(bytes_per_request: u64, connections: u8) -> Self {
        Self {
            bytes_per_request,
            connections,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct UploadOpts {
    pub bytes_per_request: u64,
    pub connections: u8,
}

impl UploadOpts {
    pub fn new(bytes_per_request: u64, connections: u8) -> Self {
        Self {
            bytes_per_request,
            connections,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct UploadResult {
    pub bytes_sent: u64,
    pub elapsed: Duration,
}

impl UploadResult {
    pub fn new(bytes_sent: u64, elapsed: Duration) -> Self {
        Self {
            bytes_sent,
            elapsed,
        }
    }
}

/// Errors crossing the `Backend` trait boundary.
///
/// Per AGENTS.md exit code table, the orchestrator (STAGE-002) is
/// responsible for translating variants to process exit codes:
/// `Network` → 3, `Protocol` → 4. The lib does not translate; the
/// `main.rs` shim does (via `anyhow::Error::downcast_ref::<BackendError>()`
/// once STAGE-002 wires the orchestrator).
///
/// Marked `#[non_exhaustive]` so STAGE-002 can add `Timeout`,
/// `Cancelled`, `BodyTooLarge`, etc. without a semver-breaking change.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("not yet implemented")]
    NotImplemented,
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
}
