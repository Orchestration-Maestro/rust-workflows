//! `fuzz.yml`: the nightly installed with `rust-src` and `cargo-fuzz`, and
//! every committed target replayed, corpus first, within the time budget.

use crate::harness::{Fixture, succeeds};
use std::fs;

#[test]
fn the_fuzz_toolchain_installs_the_nightly_with_rust_src_and_cargo_fuzz() {
    let mut fixture = Fixture::new();
    fixture.set("RUSTUP_TOOLCHAIN", "nightly-2026-09-14");
    fixture.stub("rustup", "");
    fixture.stub("cargo", "");
    succeeds(&fixture.run_body("rust-gate fuzz toolchain"));
    let trace = fixture.trace();
    for command in [
        "rustup toolchain install nightly-2026-09-14 --profile minimal --component rust-src",
        "cargo install --locked cargo-fuzz --version 0.13.1",
        "rustup run nightly-2026-09-14 rustc --version",
    ] {
        assert!(trace.contains(command), "{command} not in {trace}");
    }
    assert!(fixture.root.join("reports/toolchain.txt").is_file());
}

#[test]
fn the_fuzz_replay_runs_every_committed_target_with_the_corpus_first() {
    let mut fixture = Fixture::new();
    fixture.set("TARGET", "");
    fixture.set("MAX_TOTAL_TIME", "60");
    let targets = fixture.root.join("project/fuzz/fuzz_targets");
    fs::create_dir_all(&targets).unwrap();
    for name in ["parse.rs", "render.rs", "README.md"] {
        fs::write(targets.join(name), "").unwrap();
    }
    fixture.stub("cargo", "");
    succeeds(&fixture.run("fuzz", "replay"));
    let trace = fixture.trace();
    let corpus = trace.find("cargo fuzz run parse -- -runs=0").expect(&trace);
    let budget = trace
        .find("cargo fuzz run parse -- -max_total_time=60")
        .expect(&trace);
    let next = trace
        .find("cargo fuzz run render -- -runs=0")
        .expect(&trace);
    assert!(corpus < budget && budget < next);
    assert!(!trace.contains("README"));
    fs::remove_dir_all(&targets).unwrap();
    fs::create_dir_all(&targets).unwrap();
    let refused = fixture.run("fuzz", "replay");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("No fuzz targets found"), "{stderr}");
}
