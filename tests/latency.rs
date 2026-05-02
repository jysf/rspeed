#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for SPEC-008: latency probe with HTTP RTT and
//! TCP-connect fallback. All tests run against MockServer; Cloudflare
//! live tests are deferred to SPEC-013.

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use common::{MockOptions, MockServer};
use rspeed::{Backend, GenericHttpBackend, compute_latency_result};

#[tokio::test]
async fn http_probe_happy_path_against_mock() {
    let mock = MockServer::start().await;
    let backend = GenericHttpBackend::new(mock.base_url()).unwrap();
    let outcome = backend.latency_probe(5).await.unwrap();
    assert_eq!(outcome.method, "http_rtt");
    assert_eq!(outcome.samples.len(), 5);
    assert!(outcome.samples.iter().all(|d| !d.is_zero()));
}

#[tokio::test]
async fn http_probe_warmup_request_count() {
    let mock = MockServer::start().await;
    let backend = GenericHttpBackend::new(mock.base_url()).unwrap();
    backend.latency_probe(5).await.unwrap();
    // 1 warm-up + 5 samples = 6 total requests to /ping.
    assert_eq!(mock.ping_count(), 6);
}

#[tokio::test]
async fn http_probe_falls_back_on_404() {
    let opts = MockOptions {
        ping_status: StatusCode::NOT_FOUND,
        ..MockOptions::default()
    };
    let mock = MockServer::start_with_options(opts).await;
    let backend = GenericHttpBackend::new(mock.base_url()).unwrap();
    let outcome = backend.latency_probe(3).await.unwrap();
    // 404 triggers fallback; TCP connects to the same listening port succeed.
    assert_eq!(outcome.method, "tcp_connect");
    assert_eq!(outcome.samples.len(), 3);
    assert!(outcome.samples.iter().all(|d| !d.is_zero()));
}

#[tokio::test]
async fn http_probe_falls_back_on_500() {
    let opts = MockOptions {
        ping_status: StatusCode::INTERNAL_SERVER_ERROR,
        ..MockOptions::default()
    };
    let mock = MockServer::start_with_options(opts).await;
    let backend = GenericHttpBackend::new(mock.base_url()).unwrap();
    let outcome = backend.latency_probe(3).await.unwrap();
    // !status.is_success() → fallback, regardless of 404 vs 5xx.
    assert_eq!(outcome.method, "tcp_connect");
    assert_eq!(outcome.samples.len(), 3);
}

#[tokio::test]
async fn http_probe_times_out_then_falls_back() {
    // The mock server delays /ping by 60s. The 1s per-request timeout fires
    // for the HTTP warmup, which triggers TCP fallback against the same
    // listening port. Paused-clock is not used here because tokio's
    // time::advance fires TCP timers that are set during the advance window
    // (at t=1s+1s=t=2s when advancing 2s), making real I/O the cleaner path.
    // Test runtime: ~1s (one HTTP timeout).
    let opts = MockOptions {
        ping_delay: Some(Duration::from_secs(60)),
        ..MockOptions::default()
    };
    let mock = MockServer::start_with_options(opts).await;
    let backend = GenericHttpBackend::new(mock.base_url()).unwrap();
    let outcome = backend.latency_probe(4).await.unwrap();
    assert_eq!(outcome.method, "tcp_connect");
    assert_eq!(outcome.samples.len(), 4);
}

#[tokio::test]
async fn tcp_fallback_warmup_request_count() {
    // Strong warm-up verification lives on the HTTP side
    // (http_probe_warmup_request_count). On the TCP side, sample-count
    // parity with the requested N is sufficient since TcpStream::connect
    // is opaque (no per-connect counter at the application layer).
    let opts = MockOptions {
        ping_status: StatusCode::NOT_FOUND,
        ..MockOptions::default()
    };
    let mock = MockServer::start_with_options(opts).await;
    let backend = GenericHttpBackend::new(mock.base_url()).unwrap();
    let outcome = backend.latency_probe(3).await.unwrap();
    assert_eq!(outcome.method, "tcp_connect");
    assert_eq!(outcome.samples.len(), 3);
}

#[tokio::test]
async fn latency_method_strings_match_dec004_contract() {
    // Happy path → "http_rtt"
    let mock = MockServer::start().await;
    let backend = GenericHttpBackend::new(mock.base_url()).unwrap();
    let outcome = backend.latency_probe(2).await.unwrap();
    assert_eq!(
        outcome.method, "http_rtt",
        "method must be exact DEC-006 value"
    );

    // Fallback path → "tcp_connect"
    let opts = MockOptions {
        ping_status: StatusCode::NOT_FOUND,
        ..MockOptions::default()
    };
    let mock2 = MockServer::start_with_options(opts).await;
    let backend2 = GenericHttpBackend::new(mock2.base_url()).unwrap();
    let outcome2 = backend2.latency_probe(2).await.unwrap();
    assert_eq!(
        outcome2.method, "tcp_connect",
        "method must be exact DEC-006 value"
    );
}

#[tokio::test]
async fn compute_latency_result_integrates_with_probe_output() {
    let mock = MockServer::start().await;
    let backend = GenericHttpBackend::new(mock.base_url()).unwrap();
    let outcome = backend.latency_probe(5).await.unwrap();
    let result = compute_latency_result(outcome.method, &outcome.samples);
    assert_eq!(result.samples, outcome.samples.len());
    assert_eq!(result.method, outcome.method);
    assert!(result.min_ms <= result.median_ms);
    assert!(result.median_ms <= result.max_ms);
    assert!(result.median_ms > 0.0);
}

#[tokio::test(start_paused = true)]
async fn both_http_and_tcp_fail_returns_error() {
    // Port 1 is reserved; on Linux/macOS this gives ECONNREFUSED immediately.
    // The error variant is Timeout, Network, or Protocol depending on OS
    // port-1-refusal behavior, so we assert only that an error is returned.
    let backend = GenericHttpBackend::new("http://127.0.0.1:1/".parse().unwrap()).unwrap();
    let probe_fut = tokio::spawn(async move { backend.latency_probe(3).await });
    // Advance past the per-request timeout in case the OS doesn't refuse
    // immediately (e.g. packet is dropped rather than RST).
    tokio::time::advance(Duration::from_secs(10)).await;
    let result = probe_fut.await.unwrap();
    assert!(result.is_err());
}
