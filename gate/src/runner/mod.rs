//! The runner as the gate sees it: inputs and the `GITHUB_*` files, the
//! job's directories, and tools run with their own exit status. Standard
//! library only, and nothing above it.

mod commands;
mod github_actions;
mod outcome;
mod step_declaration;

pub(crate) use commands::{Cmd, non_empty, tee_line, write};
pub(crate) use github_actions::{
    Job, add_to_path, export, flag, input, native_linux, optional, output, path, summary,
};
pub(crate) use outcome::{Failure, Outcome};
pub(crate) use step_declaration::{Step, enter};
