#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Live integration tests against the real Cloudflare speed endpoint.
//! Gated on the `live` feature; skipped in default CI runs.
//! Run with: cargo test --features live

#[cfg(feature = "live")]
mod live {
    use rspeed::{CloudflareBackend, Config, Format, TestSession};
    use rspeed::config::IpVersion;
    use rspeed::ColorWhen;

    fn live_config() -> Config {
        Config {
            duration_secs: 5,
            connections: 4,
            server: None,
            do_download: true,
            do_upload: true,
            format: Format::Json,
            color: ColorWhen::Never,
            ip_version: IpVersion::Auto,
            verbose: 0,
        }
    }

    #[tokio::test]
    async fn cloudflare_download_and_upload_return_nonzero_bytes() {
        let backend = Box::new(CloudflareBackend::new().expect("CloudflareBackend::new failed"));
        let session = TestSession::new(backend, live_config());
        let result = session.run().await.expect("TestSession::run failed");

        let dl = result.download.expect("download phase missing");
        let ul = result.upload.expect("upload phase missing");

        assert!(dl.bytes > 0, "download bytes should be > 0, got {}", dl.bytes);
        assert!(ul.bytes > 0, "upload bytes should be > 0, got {}", ul.bytes);
        assert!(dl.mbps > 0.0, "download Mbps should be > 0, got {}", dl.mbps);
    }
}
