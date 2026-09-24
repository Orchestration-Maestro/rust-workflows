//! What the steps check and share, one module per concern so a call site
//! names the kind of rule it reaches for: paths in the checkout, simple
//! names, Rust versions, the release vocabulary, private directories,
//! Cargo's records, the typed `ci.yml` inputs, Rust source read into module
//! trees, `maestro-quality.toml` and the findings rules report. Built on the
//! runner, never on a step.

pub(crate) mod cargo_metadata;
pub(crate) mod checkout_paths;
pub(crate) mod findings;
pub(crate) mod inputs;
pub(crate) mod lint_policy;
pub(crate) mod manifests;
pub(crate) mod module_tree;
pub(crate) mod private_directories;
pub(crate) mod quality_config;
pub(crate) mod release_boundary;
pub(crate) mod rust_code;
mod rust_paths;
pub(crate) mod rust_tests;
pub(crate) mod rust_versions;
pub(crate) mod simple_names;
