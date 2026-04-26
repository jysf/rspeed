//! rspeed library API. The binary at `src/main.rs` is a thin shim
//! over `run()`. STAGE-002 specs add real measurement code here.

/// Entry point invoked by `main`. Returns a process exit code.
pub fn run() -> anyhow::Result<i32> {
    println!("rspeed v{}", env!("CARGO_PKG_VERSION"));
    Ok(0)
}
