use std::process::ExitCode;

fn main() -> ExitCode {
    match rspeed::run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}
