//! `rust-gate architecture`: the source rules every organization repository
//! holds to, read from its source and its manifests alone. The
//! step is `step.rs`; each other module holds one group of rules.

mod cycles;
mod doors;
mod layers;
mod lints;
mod names;
mod packages;
mod roots;
mod seams;
mod sizes;
mod sources;
mod step;

pub(super) use step::STEPS;
