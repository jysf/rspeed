//! Cloudflare backend stub. STAGE-002 fills in download/upload/latency.

use async_trait::async_trait;
use std::time::Duration;

use super::{Backend, BackendError, DownloadOpts, DownloadStream, UploadOpts, UploadResult};

#[derive(Debug, Default)]
pub struct CloudflareBackend {
    // STAGE-002 will store a reqwest::Client here. For SPEC-005,
    // the stub doesn't make any requests.
}

#[async_trait]
impl Backend for CloudflareBackend {
    fn name(&self) -> &'static str {
        "cloudflare"
    }

    async fn latency_probe(&self, _samples: usize) -> Result<Vec<Duration>, BackendError> {
        Err(BackendError::NotImplemented)
    }

    async fn download(&self, _opts: &DownloadOpts) -> Result<DownloadStream, BackendError> {
        Err(BackendError::NotImplemented)
    }

    async fn upload(&self, _opts: &UploadOpts) -> Result<UploadResult, BackendError> {
        Err(BackendError::NotImplemented)
    }
}
