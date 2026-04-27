#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Smoke tests for the SPEC-006 mock server fixture.

mod common;

use common::MockServer;
use rspeed::{Backend, GenericHttpBackend};

#[tokio::test]
async fn mock_health_returns_200() {
    let mock = MockServer::start().await;
    let resp = reqwest::get(format!("{}health", mock.base_url()))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn mock_download_returns_requested_bytes() {
    let mock = MockServer::start().await;
    let resp = reqwest::get(format!("{}download?bytes=1024", mock.base_url()))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.bytes().await.unwrap();
    assert_eq!(body.len(), 1024);
}

#[tokio::test]
async fn mock_upload_echoes_byte_count() {
    let mock = MockServer::start().await;
    let payload = vec![0u8; 512];
    let resp = reqwest::Client::new()
        .post(format!("{}upload", mock.base_url()))
        .body(payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let text = resp.text().await.unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["received"], 512);
}

#[tokio::test]
async fn generic_backend_reports_name() {
    let mock = MockServer::start().await;
    let backend = GenericHttpBackend::new(mock.base_url());
    assert_eq!(backend.name(), "generic");
}
