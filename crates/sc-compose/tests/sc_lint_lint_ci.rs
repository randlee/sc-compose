mod support;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use support::{parse_stdout, repo_root, sc_compose, temp_root};

const RUNTIME_FILES: &[&str] = &[
    "lint_cargo_deny.py",
    "lint_cargo_shear.py",
    "check_version_sync.py",
    "lint_manifests.py",
    "lint_codespell.py",
    "run_pytests.py",
    "lint_sc_boundary.py",
    "lint_sc_portability.py",
    "lint_common.py",
];

#[test]
fn lint_ci_preserves_known_sc_lint_boundary_packaging_defect() {
    let (root, output) = run_target("pass");
    assert_eq!(
        output.status.code(),
        Some(5),
        "lint ci should preserve the known backend failure; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.ci");
    assert_eq!(payload["target"], "lint.ci");
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["command"], "lint.ci");
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["step"],
        "sc-boundary"
    );
    assert!(
        payload["raw_payload"]["error"]["details"]["command"]
            .as_str()
            .is_some_and(|command| command.contains("lint_sc_boundary.py"))
    );
    let stdout = payload["raw_payload"]["error"]["details"]["stdout"]
        .as_str()
        .expect("boundary utility stdout");
    let stderr = payload["raw_payload"]["error"]["details"]["stderr"]
        .as_str()
        .expect("boundary utility stderr");
    let combined = format!("{stdout}\n{stderr}");
    assert!(combined.contains("sc-lint-boundary"));
    assert!(combined.contains("not found") || combined.contains("could not find"));
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(
        payload["diagnostics"][0]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );

    let report = root.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("known defect report");
    assert!(report_text.contains("lint.ci"));
    assert!(report_text.contains("failed"));
    assert!(report_text.contains("sc-boundary"));
    assert!(report_text.contains("sc-lint-boundary"));
    assert!(
        root.join("reports/latest/sc-lint/raw/lint.ci.json")
            .is_file()
    );

    remove_fixture(&root);
}

#[test]
fn lint_ci_manifest_failure_remains_non_pass_with_structured_diagnostics() {
    let (root, output) = run_target("failing-manifest");
    assert_eq!(
        output.status.code(),
        Some(5),
        "lint ci should retain profile failure status; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.ci");
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["command"], "lint.ci");
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["step"],
        "manifests"
    );
    assert!(
        payload["raw_payload"]["error"]["details"]["command"]
            .as_str()
            .is_some_and(|command| command.contains("lint_manifests.py"))
    );
    let stdout = payload["raw_payload"]["error"]["details"]["stdout"]
        .as_str()
        .expect("manifest utility stdout");
    assert!(stdout.contains("manifests failed"));
    assert!(stdout.contains("homepage"));
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(
        payload["diagnostics"][0]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );

    let report = root.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("manifest failure report");
    assert!(report_text.contains("lint.ci"));
    assert!(report_text.contains("failed"));
    assert!(report_text.contains("homepage"));
    assert!(
        root.join("reports/latest/sc-lint/raw/lint.ci.json")
            .is_file()
    );

    remove_fixture(&root);
}

fn run_target(fixture: &str) -> (PathBuf, Output) {
    let root = temp_root(&format!("sc-lint-ci-{fixture}"));
    copy_directory(
        &repo_root()
            .join("tests/fixtures/sc-lint/lint-ci")
            .join(fixture),
        &root,
    );
    materialize_sc_lint_runtime(&root);
    fs::create_dir_all(root.join(".sc/sc-lint/targets")).expect("target registry");
    fs::copy(
        repo_root().join(".sc/sc-lint/targets/lint-ci.toml"),
        root.join(".sc/sc-lint/targets/lint-ci.toml"),
    )
    .expect("lint-ci descriptor");

    let output = sc_compose()
        .args([
            "lint",
            "--root",
            root.to_str().expect("UTF-8 fixture root"),
            "--target",
            "lint-ci",
            "--json",
        ])
        .env("SC_LOG_ROOT", root.join("logs"))
        .output()
        .expect("run sc-compose lint ci");
    (root, output)
}

fn sc_lint_just_root() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(source_root) = env::var_os("SC_LINT_SOURCE_ROOT") {
        candidates.push(PathBuf::from(source_root).join(".just"));
    }
    candidates.push(repo_root().join(".just"));
    for ancestor in repo_root().ancestors() {
        candidates.push(ancestor.join("sc-lint").join(".just"));
    }

    candidates
        .into_iter()
        .find(|candidate| {
            RUNTIME_FILES
                .iter()
                .all(|file| candidate.join(file).is_file())
        })
        .unwrap_or_else(|| {
            panic!(
                "sc-lint Python utilities are unavailable; run the setup-sc-lint action or set SC_LINT_SOURCE_ROOT"
            )
        })
}

fn materialize_sc_lint_runtime(root: &Path) {
    let source = sc_lint_just_root();
    let destination = root.join(".just");
    fs::create_dir_all(&destination).expect("fixture just directory");
    for file in RUNTIME_FILES {
        fs::copy(source.join(file), destination.join(file)).expect("materialize sc-lint utility");
    }
    fs::copy(
        source.join("lint-config.toml"),
        destination.join("lint-config.toml"),
    )
    .expect("materialize sc-lint lint config");
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
            fs::copy(&source_path, &destination_path).expect("fixture file");
        }
    }
}

fn remove_fixture(root: &Path) {
    fs::remove_dir_all(root).expect("remove temporary lint-ci fixture");
}
