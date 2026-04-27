#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Shared integration-test fixtures. Each integration test file
//! does `mod common;` to gain access. Per project convention,
//! unwrap/expect are allowed here — fixture code should fail loudly
//! if setup goes wrong.

use axum::{
    Json, Router,
    body::Body,
    extract::Query,
    http::header,
    response::Response,
    routing::{get, post},
};
use bytes::Bytes;
use futures::stream;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use url::Url;

const DOWNLOAD_DEFAULT_BYTES: u64 = 1_000_000;
const DOWNLOAD_MAX_BYTES: u64 = 1_000_000_000;
const CHUNK_BYTES: usize = 64 * 1024;

pub struct MockServer {
    addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    handle: JoinHandle<()>,
}

impl MockServer {
    pub async fn start() -> Self {
        let app = Router::new()
            .route("/health", get(health))
            .route("/ping", get(ping))
            .route("/download", get(download))
            .route("/upload", post(upload));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .unwrap();
        });

        Self {
            addr,
            shutdown_tx: Some(tx),
            handle,
        }
    }

    pub fn base_url(&self) -> Url {
        format!("http://{}/", self.addr).parse().unwrap()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.handle.abort();
    }
}

// --- handlers ---

async fn health() -> &'static str {
    "ok"
}

async fn ping() -> &'static str {
    ""
}

#[derive(Deserialize)]
struct DownloadQuery {
    #[serde(default)]
    bytes: Option<u64>,
}

async fn download(Query(q): Query<DownloadQuery>) -> Response {
    let n = q
        .bytes
        .unwrap_or(DOWNLOAD_DEFAULT_BYTES)
        .min(DOWNLOAD_MAX_BYTES);

    let chunk: Bytes = Bytes::from(vec![0u8; CHUNK_BYTES]);
    let full_chunks = n / CHUNK_BYTES as u64;
    let tail = (n % CHUNK_BYTES as u64) as usize;

    let chunks = stream::iter(
        std::iter::repeat_n(chunk.clone(), full_chunks as usize)
            .chain(if tail > 0 {
                Some(chunk.slice(0..tail))
            } else {
                None
            })
            .map(Ok::<_, std::io::Error>),
    );

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, n)
        .body(Body::from_stream(chunks))
        .unwrap()
}

#[derive(Serialize)]
struct UploadResponse {
    received: u64,
}

async fn upload(body: Bytes) -> Json<UploadResponse> {
    Json(UploadResponse {
        received: body.len() as u64,
    })
}
