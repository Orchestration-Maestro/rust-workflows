//! The quality gate refuses a workspace whose manifests or sources reach
//! outside the checkout, before it formats, lints or tests anything.

use crate::harness::{Fixture, refused};
use serde_json::json;

#[test]
fn the_quality_gate_refuses_a_workspace_reaching_outside_the_checkout() {
    let mut f = Fixture::new();
    f.set("GITHUB_WORKSPACE", &f.root.display().to_string());
    let metadata = json!({"workspace_members": ["fixture"], "packages": [{
        "id": "fixture", "manifest_path": "/etc/hostname",
        "targets": [{"src_path": f.root.join("project/src/lib.rs")}]}]});
    f.set("METADATA", &metadata.to_string());
    f.stub(
        "cargo",
        r#"[[ "$1" == metadata ]] && printf '%s' "$METADATA"; exit 0"#,
    );
    refused(
        &f.run("ci", "quality"),
        "Workspace manifests and sources must remain inside checkout",
    );
    assert!(
        !f.calls().contains("fmt"),
        "nothing was formatted or linted"
    );
}
