#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for `MetricsAccumulator` (SPEC-007).
//!
//! Timing-sensitive tests use `#[tokio::test(start_paused = true)]` so
//! `tokio::time::advance` deterministically drives the ticker without
//! scheduler jitter.

use std::time::Duration;

use rspeed::{MetricsAccumulator, Phase};

#[tokio::test]
async fn snapshot_starts_in_latency_phase() {
    let acc = MetricsAccumulator::new(Duration::from_millis(100), Duration::from_secs(2));
    let rx = acc.subscribe();
    let snap = rx.borrow().clone();
    assert_eq!(snap.phase, Phase::Latency);
    assert_eq!(snap.bytes_so_far, 0);
}

#[tokio::test(start_paused = true)]
async fn record_bytes_increments_bytes_so_far() {
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_secs(0));
    let mut rx = acc.subscribe();
    acc.record_bytes(1024);
    let _h = acc.start_ticking();

    tokio::time::advance(Duration::from_millis(50)).await;
    rx.changed().await.unwrap();
    let snap = rx.borrow().clone();
    assert!(snap.bytes_so_far >= 1024, "got {}", snap.bytes_so_far);
}

#[tokio::test(start_paused = true)]
async fn snapshot_emitted_on_interval() {
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_secs(0));
    let mut rx = acc.subscribe();
    let _h = acc.start_ticking();

    let mut count = 0;
    for _ in 0..4 {
        tokio::time::advance(Duration::from_millis(50)).await;
        if rx.changed().await.is_ok() {
            count += 1;
        }
    }
    assert!(count >= 3, "expected ≥ 3 snapshots, got {count}");
}

#[tokio::test(start_paused = true)]
async fn current_mbps_reflects_last_interval_only() {
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_secs(0));
    let mut rx = acc.subscribe();
    let _h = acc.start_ticking();

    acc.record_bytes(1_000_000);
    tokio::time::advance(Duration::from_millis(50)).await;
    rx.changed().await.unwrap();
    let snap1 = rx.borrow().clone();
    assert!(
        snap1.current_mbps > 0.0,
        "first current_mbps was {}",
        snap1.current_mbps
    );

    tokio::time::advance(Duration::from_millis(50)).await;
    rx.changed().await.unwrap();
    let snap2 = rx.borrow().clone();
    assert!(
        snap2.current_mbps.abs() < f64::EPSILON,
        "second current_mbps was {}",
        snap2.current_mbps
    );
}

#[tokio::test(start_paused = true)]
async fn warmup_bytes_excluded_from_finish() {
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_millis(200));
    let _rx = acc.subscribe();
    let _h = acc.start_ticking();

    acc.record_bytes(1_000_000);
    tokio::time::advance(Duration::from_millis(250)).await;

    acc.record_bytes(500_000);
    tokio::time::advance(Duration::from_millis(50)).await;
    tokio::task::yield_now().await;

    let result = acc.finish(1, 1);
    assert_eq!(
        result.bytes, 500_000,
        "expected 500_000 post-warmup bytes, got {}",
        result.bytes
    );
}

#[tokio::test(start_paused = true)]
async fn finish_computes_mean_and_percentiles() {
    // Per-interval bytes chosen so current_mbps lands on [100, 200, 300, 400, 500].
    // bytes = mbps * interval_secs * 1e6 / 8, with interval = 50ms.
    let bytes_per_round: [u64; 5] = [625_000, 1_250_000, 1_875_000, 2_500_000, 3_125_000];

    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_millis(0));
    let mut rx = acc.subscribe();
    let _h = acc.start_ticking();

    for &b in &bytes_per_round {
        acc.record_bytes(b);
        tokio::time::advance(Duration::from_millis(50)).await;
        rx.changed().await.unwrap();
    }

    let result = acc.finish(1, 1);
    let eps = 0.01;
    assert!(
        (result.mbps - 300.0).abs() < eps,
        "mbps was {}",
        result.mbps
    );
    assert!(
        (result.mbps_p50 - 300.0).abs() < eps,
        "mbps_p50 was {}",
        result.mbps_p50
    );
    assert!(
        (result.mbps_p95 - 500.0).abs() < eps,
        "mbps_p95 was {}",
        result.mbps_p95
    );
}

#[tokio::test(start_paused = true)]
async fn abort_stops_ticking() {
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_secs(0));
    let mut rx = acc.subscribe();
    let h = acc.start_ticking();

    tokio::time::advance(Duration::from_millis(50)).await;
    rx.changed().await.unwrap();

    h.abort();
    tokio::task::yield_now().await;

    tokio::time::advance(Duration::from_millis(150)).await;
    tokio::task::yield_now().await;

    let res = tokio::time::timeout(Duration::ZERO, rx.changed()).await;
    assert!(
        res.is_err(),
        "expected no further snapshots after abort, got {res:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn multiple_subscribers_receive_same_snapshot() {
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_secs(0));
    let mut rx1 = acc.subscribe();
    let mut rx2 = acc.subscribe();
    acc.record_bytes(2048);
    let _h = acc.start_ticking();

    tokio::time::advance(Duration::from_millis(50)).await;
    rx1.changed().await.unwrap();
    rx2.changed().await.unwrap();

    let s1 = rx1.borrow().clone();
    let s2 = rx2.borrow().clone();
    assert_eq!(s1.bytes_so_far, s2.bytes_so_far);
    assert!((s1.current_mbps - s2.current_mbps).abs() < f64::EPSILON);
}

#[tokio::test(start_paused = true)]
async fn set_phase_visible_in_next_snapshot() {
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_secs(0));
    let mut rx = acc.subscribe();
    let _h = acc.start_ticking();
    acc.set_phase(Phase::Download);

    tokio::time::advance(Duration::from_millis(50)).await;
    rx.changed().await.unwrap();
    let snap = rx.borrow().clone();
    assert_eq!(snap.phase, Phase::Download);
}

#[tokio::test(start_paused = true)]
async fn clone_shares_state() {
    let acc = MetricsAccumulator::new(Duration::from_millis(50), Duration::from_secs(0));
    let mut rx = acc.subscribe();

    let acc2 = acc.clone();
    acc2.record_bytes(9999);

    let _h = acc.start_ticking();
    tokio::time::advance(Duration::from_millis(50)).await;
    rx.changed().await.unwrap();

    let snap = rx.borrow().clone();
    assert!(snap.bytes_so_far >= 9999, "got {}", snap.bytes_so_far);
}
