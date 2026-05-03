#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]
//! Shared integration-test fixtures. Each integration test file
//! does `mod common;` to gain access. Per project convention,
//! unwrap/expect are allowed here — fixture code should fail loudly
//! if setup goes wrong.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bytes::Bytes;
use futures::stream;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use url::Url;

const DOWNLOAD_DEFAULT_BYTES: u64 = 1_000_000;
const DOWNLOAD_MAX_BYTES: u64 = 1_000_000_000;
const CHUNK_BYTES: usize = 64 * 1024;

/// Options for the mock server endpoints. `Default` reproduces the
/// original SPEC-006 behavior: 200 OK for all endpoints, no delay,
/// counters increment.
#[derive(Clone)]
pub struct MockOptions {
    /// HTTP status code returned by /ping. Default: 200 OK.
    pub ping_status: StatusCode,
    /// Optional delay before /ping responds (uses tokio::time::sleep,
    /// so paused-clock tests can advance over it).
    pub ping_delay: Option<Duration>,
    /// HTTP status code returned by /download. Default: 200 OK.
    pub download_status: StatusCode,
    /// HTTP status code returned by /upload. Default: 200 OK.
    pub upload_status: StatusCode,
}

impl Default for MockOptions {
    fn default() -> Self {
        Self {
            ping_status: StatusCode::OK,
            ping_delay: None,
            download_status: StatusCode::OK,
            upload_status: StatusCode::OK,
        }
    }
}

#[derive(Clone)]
struct AppState {
    ping_counter: Arc<AtomicU64>,
    ping_status: StatusCode,
    ping_delay: Option<Duration>,
    download_counter: Arc<AtomicU64>,
    download_status: StatusCode,
    upload_counter: Arc<AtomicU64>,
    upload_status: StatusCode,
}

pub struct MockServer {
    addr: SocketAddr,
    ping_counter: Arc<AtomicU64>,
    download_counter: Arc<AtomicU64>,
    upload_counter: Arc<AtomicU64>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    handle: JoinHandle<()>,
}

impl MockServer {
    /// Start with default options (reproduces SPEC-006 behavior).
    pub async fn start() -> Self {
        Self::start_with_options(MockOptions::default()).await
    }

    /// Start with custom options (e.g. a non-2xx status or a delay).
    pub async fn start_with_options(opts: MockOptions) -> Self {
        let ping_counter = Arc::new(AtomicU64::new(0));
        let download_counter = Arc::new(AtomicU64::new(0));
        let upload_counter = Arc::new(AtomicU64::new(0));

        let state = AppState {
            ping_counter: ping_counter.clone(),
            ping_status: opts.ping_status,
            ping_delay: opts.ping_delay,
            download_counter: download_counter.clone(),
            download_status: opts.download_status,
            upload_counter: upload_counter.clone(),
            upload_status: opts.upload_status,
        };

        let app = Router::new()
            .route("/health", get(health))
            .route("/ping", get(ping_handler))
            .route("/download", get(download))
            .route("/upload", post(upload))
            .with_state(state);

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
            ping_counter,
            download_counter,
            upload_counter,
            shutdown_tx: Some(tx),
            handle,
        }
    }

    pub fn base_url(&self) -> Url {
        format!("http://{}/", self.addr).parse().unwrap()
    }

    /// Number of requests received at /ping since the server started.
    pub fn ping_count(&self) -> u64 {
        self.ping_counter.load(Ordering::Relaxed)
    }

    /// Number of requests received at /download since the server started.
    pub fn download_count(&self) -> u64 {
        self.download_counter.load(Ordering::Relaxed)
    }

    /// Number of requests received at /upload since the server started.
    pub fn upload_count(&self) -> u64 {
        self.upload_counter.load(Ordering::Relaxed)
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

async fn ping_handler(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    state.ping_counter.fetch_add(1, Ordering::Relaxed);
    if let Some(delay) = state.ping_delay {
        tokio::time::sleep(delay).await;
    }
    (state.ping_status, "")
}

#[derive(Deserialize)]
struct DownloadQuery {
    #[serde(default)]
    bytes: Option<u64>,
}

async fn download(State(state): State<AppState>, Query(q): Query<DownloadQuery>) -> Response {
    state.download_counter.fetch_add(1, Ordering::Relaxed);
    if !state.download_status.is_success() {
        return Response::builder()
            .status(state.download_status)
            .body(Body::empty())
            .unwrap();
    }

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

async fn upload(State(state): State<AppState>, body: Bytes) -> Response {
    state.upload_counter.fetch_add(1, Ordering::Relaxed);
    if !state.upload_status.is_success() {
        return Response::builder()
            .status(state.upload_status)
            .body(Body::empty())
            .unwrap();
    }
    Json(UploadResponse {
        received: body.len() as u64,
    })
    .into_response()
}
