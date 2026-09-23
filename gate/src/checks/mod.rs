//! What the steps check and share, one module per concern so a call site
//! names the kind of rule it reaches for: paths in the checkout, simple
//! names, Rust versions, the release vocabulary, private directories,
//! Cargo's records and the typed `ci.yml` inputs. Built on the runner,
//! never on a step.

pub(crate) mod cargo_metadata;
pub(crate) mod checkout_paths;
pub(crate) mod inputs;
pub(crate) mod private_directories;
pub(crate) mod release_boundary;
pub(crate) mod rust_versions;
pub(crate) mod simple_names;
