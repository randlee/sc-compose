//! Parallel lint fixtures must start with Cargo's current lockfile.
mod support;

use std::fs;
use std::process::Command;
use support::{TempFixture, copy_directory, repo_root};

#[test]
fn parallel_lint_fixture_lockfiles_match_offline_regeneration() {
    for fixture in [
        "lint-ci/pass",
        "lint-ci/failing-manifest",
        "lint-fast/pass",
        "lint-fast/failing-manifest",
    ] {
        let source = repo_root().join("tests/fixtures/sc-lint").join(fixture);
        let copy = TempFixture::new("parallel-lint-lock");
        copy_directory(&source, &copy.path);
        let lock = copy.path.join("Cargo.lock");
        let checked_in = fs::read(&lock).expect("read fixture lockfile");
        let result = Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(&copy.path)
            .env("CARGO_HOME", copy.path.join("cargo-home"))
            .output()
            .expect("run cargo generate-lockfile");
        assert!(
            result.status.success(),
            "{fixture}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            fs::read(&lock).expect("read regenerated lockfile"),
            checked_in,
            "{fixture}: checked-in Cargo.lock differs from offline regeneration"
        );
    }
}
