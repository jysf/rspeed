//! Resolved, downstream-facing configuration. The rest of rspeed
//! consumes Config; clap-shaped Cli stays out of the way.

use url::Url;

use crate::cli;

// Re-export the CLI value enums so library consumers don't need to
// import from `cli` directly (which is private to the binary anyway).
pub use crate::cli::{ColorWhen, Format};

#[derive(Debug)]
pub struct Config {
    pub duration_secs: u32,
    pub connections: u8,
    pub server: Option<Url>,
    pub do_upload: bool,
    pub do_download: bool,
    pub format: Format,
    pub color: ColorWhen,
    pub ip_version: IpVersion,
    /// Verbosity count from `-v` / `-vv` / `-vvv`. 0 = Warn, 1 = Info,
    /// 2 = Debug, 3+ = Trace. Mapping to a logger happens when logging
    /// integration lands (deferred — not in MVP's STAGE-001).
    pub verbose: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum IpVersion {
    Auto,
    V4,
    V6,
}

impl Config {
    pub fn validate(&self) -> Result<(), crate::error::TestError> {
        if let Some(url) = &self.server
            && !url.path().ends_with('/')
        {
            return Err(crate::error::TestError::Config(format!(
                "--server URL must end with a trailing slash (got: {url})"
            )));
        }
        Ok(())
    }

    pub(crate) fn server_url_string(&self) -> String {
        self.server
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_else(|| "https://speed.cloudflare.com/".to_string())
    }

    /// Reflects user *intent*, not observed wire behaviour.
    /// STAGE-004 may refine this via reqwest::local_addr() to
    /// report the family actually used by the connection pool.
    pub(crate) fn ip_version_string(&self) -> String {
        match self.ip_version {
            IpVersion::Auto => "auto",
            IpVersion::V4 => "ipv4",
            IpVersion::V6 => "ipv6",
        }
        .to_string()
    }
}

impl From<cli::Cli> for Config {
    fn from(c: cli::Cli) -> Self {
        let ip_version = match (c.ipv4, c.ipv6) {
            (true, false) => IpVersion::V4,
            (false, true) => IpVersion::V6,
            _ => IpVersion::Auto,
        };
        Self {
            duration_secs: c.duration,
            connections: c.connections,
            server: c.server,
            do_upload: !c.no_upload,
            do_download: !c.no_download,
            format: c.format,
            color: c.color,
            ip_version,
            verbose: c.verbose,
        }
    }
}
