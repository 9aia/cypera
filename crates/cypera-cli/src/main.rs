//! `cypera` binary — thin shim over the engine lib.

use std::process::ExitCode;

fn main() -> ExitCode {
    match cypera::run_from(std::env::args_os()) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("cypera: {e:#}");
            ExitCode::FAILURE
        }
    }
}
