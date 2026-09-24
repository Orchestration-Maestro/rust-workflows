//! `rust-gate architecture`: the module structure every organization
//! repository holds to, ARC-001 to ARC-007, read from its source alone. The
//! step is `step.rs`; each other module holds one group of rules.

mod cycles;
mod doors;
mod layers;
mod roots;
mod seams;
mod step;

pub(super) use step::STEPS;
