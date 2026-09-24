//! Contract tests for the reusable workflows: one directory per what the tests
//! prove, `ci`, `publishers`, `nightly`, `gate` and `repository`, and the
//! harness they share, the one door to the workflows, the gate and the fixture.

#![cfg(test)]
#![forbid(unsafe_code)]

mod ci;
mod gate;
mod harness;
mod nightly;
mod publishers;
mod repository;
