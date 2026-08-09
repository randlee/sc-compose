//! End-to-end coverage for the identity-literals sc-lint target.
//!
//! The Python adapter is deliberately sourced from the pinned sc-lint
//! materialization used by CI (or the sibling checkout for local runs). It is
//! copied only into each temporary fixture, never into this repository.

mod support;

use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use support::{repo_root, sc_compose, temp_root};

const TARGET: &str = "identity-literals";
const IDENTITY: &str = "team-lead@example.invalid";
const RUNTIME_FILES: &[&str] = &[
    "lint_identity_literals.py",
    "lint_common.py",
    "python_adapter.py",
    "view_common.py",
];

#[test]
fn identity_literals_pass_preserves_adapter_envelope_and_report() {
    let (root, output) = run_target("pass");
    assert_success(&output);

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.identity-literals");
    assert_eq!(payload["target"], "lint.identity-literals");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(payload["raw_payload"]["ok"], true);
    assert_eq!(payload["raw_payload"]["command"], "lint.identity-literals");
    assert_eq!(
        payload["raw_payload"]["data"]["adapter"],
        "sc-lint-python-v1"
    );
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(
        payload["raw_payload"]["data"]["findings"],
        serde_json::json!([])
    );
    assert_eq!(
        payload["raw_payload"]["data"]["forbidden_literals"][0],
        IDENTITY
    );
    assert_eq!(
        payload["raw_payload"]["data"]["production_canonical_literals"][IDENTITY][0],
        "crates/demo/src/constants.rs"
    );
    assert_eq!(
        payload["raw_artifact"],
        "reports/latest/sc-lint/raw/lint.identity-literals.json"
    );

    let report = fs::read_to_string(root.join("reports/latest/sc-lint/index.html"))
        .expect("identity-literals report");
    assert!(report.contains("lint.identity-literals"));
    assert!(report.contains("pass"));
    assert!(
        root.join("reports/latest/sc-lint/raw/lint.identity-literals.json")
            .is_file()
    );

    remove_fixture(&root);
}

#[test]
fn identity_literals_findings_remain_non_pass_with_structured_evidence() {
    let (root, output) = run_target("fail");
    assert_success(&output);

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.identity-literals");
    assert_eq!(payload["outcome"], "findings");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["findings_count"], 3);
    assert_ne!(payload["outcome"], "pass");
    assert_eq!(payload["raw_payload"]["ok"], true);
    assert_eq!(
        payload["raw_payload"]["data"]["adapter"],
        "sc-lint-python-v1"
    );
    assert_eq!(payload["raw_payload"]["data"]["status"], "fail");
    assert_eq!(
        payload["raw_payload"]["data"]["violation_kinds"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let findings = payload["findings"].as_array().expect("finding array");
    assert!(
        findings
            .iter()
            .any(|finding| finding.as_str().unwrap().contains("crates/demo/src/lib.rs"))
    );
    assert!(findings.iter().any(|finding| {
        finding
            .as_str()
            .unwrap()
            .contains("crates/demo/tests/identity_literals.rs")
    }));
    assert!(
        findings
            .iter()
            .all(|finding| finding.as_str().unwrap().contains(IDENTITY))
    );

    let report = fs::read_to_string(root.join("reports/latest/sc-lint/index.html"))
        .expect("identity-literals report");
    assert!(report.contains("findings"));
    assert!(report.contains("crates/demo/src/lib.rs"));
    assert!(report.contains(IDENTITY));
    assert!(
        root.join("reports/latest/sc-lint/raw/lint.identity-literals.json")
            .is_file()
    );

    remove_fixture(&root);
}

fn run_target(fixture: &str) -> (PathBuf, Output) {
    let root = temp_root(&format!("sc-lint-identity-literals-{fixture}"));
    copy_directory(
        &repo_root()
            .join("tests/fixtures/sc-lint/identity-literals")
            .join(fixture),
        &root,
    );
    materialize_sc_lint_runtime(&root);
    fs::create_dir_all(root.join(".sc/sc-lint/targets")).expect("target registry");
    fs::copy(
        repo_root().join(".sc/sc-lint/targets/identity-literals.toml"),
        root.join(".sc/sc-lint/targets/identity-literals.toml"),
    )
    .expect("identity-literals descriptor");

    let output = sc_compose()
        .args([
            "lint",
            "--root",
            root.to_str().expect("UTF-8 fixture root"),
            "--target",
            TARGET,
            "--json",
        ])
        .output()
        .expect("run sc-compose identity-literals");
    (root, output)
}

fn materialize_sc_lint_runtime(root: &Path) {
    let source = sc_lint_just_root();
    let destination = root.join(".just");
    fs::create_dir_all(&destination).expect("fixture just directory");
    for file in RUNTIME_FILES {
        let source_file = source.join(file);
        assert!(
            source_file.is_file(),
            "missing sc-lint runtime file: {}",
            source_file.display()
        );
        fs::copy(&source_file, destination.join(file)).expect("materialize sc-lint runtime");
    }
}

fn sc_lint_just_root() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(source_root) = env::var_os("SC_LINT_SOURCE_ROOT") {
        candidates.push(PathBuf::from(source_root).join(".just"));
    }
    candidates.push(repo_root().join(".just"));
    for ancestor in repo_root().ancestors() {
        candidates.push(ancestor.join("sc-lint/.just"));
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.join("lint_identity_literals.py").is_file())
        .unwrap_or_else(|| {
            panic!(
                "sc-lint Python utilities are unavailable; run the setup-sc-lint action or set SC_LINT_SOURCE_ROOT"
            )
        })
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("fixture destination");
    for entry in fs::read_dir(source).expect("fixture source") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).expect("fixture parent");
            }
            fs::copy(source_path, destination_path).expect("fixture file");
        }
    }
}

fn parse_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "sc-compose did not emit JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "sc-compose lint failed: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn remove_fixture(root: &Path) {
    fs::remove_dir_all(root).expect("remove temporary identity-literals fixture");
}
