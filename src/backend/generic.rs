//! Generic HTTP backend. Implements DEC-003's documented protocol
//! against a user-supplied URL. STAGE-002 fills in download/upload.

use std::time::Duration;

use async_trait::async_trait;
use url::Url;

use super::{
    Backend, BackendError, DownloadOpts, DownloadStream, LatencyProbeOutcome, UploadOpts,
    UploadResult, throughput,
};

#[derive(Debug)]
pub struct GenericHttpBackend {
    base_url: Url,
    client: reqwest::Client,
    ping_url: Url,
    tcp_target: String,
    download_base_url: Url,
    upload_url: Url,
}

impl GenericHttpBackend {
    pub fn new(base_url: Url) -> Result<Self, BackendError> {
        let client = reqwest::Client::builder().no_proxy().build()?;

        let ping_url = base_url
            .join("ping")
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        let host = base_url
            .host_str()
            .ok_or_else(|| BackendError::Protocol("base URL missing host".to_string()))?;
        let port = base_url
            .port_or_known_default()
            .ok_or_else(|| BackendError::Protocol("base URL missing port".to_string()))?;
        let tcp_target = format!("{host}:{port}");

        let download_base_url = base_url
            .join("download")
            .map_err(|e| BackendError::Protocol(e.to_string()))?;
        let upload_url = base_url
            .join("upload")
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        Ok(Self {
            base_url,
            client,
            ping_url,
            tcp_target,
            download_base_url,
            upload_url,
        })
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

    async fn latency_probe(&self, samples: usize) -> Result<LatencyProbeOutcome, BackendError> {
        super::latency::probe(
            &self.client,
            &self.ping_url,
            &self.tcp_target,
            samples,
            Duration::from_secs(1),
        )
        .await
    }

    async fn download(&self, opts: &DownloadOpts) -> Result<DownloadStream, BackendError> {
        throughput::download_parallel(&self.client, &self.download_base_url, opts).await
    }

    async fn upload(&self, opts: &UploadOpts) -> Result<UploadResult, BackendError> {
        throughput::upload_parallel(&self.client, &self.upload_url, opts).await
    }
}
