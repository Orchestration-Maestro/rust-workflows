//! The gate: every step body of the reusable workflows as one binary, built
//! from the pinned commit of this repository and called by subcommand. Inputs
//! arrive as environment variables, exactly as the step bodies received them
//! through `env:`. A failure ends the step with the failing tool's own exit
//! status, or with one message on stderr and status 1.

#![forbid(unsafe_code)]
#![deny(clippy::missing_docs_in_private_items)]

use std::env;
use std::process;

mod checks;
mod runner;
mod steps;

fn main() {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_default();
    if command == "describe" {
        print!("{}", steps::describe());
        return;
    }
    let step = arguments.next().unwrap_or_default();
    if let Err(failure) = steps::run(&command, &step) {
        if let Some(message) = failure.message {
            eprintln!("{message}");
        }
        process::exit(failure.code);
    }
}
