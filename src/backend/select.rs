//! Backend selection from Config. See DEC-003.

use crate::config::Config;

use super::{Backend, BackendError, CloudflareBackend, GenericHttpBackend};

/// Choose a backend based on the user's Config.
///
/// - `--server <url>` set → `GenericHttpBackend`
/// - otherwise → `CloudflareBackend` (default)
///
/// Returns `Err(BackendError)` if client construction fails (e.g. TLS
/// init failure). The cascade through `lib::run()` is one `?` operator;
/// `anyhow::Result<i32>` wraps `BackendError` via its `std::error::Error`
/// impl.
pub fn select(config: &Config) -> Result<Box<dyn Backend + Send + Sync>, BackendError> {
    match &config.server {
        Some(url) => Ok(Box::new(GenericHttpBackend::new(url.clone())?)),
        None => Ok(Box::new(CloudflareBackend::new()?)),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // test literals are known-valid; panicking is correct behavior here
mod tests {
    use super::*;
    use crate::config::{ColorWhen, Config, Format, IpVersion};
    use url::Url;

    fn cfg(server: Option<Url>) -> Config {
        Config {
            duration_secs: 10,
            connections: 4,
            server,
            do_upload: true,
            do_download: true,
            format: Format::Human,
            color: ColorWhen::Auto,
            ip_version: IpVersion::Auto,
            verbose: 0,
        }
    }

    #[test]
    fn cloudflare_when_no_server() {
        let backend = select(&cfg(None)).unwrap();
        assert_eq!(backend.name(), "cloudflare");
    }

    #[test]
    fn generic_when_server_set() {
        let url: Url = "https://example.com".parse().unwrap();
        let backend = select(&cfg(Some(url))).unwrap();
        assert_eq!(backend.name(), "generic");
    }
}
