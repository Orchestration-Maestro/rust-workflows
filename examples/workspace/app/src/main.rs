//! Workspace consumer fixture that prints the core library's result.

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let total = maestro_workspace_arithmetic::checked_sum(20, 22);
    if let Some(total) = total {
        println!("{total}");
    }
    ExitCode::from(u8::from(total.is_none()))
}
