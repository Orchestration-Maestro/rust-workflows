//! What both publishers ask of a release before anything is published.

use super::simple_names::is_hex;
use crate::runner::{Cmd, Outcome, flag, input, summary};
use std::fs;
use std::path::Path;

/// The checks every live publication shares: a push or a dispatch, on a
/// `vMAJOR.MINOR.PATCH` tag a ruleset protects.
pub(crate) fn is_protected_release() -> Outcome {
    let event = input("EVENT")?;
    if event != "push" && event != "workflow_dispatch" {
        return Err("Live publication requires push or workflow_dispatch".into());
    }
    if !is_release_tag(&input("REF")?) {
        return Err("Use a vMAJOR.MINOR.PATCH release tag".into());
    }
    if input("PROTECTED")? != "true" {
        return Err("Release tag must be protected by a ruleset".into());
    }
    Ok(())
}

/// Query GitHub rather than treating an environment name as approval. Missing
/// permissions, unavailable reviewers on private Team repositories and malformed
/// responses all fail closed. Dry-run callers never enter this function.
pub(crate) fn is_approved_environment() -> Outcome {
    let repository = input("GITHUB_REPOSITORY")?;
    let endpoint = format!("repos/{repository}/environments/release");
    let environment = Cmd::new("gh api")
        .arg(&endpoint)
        .args(["--hostname", "github.com"])
        .capture()
        .map_err(|_| "Cannot verify release environment protection")?;
    require_json(
        &environment,
        r#"
        .deployment_branch_policy.protected_branches == false and
        .deployment_branch_policy.custom_branch_policies == true and
        (.protection_rules | type == "array") and
        any(.protection_rules[];
            .type == "required_reviewers" and
            (.reviewers | type == "array" and length > 0) and
            all(.reviewers[];
                (.type == "User" or .type == "Team") and
                (.reviewer.id | type == "number" and . > 0)))
    "#,
        "release environment must require reviewers and custom tag policies",
    )?;
    let policies = Cmd::new("gh api")
        .arg(format!(
            "{endpoint}/deployment-branch-policies?per_page=100"
        ))
        .args(["--hostname", "github.com"])
        .capture()
        .map_err(|_| "Cannot verify release environment tag policies")?;
    require_json(
        &policies,
        r#"
        .total_count == 1 and (.branch_policies | type == "array" and length == 1) and
        .branch_policies[0].type == "tag" and .branch_policies[0].name == "v*"
    "#,
        "release environment must allow only the v* tag policy",
    )
}

/// Check all local assets and remote identities before adding anything to an
/// existing release. Never create a tag/release or overwrite an existing asset.
/// GitHub uploads are not atomic; an error can leave earlier assets uploaded.
pub(crate) fn upload_release(files: &[&str]) -> Outcome {
    if flag("DRY_RUN")? {
        return Ok(());
    }
    let revision = input("REVISION")?;
    if !is_hex(&revision, 40) || revision != input("GITHUB_SHA")? {
        return Err("Publication revision must match this source".into());
    }
    for file in files {
        if !fs::symlink_metadata(file).is_ok_and(|meta| meta.is_file() && meta.len() > 0) {
            return Err("Release asset is missing or unsafe".into());
        }
    }
    is_protected_release()?;
    is_approved_environment()?;
    let repository = input("GITHUB_REPOSITORY")?;
    let reference = input("REF")?;
    let tag = reference.trim_start_matches("refs/tags/");
    let commit = Cmd::new("gh api")
        .arg(format!("repos/{repository}/commits/refs%2Ftags%2F{tag}"))
        .args(["--hostname", "github.com"])
        .capture()
        .map_err(|_| "Cannot resolve the release tag revision")?;
    require_json(
        &commit,
        &format!(".sha == \"{revision}\""),
        "Release tag does not match the validated revision",
    )?;
    let release = Cmd::new("gh release view")
        .arg(tag)
        .args([
            "--repo",
            &format!("github.com/{repository}"),
            "--json",
            "tagName,assets",
        ])
        .capture()
        .map_err(|_| "Release must already exist for this tag")?;
    let names = files
        .iter()
        .filter_map(|file| Path::new(file).file_name())
        .map(|name| format!("\"{}\"", name.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(",");
    require_json(
        &release,
        &format!(
            r#"
        .tagName == "{tag}" and (.assets | type == "array") and
        all(.assets[]; (.name | type == "string") and (.name as $name |
            [{names}] | index($name) == null))
    "#
        ),
        "Release must exist for this tag with no matching assets",
    )?;
    Cmd::new("gh release upload")
        .arg(tag)
        .args(["--repo", &format!("github.com/{repository}")])
        .args(files.iter().copied())
        .run()?;
    summary(&format!(
        "## Release assets published\n\nRevision: {revision}\nTag: {tag}\n"
    ))
}

/// A single true JSON result is required, not merely successful parsing.
fn require_json(json: &str, filter: &str, message: &str) -> Outcome {
    let result = Cmd::new("jaq -e")
        .arg(filter)
        .stdin_bytes(json.as_bytes())
        .capture()
        .map_err(|_| message)?;
    if result.trim() != "true" {
        return Err(message.into());
    }
    Ok(())
}

/// `refs/tags/vMAJOR.MINOR.PATCH`, with an optional dot- or hyphen-separated
/// alphanumeric pre-release suffix.
fn is_release_tag(reference: &str) -> bool {
    let Some(rest) = reference.strip_prefix("refs/tags/v") else {
        return false;
    };
    let (version, suffix) = rest.split_once('-').unwrap_or((rest, ""));
    let numeric = version.split('.').collect::<Vec<_>>();
    let core = numeric.len() == 3
        && numeric
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    let suffix_ok = rest.split_once('-').is_none()
        || suffix.split(['.', '-']).all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        });
    core && suffix_ok
}

#[cfg(test)]
mod tests {
    use super::is_release_tag;

    #[test]
    fn release_tags_are_semantic_versions() {
        for good in [
            "refs/tags/v1.2.3",
            "refs/tags/v0.1.0-rc.1",
            "refs/tags/v1.0.0-beta-2",
        ] {
            assert!(is_release_tag(good), "{good}");
        }
        for bad in [
            "refs/tags/1.2.3",
            "refs/heads/v1.2.3",
            "refs/tags/v1.2",
            "refs/tags/v1.2.3-",
            "refs/tags/v1.2.3-rc..1",
        ] {
            assert!(!is_release_tag(bad), "{bad}");
        }
    }
}
