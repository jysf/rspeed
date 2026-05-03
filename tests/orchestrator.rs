#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! End-to-end orchestrator integration tests for SPEC-012.

mod common;

use std::time::Duration;

use common::MockServer;
use rspeed::config::IpVersion;
use rspeed::{ColorWhen, Config, Format, GenericHttpBackend, Phase, TestError, TestSession};

fn build_config(mock: &MockServer, do_download: bool, do_upload: bool) -> Config {
    Config {
        duration_secs: 1,
        connections: 4,
        server: Some(mock.base_url()),
        do_download,
        do_upload,
        format: Format::Json,
        color: ColorWhen::Never,
        ip_version: IpVersion::Auto,
        verbose: 0,
    }
}

#[tokio::test]
async fn orchestrator_run_against_mock_populates_all_phases() {
    let mock = MockServer::start().await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = build_config(&mock, true, true);
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await.unwrap();

    assert!(result.latency.samples > 0);
    assert_eq!(result.latency.method, "http_rtt");
    assert!(result.download.is_some() && result.download.as_ref().unwrap().bytes > 0);
    assert!(result.upload.is_some());
    // upload.bytes >= 0 always; field populated means is_some()
    assert_eq!(result.backend, "generic");
    assert!(mock.ping_count() >= 1);
    assert!(mock.download_count() >= 1);
    assert!(mock.upload_count() >= 1);
}

#[tokio::test]
async fn orchestrator_run_with_only_latency_skips_throughput_phases() {
    let mock = MockServer::start().await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = build_config(&mock, false, false);
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await.unwrap();

    assert!(result.latency.samples > 0);
    assert!(result.download.is_none());
    assert!(result.upload.is_none());
    assert_eq!(mock.download_count(), 0);
    assert_eq!(mock.upload_count(), 0);
}

#[tokio::test]
async fn orchestrator_test_result_round_trips_through_serde_json() {
    let mock = MockServer::start().await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = build_config(&mock, true, true);
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await.unwrap();

    let json = serde_json::to_string(&result).unwrap();
    let round_tripped: rspeed::TestResult = serde_json::from_str(&json).unwrap();

    assert_eq!(round_tripped.latency.samples, result.latency.samples);
    assert_eq!(round_tripped.latency.method, result.latency.method);
    assert_eq!(round_tripped.download.is_some(), result.download.is_some());
    assert_eq!(round_tripped.upload.is_some(), result.upload.is_some());
    assert_eq!(round_tripped.backend, result.backend);
}

#[tokio::test]
async fn orchestrator_snapshot_rx_observes_phase_transitions() {
    let mock = MockServer::start().await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = build_config(&mock, true, true);
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );

    let mut rx = session.snapshot_rx();
    let collector = tokio::spawn(async move {
        let mut phases = Vec::new();
        while rx.changed().await.is_ok() {
            let phase = rx.borrow_and_update().phase.clone();
            if !phases.contains(&phase) {
                phases.push(phase);
            }
        }
        phases
    });

    let _result = session.run().await.unwrap();
    drop(session); // drops snapshot_tx, closes collector loop

    let phases = collector.await.unwrap();
    assert!(phases.contains(&Phase::Download));
    assert!(phases.contains(&Phase::Upload));
}

#[tokio::test]
async fn orchestrator_skip_download_omits_download_request() {
    let mock = MockServer::start().await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = build_config(&mock, false, true);
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await.unwrap();

    assert!(result.download.is_none());
    assert!(result.upload.is_some());
    assert_eq!(mock.download_count(), 0);
    assert!(mock.upload_count() >= 1);
}

#[tokio::test]
async fn orchestrator_skip_upload_omits_upload_request() {
    let mock = MockServer::start().await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = build_config(&mock, true, false);
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await.unwrap();

    assert!(result.download.is_some());
    assert!(result.upload.is_none());
    assert!(mock.download_count() >= 1);
    assert_eq!(mock.upload_count(), 0);
}

#[tokio::test]
async fn orchestrator_latency_failure_returns_test_error_latency() {
    let backend =
        Box::new(GenericHttpBackend::new("http://127.0.0.1:1/".parse().unwrap()).unwrap());
    let config = Config {
        duration_secs: 1,
        connections: 1,
        server: Some("http://127.0.0.1:1/".parse().unwrap()),
        do_download: true,
        do_upload: true,
        format: Format::Json,
        color: ColorWhen::Never,
        ip_version: IpVersion::Auto,
        verbose: 0,
    };
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );
    let result = session.run().await;
    assert!(matches!(result, Err(TestError::Latency(_))));
}

#[tokio::test]
async fn config_validate_rejects_server_without_trailing_slash() {
    // url::Url normalises bare-host URLs (http://example.com) to include a trailing
    // slash on the path, so we need an explicit path component without one.
    let no_slash: url::Url = "http://example.com/api".parse().unwrap();
    let config = Config {
        duration_secs: 10,
        connections: 4,
        server: Some(no_slash),
        do_download: true,
        do_upload: true,
        format: Format::Json,
        color: ColorWhen::Never,
        ip_version: IpVersion::Auto,
        verbose: 0,
    };
    let err = config.validate().unwrap_err();
    assert!(matches!(err, TestError::Config(ref msg) if msg.contains("trailing slash")));

    let with_slash: url::Url = "http://example.com/".parse().unwrap();
    let config2 = Config {
        duration_secs: 10,
        connections: 4,
        server: Some(with_slash),
        do_download: true,
        do_upload: true,
        format: Format::Json,
        color: ColorWhen::Never,
        ip_version: IpVersion::Auto,
        verbose: 0,
    };
    assert!(config2.validate().is_ok());

    let config3 = Config {
        duration_secs: 10,
        connections: 4,
        server: None,
        do_download: true,
        do_upload: true,
        format: Format::Json,
        color: ColorWhen::Never,
        ip_version: IpVersion::Auto,
        verbose: 0,
    };
    assert!(config3.validate().is_ok());
}

#[tokio::test]
async fn orchestrator_test_session_run_is_repeatable() {
    let mock = MockServer::start().await;
    let backend = Box::new(GenericHttpBackend::new(mock.base_url()).unwrap());
    let config = build_config(&mock, false, false);
    let session = TestSession::with_intervals(
        backend,
        config,
        rspeed::DEFAULT_SNAPSHOT_INTERVAL,
        Duration::ZERO,
    );

    let result1 = session.run().await;
    assert!(result1.is_ok());
    let ping_after_first = mock.ping_count();
    assert!(ping_after_first >= 1);

    let result2 = session.run().await;
    assert!(result2.is_ok());
    let ping_after_second = mock.ping_count();
    assert_eq!(ping_after_second, ping_after_first * 2);
}
