//! The publishing workflows: dry run first, a live path only a protected
//! release takes, and what each verifies before handing a release on.

mod binary_attestation;
mod crate_and_binaries;
mod crate_toolchains;
mod evidence_publication;
mod github_releases;
mod payload_verification;
mod recorded_attestation;
