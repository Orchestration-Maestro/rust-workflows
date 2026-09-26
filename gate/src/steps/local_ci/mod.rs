//! `rust-gate ci --local`: the `checks` job of `ci.yml` on a developer's
//! machine. The step is `step.rs`; `job.rs` is the job as a local run takes
//! it, `checkout.rs` the commit it checks, `environment.rs` the environment
//! every step starts with and `cache.rs` what CI's cache keeps between runs.

mod cache;
mod checkout;
mod environment;
mod job;
mod step;

pub(super) use step::STEPS;
