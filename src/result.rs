//! Canonical result types per DEC-006: a single struct per output, three
//! renderers downstream. SPEC-007 defines the types and the
//! `compute_latency_result` helper; the orchestrator (SPEC-012) populates
//! `TestResult`. JSON output is exactly `TestResult`'s `Serialize` shape —
//! field renames are a public-API break.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize)]
pub struct TestResult {
    pub started_at: DateTime<Utc>,
    pub backend: String,
    pub server_url: String,
    pub ip_version: String,
    pub duration_secs: f64,
    pub latency: LatencyResult,
    pub download: Option<ThroughputResult>,
    pub upload: Option<ThroughputResult>,
}

#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize)]
pub struct LatencyResult {
    pub method: String,
    pub samples: usize,
    pub median_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub jitter_ms: f64,
}

#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize)]
pub struct ThroughputResult {
    pub mbps: f64,
    pub mbps_p50: f64,
    pub mbps_p95: f64,
    pub bytes: u64,
    pub connections_configured: usize,
    pub connections_active: usize,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub elapsed: Duration,
    pub phase: Phase,
    pub current_mbps: f64,
    pub bytes_so_far: u64,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            elapsed: Duration::ZERO,
            phase: Phase::Latency,
            current_mbps: 0.0,
            bytes_so_far: 0,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Phase {
    #[default]
    Latency,
    Download,
    Upload,
}

/// Reduce a slice of latency samples to the canonical `LatencyResult`.
///
/// Panics on an empty slice — a latency probe that returns zero samples is
/// a programming error upstream, not a runtime condition the orchestrator
/// should silently fold into a zeroed result.
pub fn compute_latency_result(method: &str, samples: &[Duration]) -> LatencyResult {
    assert!(
        !samples.is_empty(),
        "compute_latency_result called with empty samples — bug upstream"
    );
    let n = samples.len();

    let mut ms: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1_000.0).collect();
    ms.sort_by(|a, b| a.total_cmp(b));

    let min_ms = ms[0];
    let max_ms = ms[n - 1];
    let median_ms = ms[n / 2];

    let jitter_ms = if n == 1 {
        0.0
    } else {
        let mean = ms.iter().copied().sum::<f64>() / n as f64;
        let variance = ms.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0);
        variance.sqrt()
    };

    LatencyResult {
        method: method.to_string(),
        samples: n,
        median_ms,
        min_ms,
        max_ms,
        jitter_ms,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use chrono::TimeZone;

    #[test]
    fn compute_latency_result_basic() {
        let samples = [
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(300),
        ];
        let r = compute_latency_result("http_rtt", &samples);
        assert_eq!(r.method, "http_rtt");
        assert_eq!(r.samples, 3);
        assert!((r.median_ms - 200.0).abs() < 1e-9);
        assert!((r.min_ms - 100.0).abs() < 1e-9);
        assert!((r.max_ms - 300.0).abs() < 1e-9);
    }

    #[test]
    fn compute_latency_result_jitter() {
        let samples = [Duration::from_millis(150); 5];
        let r = compute_latency_result("http_rtt", &samples);
        assert!(r.jitter_ms.abs() < 1e-9);
    }

    #[test]
    fn test_result_serializes_to_json() {
        let started_at = Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap();
        let result = TestResult {
            started_at,
            backend: "cloudflare".to_string(),
            server_url: "https://speed.cloudflare.com/".to_string(),
            ip_version: "ipv4".to_string(),
            duration_secs: 8.0,
            latency: LatencyResult {
                method: "http_rtt".to_string(),
                samples: 5,
                median_ms: 12.5,
                min_ms: 10.0,
                max_ms: 15.0,
                jitter_ms: 1.2,
            },
            download: Some(ThroughputResult {
                mbps: 950.0,
                mbps_p50: 940.0,
                mbps_p95: 980.0,
                bytes: 1_200_000_000,
                connections_configured: 4,
                connections_active: 4,
            }),
            upload: Some(ThroughputResult {
                mbps: 200.0,
                mbps_p50: 195.0,
                mbps_p95: 220.0,
                bytes: 250_000_000,
                connections_configured: 4,
                connections_active: 4,
            }),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"download\""));
        assert!(json.contains("\"upload\""));
        assert!(json.contains("\"latency\""));
        assert!(json.contains("\"started_at\""));
    }

    #[test]
    fn snapshot_default_is_all_zero() {
        let s = Snapshot::default();
        assert_eq!(s.elapsed, Duration::ZERO);
        assert_eq!(s.phase, Phase::Latency);
        assert!(s.current_mbps.abs() < f64::EPSILON);
        assert_eq!(s.bytes_so_far, 0);
    }
}
