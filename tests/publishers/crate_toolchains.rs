//! publish-crate.yml, its two toolchains: the dry run installs the one the
//! project pins and hands its channel on, the publication installs the one
//! the dry run validated.

use crate::harness::{Fixture, refused, succeeds};
use std::fs;

#[test]
fn the_publication_toolchain_is_the_validated_one_installed_in_the_project() {
    let mut fixture = Fixture::new();
    fixture.set("TOOLCHAIN", "1.97.1");
    fixture.stub("rustup", "");
    succeeds(&fixture.run_body("rust-gate publish-crate publish-toolchain"));
    let trace = fixture.trace();
    assert!(
        trace.contains("rustup toolchain install 1.97.1 --profile minimal"),
        "{trace}"
    );
    let environment = fs::read_to_string(fixture.root.join("environment")).unwrap();
    assert!(
        environment.contains("RUSTUP_TOOLCHAIN=1.97.1"),
        "{environment}"
    );
    let project = format!("PROJECT={}", fixture.root.join("project").display());
    assert!(environment.contains(&project), "{environment}");
    fixture.stub("rustup", "exit 2");
    let output = fixture.run_body("rust-gate publish-crate publish-toolchain");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn the_publication_refuses_a_toolchain_it_did_not_validate() {
    // The staging job hands the channel over as an output; the publication
    // checks it again before installing or exporting anything.
    for toolchain in ["stable", "1.97", "1.97.1\nINJECT=yes"] {
        let mut fixture = Fixture::new();
        fixture.set("TOOLCHAIN", toolchain);
        fixture.stub("rustup", "");
        refused(
            &fixture.run_body("rust-gate publish-crate publish-toolchain"),
            "TOOLCHAIN must be an exact stable version",
        );
        assert!(!fixture.root.join("environment").exists(), "{toolchain:?}");
    }
}

#[test]
fn the_dry_run_toolchain_is_the_one_the_project_pins() {
    let fixture = Fixture::new();
    fixture.stub("rustup", "");
    succeeds(&fixture.run("publish-crate", "toolchain"));
    assert!(
        fixture
            .trace()
            .contains("rustup toolchain install 1.98.1 --profile minimal")
    );
    let output = fs::read_to_string(fixture.root.join("output")).unwrap();
    assert!(output.contains("channel=1.98.1"), "{output}");
    let pin = fixture.root.join("project/rust-toolchain.toml");
    fs::write(&pin, "[toolchain]\nchannel=\"stable\"\n").unwrap();
    refused(
        &fixture.run("publish-crate", "toolchain"),
        "rust-toolchain.toml must pin an exact stable version",
    );
}
