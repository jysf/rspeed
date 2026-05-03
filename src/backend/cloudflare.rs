//! Cloudflare backend.

use std::time::Duration;

use async_trait::async_trait;
use url::Url;

use super::{
    Backend, BackendError, DownloadOpts, DownloadStream, LatencyProbeOutcome, UploadOpts,
    UploadResult, throughput,
};

#[derive(Debug)]
pub struct CloudflareBackend {
    client: reqwest::Client,
    ping_url: Url,
    tcp_target: String,
    download_base_url: Url,
    upload_url: Url,
}

impl CloudflareBackend {
    pub fn new() -> Result<Self, BackendError> {
        let client = reqwest::Client::builder().no_proxy().build()?;
        let ping_url = "https://speed.cloudflare.com/__ping"
            .parse()
            .map_err(|e: url::ParseError| BackendError::Protocol(e.to_string()))?;
        let download_base_url = "https://speed.cloudflare.com/__down"
            .parse()
            .map_err(|e: url::ParseError| BackendError::Protocol(e.to_string()))?;
        let upload_url = "https://speed.cloudflare.com/__up"
            .parse()
            .map_err(|e: url::ParseError| BackendError::Protocol(e.to_string()))?;
        Ok(Self {
            client,
            ping_url,
            tcp_target: "speed.cloudflare.com:443".to_string(),
            download_base_url,
            upload_url,
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

    async fn download(&self, opts: &DownloadOpts) -> Result<DownloadStream, BackendError> {
        throughput::download_parallel(&self.client, &self.download_base_url, opts).await
    }

    async fn upload(&self, opts: &UploadOpts) -> Result<UploadResult, BackendError> {
        throughput::upload_parallel(&self.client, &self.upload_url, opts).await
    }
}
