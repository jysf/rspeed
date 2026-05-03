use bytes::Bytes;
use futures::StreamExt;
use reqwest::Client;
use std::time::{Duration, Instant};
use url::Url;

use super::BackendError;

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
