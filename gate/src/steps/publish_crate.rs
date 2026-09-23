//! `rust-gate publish-crate <step>`: the crate publisher's steps, from the
//! publication boundary to `cargo publish` explicitly against public crates.io.

use crate::checks::checkout_paths::canonical;
use crate::checks::release_boundary::{is_approved_environment, is_protected_release};
use crate::checks::rust_versions::{channel_value, is_exact_stable};
use crate::checks::simple_names::is_hex;
use crate::runner::{Cmd, Outcome, Step, export, flag, input, output, path};
use std::path::Path;

/// What each step declares: its inputs, its tools and its reports.
pub(crate) const STEPS: &[Step] = &[
    Step {
        workflow: "publish-crate",
        id: "authorize",
        summary: "Validate publication boundary",
        inputs: &[
            "DRY_RUN",
            "EVENT",
            "GITHUB_REPOSITORY",
            "PACKAGE",
            "PROTECTED",
            "REF",
        ],
        tools: &["gh", "jaq"],
        reports: &[],
        run: authorize,
    },
    Step {
        workflow: "publish-crate",
        id: "toolchain",
        summary: "Set up validated consumer toolchain",
        inputs: &["DIRECTORY", "GITHUB_WORKSPACE"],
        tools: &["rustup"],
        reports: &[],
        run: toolchain,
    },
    Step {
        workflow: "publish-crate",
        id: "package",
        summary: "Verify selected Cargo package",
        inputs: &["DRY_RUN", "GITHUB_WORKSPACE", "PACKAGE", "REF"],
        tools: &["cargo metadata", "cargo package", "jaq"],
        reports: &[],
        run: package,
    },
    Step {
        workflow: "publish-crate",
        id: "semver",
        summary: "Semantic-version compatibility",
        inputs: &["PACKAGE", "SEMVER_CHECK"],
        tools: &["cargo semver-checks"],
        reports: &[],
        run: semver,
    },
    Step {
        workflow: "publish-crate",
        id: "publish-toolchain",
        summary: "Set up validated consumer toolchain",
        inputs: &["DIRECTORY", "GITHUB_WORKSPACE", "TOOLCHAIN"],
        tools: &["rustup"],
        reports: &[],
        run: publish_toolchain,
    },
    Step {
        workflow: "publish-crate",
        id: "publish",
        summary: "Publish explicitly to public crates.io",
        inputs: &[
            "DRY_RUN",
            "EVENT",
            "GITHUB_REPOSITORY",
            "GITHUB_SHA",
            "PACKAGE",
            "PROTECTED",
            "REF",
            "REVISION",
            "TOKEN",
        ],
        tools: &["cargo publish", "gh", "jaq"],
        reports: &[],
        run: publish,
    },
];

/// The publication boundary: the trusted event, ref and configuration a live
/// run needs, and nothing at all for a dry run.
fn authorize() -> Outcome {
    let package = input("PACKAGE")?;
    let safe = package.len() <= 64
        && package
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        && package
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !safe {
        return Err("package must be a safe exact Cargo package name".into());
    }
    if flag("DRY_RUN")? {
        return Ok(());
    }
    is_protected_release()?;
    is_approved_environment()
}

/// Installs one toolchain inside the consumer's checkout and exports the pair
/// the rest of the job reads: where the project is, and the channel rustup
/// must use. The two callers differ only in where the version comes from, so
/// the install and the export live here once.
fn install_and_export(project: &Path, version: &str) -> Outcome {
    Cmd::new("rustup toolchain install")
        .arg(version)
        .args(["--profile", "minimal"])
        .cwd(project)
        .run()?;
    export(&[
        ("PROJECT", &project.display().to_string()),
        ("RUSTUP_TOOLCHAIN", version),
    ])
}

/// The consumer's exact toolchain, read from the one key the CI job already
/// validated rather than through a TOML parser.
fn toolchain() -> Outcome {
    let project = path("GITHUB_WORKSPACE")?.join(input("DIRECTORY")?);
    let file = std::fs::read_to_string(project.join("rust-toolchain.toml"))
        .map_err(|error| format!("rust-toolchain.toml: {error}"))?;
    let pinned = file.lines().find_map(channel_value).unwrap_or_default();
    if !is_exact_stable(&pinned) {
        return Err("rust-toolchain.toml must pin an exact stable version".into());
    }
    install_and_export(&project, &pinned)?;
    output("channel", &pinned)
}

/// The selected package must be exactly one workspace member inside the
/// checkout; a live publication also needs the tag to equal its version and
/// the manifest to permit public crates.io.
fn package() -> Outcome {
    let project = path("PROJECT")?;
    let temp = path("RUNNER_TEMP")?;
    let package = input("PACKAGE")?;
    let metadata = temp.join("package-metadata.json");
    Cmd::new("cargo metadata --format-version 1 --no-deps --locked")
        .cwd(&project)
        .stdout_to(&metadata)?;
    let selected = temp.join("selected-package.json");
    Cmd::new("jaq -e --arg package")
        .arg(&package)
        .arg(
            "
    .workspace_members as $members |
  [.packages[] | select(.name == $package and (.id as $id | $members | index($id)))] |
    if length == 1 then .[0] else error(\"package must select exactly one workspace member\") end
  ",
        )
        .arg(&metadata)
        .stdout_to(&selected)?;
    let manifest = Cmd::new("jaq -er .manifest_path")
        .arg(&selected)
        .capture()?;
    let manifest = canonical(Path::new(manifest.trim()))?;
    let root = canonical(&path("GITHUB_WORKSPACE")?)?;
    if !manifest.starts_with(&root) || manifest == root {
        return Err("Selected package escapes checkout".into());
    }
    // The same accessor the authorize step uses: a value that is neither true
    // nor false is refused here too, rather than read as anything but "false"
    // and quietly skipping the two checks below.
    if !flag("DRY_RUN")? {
        let version = Cmd::new("jaq -er .version").arg(&selected).capture()?;
        if input("REF")? != format!("refs/tags/v{}", version.trim()) {
            return Err("Release tag must equal selected package version".into());
        }
        Cmd::new("jaq -e")
            .arg(".publish == null or (.publish | index(\"crates-io\") != null)")
            .arg(&selected)
            .capture()
            .map_err(|_| "Package does not permit crates.io publication")?;
    }
    Cmd::new("cargo package --package")
        .arg(&package)
        .arg("--locked")
        .cwd(&project)
        .run()
}

/// Compares the selected package against its newest published release. A
/// crate's first publication has no baseline, so this stays opt-in until one
/// version exists on crates.io.
fn semver() -> Outcome {
    if !flag("SEMVER_CHECK")? {
        println!("SKIPPED: semver-check=false");
        return Ok(());
    }
    Cmd::new("cargo semver-checks check-release --package")
        .arg(input("PACKAGE")?)
        .cwd(&path("PROJECT")?)
        .run()
}

/// The publish job installs the channel the staging job read and validated
/// rather than reading the file again, and checks it again: it arrived as a
/// job output, not from the file.
fn publish_toolchain() -> Outcome {
    let toolchain = input("TOOLCHAIN")?;
    if !is_exact_stable(&toolchain) {
        return Err("TOOLCHAIN must be an exact stable version".into());
    }
    let project = path("GITHUB_WORKSPACE")?.join(input("DIRECTORY")?);
    install_and_export(&project, &toolchain)
}

/// Native Cargo repackages; verification already ran for this exact source
/// and package. The registry token reaches Cargo through its own variables
/// and nowhere else.
fn publish() -> Outcome {
    if flag("DRY_RUN")? {
        return Ok(());
    }
    let token = input("TOKEN")?;
    if token.is_empty() {
        return Err("CARGO_REGISTRY_TOKEN is required for live publication".into());
    }
    let revision = input("REVISION")?;
    if !is_hex(&revision, 40) || revision != input("GITHUB_SHA")? {
        return Err("Publication revision must match this source".into());
    }
    is_protected_release()?;
    is_approved_environment()?;
    Cmd::new("cargo publish --locked --no-verify --package")
        .arg(input("PACKAGE")?)
        .args(["--registry", "crates-io"])
        .env(
            "CARGO_REGISTRIES_CRATES_IO_INDEX",
            "sparse+https://index.crates.io/",
        )
        .env("CARGO_REGISTRIES_CRATES_IO_TOKEN", &token)
        .env(
            "CARGO_REGISTRIES_CRATES_IO_CREDENTIAL_PROVIDER",
            "cargo:token",
        )
        .cwd(&path("PROJECT")?)
        .run()
}
