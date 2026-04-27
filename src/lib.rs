//! rspeed library API. The binary at `src/main.rs` is a thin shim
//! over `run()`. STAGE-002 specs add real measurement code here.

use clap::Parser;

mod cli;
pub mod config;
pub use config::{ColorWhen, Config, Format};

/// Entry point invoked by `main`. Returns a process exit code.
pub fn run() -> anyhow::Result<i32> {
    let cli = cli::Cli::parse();
    let config = Config::from(cli);
    println!("{config:#?}");
    Ok(0)
}
