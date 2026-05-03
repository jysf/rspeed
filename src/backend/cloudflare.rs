//! Cloudflare backend.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
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

    fn build_download_url(base: &Url, bytes: u64) -> Result<Url, BackendError> {
        let mut url = base.clone();
        url.query_pairs_mut()
            .append_pair("bytes", &bytes.to_string());
        Ok(url)
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
        if opts.connections == 0 {
            return Err(BackendError::Protocol(
                "connections must be > 0".to_string(),
            ));
        }

        let n = opts.connections as usize;
        let bytes_per = opts.bytes_per_request;

        let futures_list: Vec<_> = (0..n)
            .map(|_| {
                let client = self.client.clone();
                let url_result = Self::build_download_url(&self.download_base_url, bytes_per);
                async move {
                    let url = url_result?;
                    throughput::download_one(&client, url).await
                }
            })
            .collect();

        let streams = futures::future::try_join_all(futures_list).await?;

        let pinned: Vec<BoxStream<'static, Result<Bytes, BackendError>>> = streams
            .into_iter()
            .map(|s| -> BoxStream<'static, Result<Bytes, BackendError>> { Box::pin(s) })
            .collect();

        Ok(Box::pin(futures::stream::select_all(pinned)))
    }

    async fn upload(&self, opts: &UploadOpts) -> Result<UploadResult, BackendError> {
        if opts.connections == 0 {
            return Err(BackendError::Protocol(
                "connections must be > 0".to_string(),
            ));
        }

        let n = opts.connections as usize;
        let bytes_per = opts.bytes_per_request;

        // DEC-005: one allocation per upload() call, cloned per connection (Bytes is refcounted).
        // Note: for large bytes_per_request values this allocation exceeds the 20MB RSS budget.
        // STAGE-004 will stream the upload body via reqwest::Body::wrap_stream() instead.
        let body = Bytes::from(vec![0u8; bytes_per as usize]);

        let start = Instant::now();

        let futures_list: Vec<_> = (0..n)
            .map(|_| {
                let client = self.client.clone();
                let url = self.upload_url.clone();
                let body = body.clone();
                async move { throughput::upload_one(&client, url, body).await }
            })
            .collect();

        futures::future::try_join_all(futures_list).await?;

        let elapsed = start.elapsed();

        Ok(UploadResult::new(bytes_per * (n as u64), elapsed))
    }
}
