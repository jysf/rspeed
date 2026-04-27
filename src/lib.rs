//! rspeed library API.

use clap::Parser;

pub mod backend;
mod cli;
pub mod config;
pub use backend::{
    Backend, BackendError, CloudflareBackend, DownloadOpts, DownloadStream, GenericHttpBackend,
    UploadOpts, UploadResult,
};
pub use config::{ColorWhen, Config, Format};

/// Entry point invoked by `main`. Returns a process exit code.
pub fn run() -> anyhow::Result<i32> {
    let cli = cli::Cli::parse();
    let config = Config::from(cli);
    let backend = backend::select(&config);
    println!("{config:#?}");
    println!("Backend: {}", backend.name());
    Ok(0)
}
