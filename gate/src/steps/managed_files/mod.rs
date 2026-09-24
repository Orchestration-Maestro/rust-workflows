//! `rust-gate sync`, `sync --check`, `init` and the `managed-files` step:
//! the files every organization repository holds exactly as the gate renders
//! them. The steps are `step.rs`; `render.rs` renders each file, `hooks.rs`
//! the commit hooks, and `pin.rs` reads the release a caller pins.

mod hooks;
mod pin;
mod render;
mod step;

pub(super) use step::STEPS;
