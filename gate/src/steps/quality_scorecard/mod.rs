//! `rust-gate scorecard`: what this run actually enforced, as data, prose and
//! a self-contained badge. The step lives in `step.rs`, the value it renders
//! in `scorecard.rs`.

mod scorecard;
mod step;

pub(super) use step::STEPS;
