//! `rust-gate hygiene`: what every tracked file of the repository holds to,
//! whatever its language. The step is `step.rs`; each other module holds one
//! group of rules.

mod comments;
mod files;
mod step;
mod widths;

pub(super) use step::STEPS;
