#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use axum::http::StatusCode;
use common::{MockOptions, MockServer};
use futures::StreamExt;
use rspeed::{Backend, BackendError, DownloadOpts, GenericHttpBackend, UploadOpts};

fn build_backend(mock: &MockServer) -> GenericHttpBackend {
    GenericHttpBackend::new(mock.base_url()).unwrap()
}

#[tokio::test]
async fn generic_backend_download_happy_path() {
    let mock = MockServer::start().await;
    let backend = build_backend(&mock);
    let mut stream = backend
        .download(&DownloadOpts::new(1_048_576, 1))
        .await
        .unwrap();
    let mut total = 0usize;
    while let Some(chunk) = stream.next().await {
        total += chunk.unwrap().len();
    }
    assert_eq!(total, 1_048_576);
    assert_eq!(mock.download_count(), 1);
}

#[tokio::test]
async fn generic_backend_upload_happy_path() {
    let mock = MockServer::start().await;
    let backend = build_backend(&mock);
    let result = backend
        .upload(&UploadOpts::new(64 * 1024, 1))
        .await
        .unwrap();
    assert_eq!(result.bytes_sent, 64 * 1024);
    assert!(!result.elapsed.is_zero());
    assert_eq!(mock.upload_count(), 1);
}

#[tokio::test]
async fn generic_backend_download_parallel_connections() {
    let mock = MockServer::start().await;
    let backend = build_backend(&mock);
    let mut stream = backend
        .download(&DownloadOpts::new(256 * 1024, 4))
        .await
        .unwrap();
    let mut total = 0usize;
    while let Some(chunk) = stream.next().await {
        total += chunk.unwrap().len();
    }
    assert_eq!(total, 4 * 256 * 1024);
    assert_eq!(mock.download_count(), 4);
}

#[tokio::test]
async fn generic_backend_upload_parallel_connections() {
    let mock = MockServer::start().await;
    let backend = build_backend(&mock);
    let result = backend
        .upload(&UploadOpts::new(16 * 1024, 4))
        .await
        .unwrap();
    assert_eq!(result.bytes_sent, 4 * 16 * 1024);
    assert_eq!(mock.upload_count(), 4);
}

#[tokio::test]
async fn generic_backend_non_2xx_download_returns_protocol_error() {
    let mock = MockServer::start_with_options(MockOptions {
        download_status: StatusCode::INTERNAL_SERVER_ERROR,
        ..MockOptions::default()
    })
    .await;
    let backend = build_backend(&mock);
    let result = backend.download(&DownloadOpts::new(1024, 1)).await;
    match result {
        Err(BackendError::Protocol(msg)) => assert!(msg.contains("500"), "msg={msg}"),
        Err(other) => panic!("expected Protocol error, got {other:?}"),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

#[tokio::test]
async fn generic_backend_connection_refused_returns_network_error() {
    let backend = GenericHttpBackend::new("http://127.0.0.1:1/".parse().unwrap()).unwrap();
    let result = backend.download(&DownloadOpts::new(1024, 1)).await;
    match result {
        Err(BackendError::Network(_)) => {}
        Err(other) => panic!("expected Network error, got {other:?}"),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

#[tokio::test]
async fn generic_backend_connections_zero_returns_error() {
    let mock = MockServer::start().await;
    let backend = build_backend(&mock);

    let dl_result = backend.download(&DownloadOpts::new(1024, 0)).await;
    match dl_result {
        Err(BackendError::Protocol(_)) => {}
        Err(other) => panic!("expected Protocol error for download, got {other:?}"),
        Ok(_) => panic!("expected error for download, got Ok"),
    }

    let ul_result = backend.upload(&UploadOpts::new(1024, 0)).await;
    match ul_result {
        Err(BackendError::Protocol(_)) => {}
        Err(other) => panic!("expected Protocol error for upload, got {other:?}"),
        Ok(_) => panic!("expected error for upload, got Ok"),
    }
}
