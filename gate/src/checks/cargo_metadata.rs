//! What several steps read out of Cargo's own records: jaq programs over
//! `cargo metadata` and the build's JSON messages.

/// Binary name and executable path of every compiler artifact.
pub(crate) const EXECUTABLES: &str =
    ".[] | select(.reason == \"compiler-artifact\" and .executable != null) |
  [.target.name, .executable] | @tsv";
