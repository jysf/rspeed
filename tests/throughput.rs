#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integration tests for SPEC-010: shared download/upload HTTP mechanics
//! and CloudflareBackend::download/upload against MockServer.

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use bytes::Bytes;
use common::{MockOptions, MockServer};
use futures::StreamExt;
use futures::stream::BoxStream;
use rspeed::backend::throughput::{download_one, upload_one};
use rspeed::{
    Backend, BackendError, CloudflareBackend, DownloadOpts, MetricsAccumulator, UploadOpts,
};

// ── download_one ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn download_one_happy_path_against_mock() {
    let mock = MockServer::start().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("{}download?bytes=1048576", mock.base_url())
        .parse()
        .unwrap();

    let stream = download_one(&client, url).await.unwrap();
    let mut stream = Box::pin(stream);

    let mut total = 0usize;
    while let Some(chunk) = stream.next().await {
        total += chunk.unwrap().len();
    }
    assert_eq!(total, 1_048_576);
}

#[tokio::test]
async fn download_one_non_2xx_returns_protocol_error() {
    let mock = MockServer::start_with_options(MockOptions {
        download_status: StatusCode::INTERNAL_SERVER_ERROR,
        ..MockOptions::default()
    })
    .await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("{}download?bytes=1024", mock.base_url())
        .parse()
        .unwrap();

    let result = download_one(&client, url).await;
    match result {
        Err(BackendError::Protocol(msg)) => assert!(msg.contains("500"), "msg={msg}"),
        Err(other) => panic!("expected Protocol error, got {other:?}"),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

#[tokio::test]
async fn download_one_connection_refused_returns_network_error() {
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = "http://127.0.0.1:1/download?bytes=1".parse().unwrap();

    let result = download_one(&client, url).await;
    match result {
        Err(BackendError::Network(_)) => {}
        Err(other) => panic!("expected Network error, got {other:?}"),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

// ── upload_one ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn upload_one_happy_path_against_mock() {
    let mock = MockServer::start().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("{}upload", mock.base_url()).parse().unwrap();
    let body = Bytes::from(vec![0u8; 64 * 1024]);

    let elapsed = upload_one(&client, url, body).await.unwrap();
    assert!(!elapsed.is_zero());
}

#[tokio::test]
async fn upload_one_non_2xx_returns_protocol_error() {
    let mock = MockServer::start_with_options(MockOptions {
        upload_status: StatusCode::INTERNAL_SERVER_ERROR,
        ..MockOptions::default()
    })
    .await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("{}upload", mock.base_url()).parse().unwrap();
    let body = Bytes::from(vec![0u8; 1024]);

    let result = upload_one(&client, url, body).await;
    match result {
        Err(BackendError::Protocol(msg)) => assert!(msg.contains("500"), "msg={msg}"),
        Err(other) => panic!("expected Protocol error, got {other:?}"),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

// ── parallel ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn parallel_downloads_via_select_all() {
    let mock = MockServer::start().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    let futures_list: Vec<_> = (0..4)
        .map(|_| {
            let c = client.clone();
            let url = format!("{}download?bytes=262144", mock.base_url())
                .parse()
                .unwrap();
            async move { download_one(&c, url).await }
        })
        .collect();

    let streams = futures::future::try_join_all(futures_list).await.unwrap();

    let pinned: Vec<BoxStream<'static, Result<Bytes, BackendError>>> = streams
        .into_iter()
        .map(|s| -> BoxStream<'static, Result<Bytes, BackendError>> { Box::pin(s) })
        .collect();

    let mut merged = futures::stream::select_all(pinned);
    let mut total = 0usize;
    while let Some(chunk) = merged.next().await {
        total += chunk.unwrap().len();
    }

    assert_eq!(total, 4 * 262_144);
    assert_eq!(mock.download_count(), 4);
}

#[tokio::test]
async fn parallel_uploads_via_try_join_all() {
    let mock = MockServer::start().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let body = Bytes::from(vec![0u8; 16 * 1024]);

    let futures_list: Vec<_> = (0..4)
        .map(|_| {
            let c = client.clone();
            let url = format!("{}upload", mock.base_url()).parse().unwrap();
            let b = body.clone();
            async move { upload_one(&c, url, b).await }
        })
        .collect();

    let results = futures::future::try_join_all(futures_list).await.unwrap();

    assert_eq!(results.len(), 4);
    assert!(results.iter().all(|d| !d.is_zero()));
    assert_eq!(mock.upload_count(), 4);
}

// ── CloudflareBackend connections == 0 guard ──────────────────────────────────

#[tokio::test]
async fn connections_zero_returns_error() {
    // The guard fires before any network call, so no live Cloudflare traffic.
    let backend = CloudflareBackend::new().unwrap();

    let dl_result = backend.download(&DownloadOpts::new(1_000_000, 0)).await;
    match dl_result {
        Err(BackendError::Protocol(_)) => {}
        Err(other) => panic!("expected Protocol error for download, got {other:?}"),
        Ok(_) => panic!("expected error for download, got Ok"),
    }

    let ul_result = backend.upload(&UploadOpts::new(1_000, 0)).await;
    match ul_result {
        Err(BackendError::Protocol(_)) => {}
        Err(other) => panic!("expected Protocol error for upload, got {other:?}"),
        Ok(_) => panic!("expected error for upload, got Ok"),
    }
}

// ── MetricsAccumulator integration ───────────────────────────────────────────

#[tokio::test(start_paused = true)]
async fn download_one_serializes_with_metrics_accumulator() {
    let mock = MockServer::start().await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    // warmup=ZERO: first tick sets bytes_at_warmup_end = 0 (before any bytes
    // are recorded), so all subsequently recorded bytes appear in finish().bytes.
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::ZERO);
    let tick_handle = acc.start_ticking();

    // Advance past the first tick interval → warmup boundary fires, bytes_at_warmup_end = 0.
    tokio::time::advance(Duration::from_millis(60)).await;
    tokio::task::yield_now().await;

    // Download 256 KiB. Real I/O works with start_paused (no reqwest timeouts set).
    let url = format!("{}download?bytes=262144", mock.base_url())
        .parse()
        .unwrap();
    let stream = download_one(&client, url).await.unwrap();
    let mut stream = Box::pin(stream);
    while let Some(chunk) = stream.next().await {
        acc.record_bytes(chunk.unwrap().len() as u64);
    }

    // Fire another tick to flush interval_bytes into the sample set.
    tokio::time::advance(Duration::from_millis(60)).await;
    tokio::task::yield_now().await;

    let result = acc.finish(1, 1);
    assert_eq!(result.bytes, 262_144, "expected 262144 post-warmup bytes");

    tick_handle.abort();
}
