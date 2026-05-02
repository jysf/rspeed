//! Shared latency probe helper. Both `CloudflareBackend` and
//! `GenericHttpBackend` delegate to `probe()` here.
//!
//! HTTP RTT measurement assumes reqwest's default HTTP/2 multiplex
//! (or HTTP/1.1 keep-alive) reuses the TCP+TLS connection across the
//! N+1 ping requests. Without pooling, every request would include
//! handshake overhead and the warm-up discard would be meaningless.
//! reqwest's default pool max is large enough for our N+1=11 case;
//! no `pool_max_idle_per_host` tuning needed.
//!
//! The `/ping` handler for `GenericHttpBackend` and `/__ping` for
//! `CloudflareBackend` serve only `/ping`; `/health`, `/download`, and
//! `/upload` handlers are unrelated to this module.

use std::time::Duration;

use tokio::time::Instant;
use url::Url;

use super::{BackendError, LatencyProbeOutcome};

/// Run the latency probe: HTTP RTT primary, TCP-connect fallback.
///
/// Issues `samples + 1` HTTP GET requests to `ping_url`, discards
/// the first (warm-up), and returns the remaining durations. On any
/// HTTP-layer failure (network error, timeout, or non-2xx status)
/// the probe restarts cleanly in TCP-connect mode against `tcp_target`.
pub(crate) async fn probe(
    client: &reqwest::Client,
    ping_url: &Url,
    tcp_target: &str,
    samples: usize,
    per_request_timeout: Duration,
) -> Result<LatencyProbeOutcome, BackendError> {
    match http_rtt_probe(client, ping_url, samples, per_request_timeout).await {
        Ok(s) => Ok(LatencyProbeOutcome::new("http_rtt", s)),
        Err(BackendError::Timeout(_) | BackendError::Network(_) | BackendError::Protocol(_)) => {
            // Discard all partial HTTP samples; restart cleanly in TCP mode
            // to avoid a mixed-method samples slice (DEC-006 requires a single method).
            let s = tcp_connect_probe(tcp_target, samples, per_request_timeout).await?;
            Ok(LatencyProbeOutcome::new("tcp_connect", s))
        }
        Err(other) => Err(other),
    }
}

async fn http_rtt_probe(
    client: &reqwest::Client,
    ping_url: &Url,
    samples: usize,
    per_request_timeout: Duration,
) -> Result<Vec<Duration>, BackendError> {
    let mut result = Vec::with_capacity(samples);
    for i in 0..=samples {
        let start = Instant::now();
        let req = client
            .get(ping_url.clone())
            .header("Accept-Encoding", "identity");
        let resp = tokio::time::timeout(per_request_timeout, req.send())
            .await
            .map_err(|_| BackendError::Timeout(per_request_timeout))?
            .map_err(BackendError::Network)?;

        if !resp.status().is_success() {
            return Err(BackendError::Protocol(format!(
                "ping returned status {}",
                resp.status()
            )));
        }
        // Drain body to release the connection back to the pool.
        resp.bytes().await.map_err(BackendError::Network)?;

        let elapsed = start.elapsed();
        if i > 0 {
            result.push(elapsed);
        }
    }
    Ok(result)
}

async fn tcp_connect_probe(
    addr: &str,
    samples: usize,
    per_connect_timeout: Duration,
) -> Result<Vec<Duration>, BackendError> {
    let mut result = Vec::with_capacity(samples);
    for i in 0..=samples {
        let start = Instant::now();
        let stream =
            tokio::time::timeout(per_connect_timeout, tokio::net::TcpStream::connect(addr))
                .await
                .map_err(|_| BackendError::Timeout(per_connect_timeout))?
                .map_err(|e| BackendError::Protocol(format!("tcp connect: {e}")))?;
        drop(stream);
        let elapsed = start.elapsed();
        if i > 0 {
            result.push(elapsed);
        }
    }
    Ok(result)
}
