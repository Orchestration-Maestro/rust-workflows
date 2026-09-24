//! Workspace consumer fixture that prints the core library's result.

#![forbid(unsafe_code)]

use std::process;

fn main() {
    match maestro_workspace_arithmetic::checked_sum(20, 22) {
        Some(total) => println!("{total}"),
        None => process::exit(1),
    }
}
