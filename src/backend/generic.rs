//! Generic HTTP backend stub. Implements DEC-003's documented
//! protocol against a user-supplied URL. STAGE-002 fills in
//! download/upload/latency.

use async_trait::async_trait;
use std::time::Duration;
use url::Url;

use super::{Backend, BackendError, DownloadOpts, DownloadStream, UploadOpts, UploadResult};

#[derive(Debug)]
pub struct GenericHttpBackend {
    base_url: Url,
}

impl GenericHttpBackend {
    pub fn new(base_url: Url) -> Self {
        Self { base_url }
    }

    /// Returns the base URL this backend was constructed with.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
}

#[async_trait]
impl Backend for GenericHttpBackend {
    fn name(&self) -> &'static str {
        "generic"
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
