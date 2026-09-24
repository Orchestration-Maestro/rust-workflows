//! How a step ends: complete, or failed with the exit status of the tool
//! that failed, or with one actionable message and status 1.

use std::io;
use std::io::ErrorKind;

/// How a step fails: with the exit status of the tool that failed and no
/// message of its own, since the tool already printed one, or with one
/// actionable message and status 1.
#[derive(Debug)]
pub(crate) struct Failure {
    /// The process exit status the step ends with.
    pub(crate) code: i32,
    /// What to print on stderr, when the tool did not already say it.
    pub(crate) message: Option<String>,
}

impl Failure {
    /// A tool that ran and failed: its status is the step's.
    pub(crate) fn status(code: i32) -> Self {
        Self {
            code,
            message: None,
        }
    }

    /// A tool that could not be started: 127 when it is missing, the way a
    /// shell reports `command not found`.
    pub(crate) fn spawn(command: &str, error: &io::Error) -> Self {
        let code = if error.kind() == ErrorKind::NotFound {
            127
        } else {
            1
        };
        Self {
            code,
            message: Some(format!("cannot run {command}: {error}")),
        }
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self {
            code: 1,
            message: Some(message),
        }
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Self::from(message.to_owned())
    }
}

/// A step either completes or fails.
pub(crate) type Outcome = Result<(), Failure>;
