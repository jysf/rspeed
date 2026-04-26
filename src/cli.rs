//! Command-line interface parsing. Private to the binary; library
//! consumers use Config directly without going through clap.

use clap::{ArgAction, Parser, ValueEnum};
use url::Url;

#[derive(Debug, Parser)]
#[command(name = "rspeed", version, about)]
pub struct Cli {
    /// Test duration in seconds.
    #[arg(short = 'd', long, default_value_t = 10)]
    pub duration: u32,

    /// Parallel connections (1..=32).
    #[arg(short = 'c', long, default_value_t = 4,
          value_parser = clap::value_parser!(u8).range(1..=32))]
    pub connections: u8,

    /// Custom server URL. Defaults to Cloudflare when unset.
    #[arg(short = 's', long)]
    pub server: Option<Url>,

    /// Skip the upload phase.
    #[arg(long, conflicts_with = "no_download")]
    pub no_upload: bool,

    /// Skip the download phase.
    #[arg(long, conflicts_with = "no_upload")]
    pub no_download: bool,

    /// Output format.
    #[arg(short = 'f', long, value_enum, default_value_t = Format::Human)]
    pub format: Format,

    /// Color output mode.
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto)]
    pub color: ColorWhen,

    /// Force IPv4.
    #[arg(short = '4', long, conflicts_with = "ipv6")]
    pub ipv4: bool,

    /// Force IPv6.
    #[arg(short = '6', long, conflicts_with = "ipv4")]
    pub ipv6: bool,

    /// Increase verbosity (-v, -vv, -vvv).
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Human,
    Json,
    Silent,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}
