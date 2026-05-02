//! Cloudflare backend. STAGE-002 fills in download/upload.

use std::time::Duration;

use async_trait::async_trait;
use url::Url;

use super::{
    Backend, BackendError, DownloadOpts, DownloadStream, LatencyProbeOutcome, UploadOpts,
    UploadResult,
};

#[derive(Debug)]
pub struct CloudflareBackend {
    client: reqwest::Client,
    ping_url: Url,
    tcp_target: String,
}

impl CloudflareBackend {
    pub fn new() -> Result<Self, BackendError> {
        let client = reqwest::Client::builder().no_proxy().build()?;
        let ping_url = "https://speed.cloudflare.com/__ping"
            .parse()
            .map_err(|e: url::ParseError| BackendError::Protocol(e.to_string()))?;
        Ok(Self {
            client,
            ping_url,
            tcp_target: "speed.cloudflare.com:443".to_string(),
        })
    }
}

#[async_trait]
impl Backend for CloudflareBackend {
    fn name(&self) -> &'static str {
        "cloudflare"
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

    async fn download(&self, _opts: &DownloadOpts) -> Result<DownloadStream, BackendError> {
        Err(BackendError::NotImplemented)
    }

    async fn upload(&self, _opts: &UploadOpts) -> Result<UploadResult, BackendError> {
        Err(BackendError::NotImplemented)
    }
}
