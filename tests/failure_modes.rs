#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Adversarial failure-mode integration tests for SPEC-013.
//! All tests drive TestSession::run() end-to-end against MockServer.

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use common::{MockOptions, MockServer};
use rspeed::config::IpVersion;
use rspeed::{BackendError, ColorWhen, Config, Format, GenericHttpBackend, TestError, TestSession};

fn build_config(mock: &MockServer) -> Config {
    Config {
        duration_secs: 2,
        connections: 1,
        server: Some(mock.base_url()),
        do_download: true,
        do_upload: false,
        format: Format::Json,
        color: ColorWhen::Never,
        ip_version: IpVersion::Auto,
        verbose: 0,
    }
}

#[tokio::test]
async fn latency_rtt_timeout_triggers_tcp_fallback() {
    // ping_delay > 1s per-request timeout → HTTP RTT fails, TCP fallback runs.
    let mock = MockServer::start_with_options(MockOptions {
        ping_delay: Some(Duration::from_secs(2)),
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = Config {
        do_download: false,
        do_upload: false,
        ..build_config(&mock)
    };
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await.unwrap();
    assert_eq!(
        result.latency.method, "tcp_connect",
        "expected TCP fallback after HTTP RTT timeout"
    );
    assert!(result.latency.samples > 0);
}

#[tokio::test]
async fn download_timeout_surfaces_test_error_download() {
    let mock = MockServer::start_with_options(MockOptions {
        download_delay: Some(Duration::from_secs(2)),
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let session = TestSession::with_intervals(
        backend,
        build_config(&mock),
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    )
    .with_deadlines(Duration::from_millis(500), rspeed::DEFAULT_UPLOAD_DEADLINE);

    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Download(BackendError::Timeout(_)))),
        "expected Download(Timeout), got: {result:?}"
    );
}

#[tokio::test]
async fn upload_timeout_surfaces_test_error_upload() {
    let mock = MockServer::start_with_options(MockOptions {
        upload_delay: Some(Duration::from_secs(2)),
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = Config {
        do_download: false,
        do_upload: true,
        ..build_config(&mock)
    };
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    )
    .with_deadlines(
        rspeed::DEFAULT_DOWNLOAD_DEADLINE,
        Duration::from_millis(500),
    );

    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Upload(BackendError::Timeout(_)))),
        "expected Upload(Timeout), got: {result:?}"
    );
}

#[tokio::test]
async fn download_mid_stream_truncation_surfaces_network_error() {
    // Request 1MB but truncate after 64KB (one chunk). Content-Length: 1MB.
    let mock = MockServer::start_with_options(MockOptions {
        download_truncate_at: Some(64 * 1024),
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let session = TestSession::with_intervals(
        backend,
        build_config(&mock),
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Download(BackendError::Network(_)))),
        "expected Download(Network), got: {result:?}"
    );
}

#[tokio::test]
async fn download_non_2xx_via_orchestrator() {
    let mock = MockServer::start_with_options(MockOptions {
        download_status: StatusCode::INTERNAL_SERVER_ERROR,
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let session = TestSession::with_intervals(
        backend,
        build_config(&mock),
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Download(BackendError::Protocol(_)))),
        "expected Download(Protocol), got: {result:?}"
    );
}

#[tokio::test]
async fn upload_non_2xx_via_orchestrator() {
    let mock = MockServer::start_with_options(MockOptions {
        upload_status: StatusCode::INTERNAL_SERVER_ERROR,
        ..MockOptions::default()
    })
    .await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = Config {
        do_download: false,
        do_upload: true,
        ..build_config(&mock)
    };
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await;
    assert!(
        matches!(result, Err(TestError::Upload(BackendError::Protocol(_)))),
        "expected Upload(Protocol), got: {result:?}"
    );
}
