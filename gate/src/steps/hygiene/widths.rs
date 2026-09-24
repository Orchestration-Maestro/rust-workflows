//! SIZE-003 for what no module tree reaches: every line of a shell script or
//! a justfile holds the organization's width, like every line of Rust.

use crate::checks::findings::Finding;
use crate::checks::quality_config::Limits;
use std::path::Path;

/// SIZE-003 over the shell scripts and justfiles among `files`.
pub(super) fn findings(workspace: &Path, files: &[String], limits: Limits) -> Vec<Finding> {
    let mut found = Vec::new();
    for file in files.iter().filter(|file| shell_like(file)) {
        let Ok(text) = std::fs::read_to_string(workspace.join(file)) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            let columns = line.chars().count();
            if columns > limits.line_columns {
                let message = format!(
                    "{columns} columns, over {}; wrap it, strings and comments included",
                    limits.line_columns
                );
                found.push(Finding::new("SIZE-003", file.clone(), number + 1, message));
            }
        }
    }
    found
}

/// Whether a file is a shell script or a justfile.
fn shell_like(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or_default();
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    name.eq_ignore_ascii_case("justfile") || matches!(extension.as_deref(), Some("sh" | "bash"))
}

#[cfg(test)]
mod tests {
    use super::shell_like;

    #[test]
    fn shell_scripts_and_justfiles_are_measured_and_nothing_else() {
        assert!(shell_like("scripts/bootstrap.sh") && shell_like("justfile"));
        assert!(shell_like("tools/run.bash") && shell_like("sub/Justfile"));
        assert!(!shell_like("src/lib.rs") && !shell_like("README.md"));
    }
}
