//! Backend selection from Config. See DEC-003.

use crate::config::Config;

use super::{Backend, CloudflareBackend, GenericHttpBackend};

/// Choose a backend based on the user's Config.
///
/// - `--server <url>` set → `GenericHttpBackend`
/// - otherwise → `CloudflareBackend` (default)
///
/// Returns `Box<dyn Backend + Send + Sync>` because STAGE-002 will
/// hold the backend across `tokio::spawn` task boundaries; the auto
/// trait bounds must be on the `dyn` type explicitly (supertrait
/// bounds on `Backend` don't propagate to trait objects automatically).
pub fn select(config: &Config) -> Box<dyn Backend + Send + Sync> {
    match &config.server {
        Some(url) => Box::new(GenericHttpBackend::new(url.clone())),
        None => Box::new(CloudflareBackend::default()),
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
        let backend = select(&cfg(None));
        assert_eq!(backend.name(), "cloudflare");
    }

    #[test]
    fn generic_when_server_set() {
        let url: Url = "https://example.com".parse().unwrap();
        let backend = select(&cfg(Some(url)));
        assert_eq!(backend.name(), "generic");
    }
}
