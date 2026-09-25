//! The pins the local gate shares with CI: tool versions, the toolchain, and
//! the speed target.

use crate::harness::{
    Fixture, described, described_step, query, root, succeeds, tool_rows, workflow,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[test]
fn the_local_gate_and_ci_install_the_same_tool_versions() {
    // ENF-008 requires the local aggregate check to run the same commands as its
    // CI gate. Two different versions of the same tool is the quiet way that
    // stops being true: the gate passes locally and fails in CI, or worse, the
    // reverse.
    let root = root();
    let version = |url: &str| -> Option<String> {
        let tail = url.split("/download/").nth(1)?;
        // A tag can carry a path, URL-encoded: `cargo-audit%2Fv0.22.2`. Reading
        // digits before decoding it yielded `2` on both sides of the comparison,
        // so that one pin could never have been caught drifting.
        let tag = tail.split('/').next()?.replace("%2F", "/");
        let tag = tag.rsplit('/').next()?;
        let digits: String = tag
            .chars()
            .skip_while(|character| !character.is_ascii_digit())
            .take_while(|character| character.is_ascii_digit() || *character == '.')
            .collect();
        (!digits.is_empty()).then_some(digits)
    };

    let mut local = pinned_tools(&root);
    assert!(local.len() > 5, "no pinned tools found in mise.toml");
    // mise installs everything else, so bootstrap.sh pins mise itself.
    let bootstrap = fs::read_to_string(root.join("scripts/bootstrap.sh")).unwrap();
    let mise = bootstrap
        .lines()
        .find_map(|line| line.strip_prefix("readonly MISE_VERSION=v"))
        .unwrap();
    local.insert("mise".to_owned(), mise.to_owned());

    // ci.yml's pins are the tables of its `rust-gate install-tools` steps.
    let ci = workflow("ci");
    let rows: Vec<_> = ci["jobs"]["checks"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(tool_rows)
        .collect();
    let mut compared = 0;
    for row in &rows {
        let (Some(hosted), Some(here)) = (version(&row.asset), local.get(&row.name)) else {
            continue;
        };
        assert_eq!(
            &hosted, here,
            "{} is pinned to {hosted} in CI but {here} in the local gate",
            row.name
        );
        compared += 1;
    }
    assert!(compared > 5, "no tool versions were actually compared");

    // Matching versions is not enough: a tool CI installs and the local gate
    // does not cannot be reproduced by a contributor, only read about. This is
    // how the optional gates silently became CI-only.
    for row in &rows {
        let name = &row.name;
        assert!(
            local.contains_key(name),
            "CI installs {name} but mise.toml does not, so its gate cannot be run locally"
        );
    }

    // Release evidence uses the runner's GitHub CLI and the same pinned parser
    // as CI. No corporate client or authentication bootstrap remains.
    let steps = described();
    assert!(
        described_step(&steps, "rust-gate publish-evidence upload")
            .is_some_and(|upload| upload.tools == ["gh", "jaq"])
    );
}

#[test]
fn every_rendered_hook_runs_a_pinned_version_and_this_repository_runs_them_all() {
    // The hooks a repository gets are rendered by the gate; the tools they
    // run come from the toolbelt `rust-gate setup` installs from this
    // repository's pins, and this repository runs every one of them on its
    // own pinned toolbelt.
    let fixture = Fixture::with_sources(&[("src/lib.rs", "//! A crate.\n")]);
    succeeds(&fixture.run_body(&format!(
        "cd project && RUST_WORKFLOWS_PIN='{} v2.0.0' rust-gate init",
        "a".repeat(40)
    )));
    let rendered = fixture.root.join("project/.pre-commit-config.yaml");
    let pinned = pinned_tools(&root());
    let programs = query(
        &rendered,
        r#".repos[].hooks[] | select(.language == "system") | .entry | split(" ")[0]"#,
    );
    let mut compared = 0;
    for program in programs.lines().filter(|program| *program != "cargo") {
        assert!(
            pinned.contains_key(program),
            "a hook runs {program}, which the toolbelt does not pin"
        );
        compared += 1;
    }
    assert!(compared > 10, "only {compared} hook tools compared");
    // The gate's own hooks build jaq at the version the toolbelt pins.
    let specs = query(
        &rendered,
        ".repos[].hooks[].additional_dependencies // [] | .[]",
    );
    assert!(
        specs
            .lines()
            .any(|spec| spec == format!("cli:jaq:{}", pinned["jaq"])),
        "{specs}"
    );
    let ids = |path: &Path| -> Vec<String> {
        query(path, ".repos[].hooks[].id")
            .lines()
            .map(str::to_owned)
            .collect()
    };
    let here = ids(&root().join(".pre-commit-config.yaml"));
    for id in ids(&rendered) {
        // The gate's rules and Clippy run in `just check` here, per crate, and
        // so does `rust-gate rules --check` on this repository's rule map;
        // `just check` is also what runs before a push, where another
        // repository runs `rust-gate ci --local`. The guide is not written
        // here: this repository keeps its own, the model, under its own
        // inventory test.
        if matches!(
            id.as_str(),
            "rust-gate-architecture"
                | "rust-gate-hygiene"
                | "rust-gate-rules"
                | "rust-gate-guide"
                | "rust-gate-ci"
        ) {
            continue;
        }
        assert!(
            here.contains(&id),
            "this repository does not run the hook {id}"
        );
    }
}

#[test]
fn the_local_toolchain_pin_has_one_copy() {
    // bootstrap.sh and the justfile used to carry their own `1.x.y`; a bump in
    // one and not the other installed a toolchain nothing ran on. Both now
    // read the root rust-toolchain.toml, so no literal may creep back.
    let root = root();
    let pin = fs::read_to_string(root.join("rust-toolchain.toml")).unwrap();
    assert!(
        pin.contains("channel = \"1."),
        "rust-toolchain.toml must pin an exact stable version"
    );
    // rustup walks up from where cargo runs, so the root file serves the gate
    // crate, the test crate and the action's build alike; a copy beside a
    // crate would be a second pin to forget.
    for copy in ["gate/rust-toolchain.toml", "tests/rust-toolchain.toml"] {
        assert!(!root.join(copy).exists(), "{copy} duplicates the root pin");
    }
    for file in ["scripts/bootstrap.sh", "justfile"] {
        let text = fs::read_to_string(root.join(file)).unwrap();
        assert!(
            text.contains("rust-toolchain.toml"),
            "{file} must read the toolchain from rust-toolchain.toml"
        );
        for literal in ["TOOLCHAIN=1.", "+1.", "install 1."] {
            assert!(
                !text.contains(literal),
                "{file} carries its own toolchain pin: {literal}"
            );
        }
    }
}

#[test]
fn the_speed_target_is_the_one_the_gate_prints() {
    // The North Star's speed KPI is measured by the justfile: `just check`
    // prints its wall time against the target. The number lives in the
    // justfile, so the table cannot promise a target the gate does not measure.
    let root = root();
    let justfile = fs::read_to_string(root.join("justfile")).unwrap();
    let target = justfile
        .lines()
        .find_map(|line| {
            line.trim_start_matches("export ")
                .strip_prefix("SPEED_TARGET_SECONDS := \"")
        })
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("the justfile declares SPEED_TARGET_SECONDS");
    assert!(
        target
            .parse::<u64>()
            .is_ok_and(|seconds| (5..=120).contains(&seconds)),
        "implausible speed target {target:?}"
    );
    assert!(
        justfile.contains("echo \"SPEED: "),
        "the justfile must print the duration in check"
    );
    // The Speed bar lives in controls.md's four axes, its KPI in northstar.md.
    let speed_row = |page: &str| -> Vec<String> {
        fs::read_to_string(root.join("docs/standards").join(page))
            .unwrap()
            .lines()
            .filter(|line| line.starts_with("| Speed |"))
            .map(str::to_owned)
            .collect()
    };
    let (bar, kpi) = (speed_row("controls.md"), speed_row("northstar.md"));
    assert_eq!(
        (bar.len(), kpi.len()),
        (1, 1),
        "one Speed bar and one Speed KPI"
    );
    assert!(
        bar[0].contains(&format!("at or under {target} seconds")),
        "the Speed bar must state the justfile's target: {}",
        bar[0]
    );
    assert!(
        kpi[0].contains(&format!("| {target} s or less |")),
        "the Speed KPI must target the justfile's number: {}",
        kpi[0]
    );
}

#[test]
fn every_pinned_tool_has_a_row_in_the_readme_toolbelt() {
    // The README table is what a reader takes the toolbelt to be. A tool added
    // to mise.toml and not to the table is one nobody knows to expect; a row
    // left behind after a removal describes something `just setup` no longer
    // installs. Only mise itself is a row without a pin: it installs the rest.
    let root = root();
    let pinned = pinned_tools(&root);
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let toolbelt = readme
        .split_once("## 🧰 Toolbelt")
        .expect("the README must carry the toolbelt section")
        .1
        .split("\n## ")
        .next()
        .unwrap()
        .to_owned();
    for name in pinned.keys() {
        assert!(
            toolbelt.contains(&format!("| `{name}` |")),
            "{name} is pinned in mise.toml but has no row in the README toolbelt"
        );
    }
    let rows = toolbelt
        .lines()
        .filter(|line| line.starts_with("| `"))
        .count();
    assert_eq!(
        rows,
        pinned.len() + 1,
        "the toolbelt must list mise and every pinned tool, and nothing else"
    );
}

/// The tools `mise.toml` pins, name to version: `name = "1.2.3"` or `name =
/// { version = "1.2.3", ... }` under `[tools]`. Its aliases give the plugins
/// that live in a monorepo the same names their release assets install them under.
fn pinned_tools(root: &Path) -> BTreeMap<String, String> {
    let mut pinned = BTreeMap::new();
    let mut in_tools = false;
    for line in fs::read_to_string(root.join("mise.toml")).unwrap().lines() {
        if line.starts_with('[') {
            in_tools = line.trim() == "[tools]";
            continue;
        }
        if !in_tools || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let after = value.split("version =").last().unwrap();
        if let Some(found) = after.split('"').nth(1) {
            pinned.insert(name.trim().trim_matches('"').to_owned(), found.to_owned());
        }
    }
    pinned
}
