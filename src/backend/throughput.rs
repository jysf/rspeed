use bytes::Bytes;
use futures::StreamExt;
use futures::stream::BoxStream;
use reqwest::Client;
use std::time::{Duration, Instant};
use url::Url;

use super::{BackendError, DownloadOpts, DownloadStream, UploadOpts, UploadResult};

pub async fn download_one(
    client: &Client,
    url: Url,
) -> Result<
    impl futures::Stream<Item = Result<Bytes, BackendError>> + Send + 'static + use<>,
    BackendError,
> {
    let response = client
        .get(url)
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(BackendError::Network)?;

    let status = response.status();
    if !status.is_success() {
        return Err(BackendError::Protocol(format!(
            "download returned HTTP {}",
            status.as_u16()
        )));
    }

    Ok(response
        .bytes_stream()
        .map(|r| r.map_err(BackendError::Network)))
}

pub async fn upload_one(client: &Client, url: Url, body: Bytes) -> Result<Duration, BackendError> {
    let body_len = body.len();
    let start = Instant::now();

    let response = client
        .post(url)
        .header("Accept-Encoding", "identity")
        .header("Content-Length", body_len.to_string())
        .body(body)
        .send()
        .await
        .map_err(BackendError::Network)?;

    let elapsed = start.elapsed();

    let status = response.status();
    if !status.is_success() {
        return Err(BackendError::Protocol(format!(
            "upload returned HTTP {}",
            status.as_u16()
        )));
    }

    Ok(elapsed)
}

pub fn build_download_url(base: &Url, bytes: u64) -> Result<Url, BackendError> {
    let mut url = base.clone();
    url.query_pairs_mut()
        .append_pair("bytes", &bytes.to_string());
    Ok(url)
}

pub async fn download_parallel(
    client: &Client,
    download_base_url: &Url,
    opts: &DownloadOpts,
) -> Result<DownloadStream, BackendError> {
    if opts.connections == 0 {
        return Err(BackendError::Protocol(
            "connections must be > 0".to_string(),
        ));
    }
    let n = opts.connections as usize;
    let bytes_per = opts.bytes_per_request;

    let futures_list: Vec<_> = (0..n)
        .map(|_| {
            let client = client.clone();
            let url_result = build_download_url(download_base_url, bytes_per);
            async move {
                let url = url_result?;
                download_one(&client, url).await
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

pub async fn upload_parallel(
    client: &Client,
    upload_url: &Url,
    opts: &UploadOpts,
) -> Result<UploadResult, BackendError> {
    if opts.connections == 0 {
        return Err(BackendError::Protocol(
            "connections must be > 0".to_string(),
        ));
    }
    let n = opts.connections as usize;
    let bytes_per = opts.bytes_per_request;

    // DEC-005: one allocation per upload() call, cloned per connection.
    let body = Bytes::from(vec![0u8; bytes_per as usize]);
    let start = Instant::now();

    let futures_list: Vec<_> = (0..n)
        .map(|_| {
            let client = client.clone();
            let url = upload_url.clone();
            let body = body.clone();
            async move { upload_one(&client, url, body).await }
        })
        .collect();

    futures::future::try_join_all(futures_list).await?;
    let elapsed = start.elapsed();
    Ok(UploadResult::new(bytes_per * (n as u64), elapsed))
}
