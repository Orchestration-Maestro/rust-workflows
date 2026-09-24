//! Workspace consumer fixture that prints the core library's result.

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match maestro_workspace_arithmetic::checked_sum(20, 22) {
        Some(total) => {
            println!("{total}");
            ExitCode::SUCCESS
        }
        None => ExitCode::FAILURE,
    }
}
