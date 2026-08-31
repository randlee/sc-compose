use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(feature = "external-lint-cli-tests")]
use serde_json::Value;

mod support;
use support::{TempFixture, copy_directory};

fn repo_root() -> PathBuf {
    let canonical = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root");
    let Some(path) = canonical.to_str() else {
        return canonical;
    };
    PathBuf::from(path.strip_prefix(r"\\?\").unwrap_or(path))
}

#[cfg(feature = "external-lint-cli-tests")]
fn sc_lint_json(root: &Path, args: &[&str]) -> Value {
    let output = Command::new("sc-lint")
        .args(["--json", "--root"])
        .arg(root)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("sc-lint must be installed for L.1 validation: {error}"));
    assert!(
        output.status.success(),
        "sc-lint command failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "sc-lint must return JSON: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
#[cfg(feature = "external-lint-cli-tests")]
fn sc_lint_version_is_pinned_to_the_bootstrap_contract() {
    let output = Command::new("sc-lint")
        .args(["version", "--json"])
        .output()
        .expect("sc-lint must be installed for L.1 validation");
    assert!(
        output.status.success(),
        "sc-lint version failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("version JSON");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["contract_schema"], "v1");
    assert_eq!(value["data"]["crate_version"], "0.4.0");
}

#[test]
#[cfg(feature = "external-lint-cli-tests")]
fn sc_lint_discovers_repository_and_fixture_roots_without_config_error() {
    let checked_in_fixture = repo_root().join("tests/fixtures/sc-lint/bootstrap");
    let fixture = TempFixture::new("sc-lint-bootstrap");
    copy_directory(&checked_in_fixture, &fixture.path);
    let original_lock = fs::read_to_string(checked_in_fixture.join("Cargo.lock"))
        .expect("read checked-in fixture lockfile");

    for root in [repo_root(), fixture.path.clone()] {
        let value = sc_lint_json(&root, &["lint", "sc-boundary"]);
        assert_eq!(value["ok"], true, "unexpected top-level failure: {value}");
        assert_ne!(
            value["error"]["code"], "CLI.CONFIG_ERROR",
            "root discovery must not fail configuration: {value}"
        );
        assert_eq!(value["data"]["version"], "0.4.0");
        assert!(value["data"]["scanned_crates"].as_u64().is_some());
    }

    assert_eq!(
        fs::read_to_string(checked_in_fixture.join("Cargo.lock"))
            .expect("re-read checked-in fixture lockfile"),
        original_lock,
        "sc-lint bootstrap validation must not mutate checked-in fixtures"
    );
}

#[test]
fn sc_lint_bootstrap_lock_matches_cargo_metadata_regeneration() {
    let repository = repo_root();
    let checked_in_fixture = repository.join("tests/fixtures/sc-lint/bootstrap");
    let fixture = TempFixture::new("sc-lint-bootstrap-lock-regeneration");
    copy_directory(&checked_in_fixture, &fixture.path);
    fs::remove_file(fixture.path.join("Cargo.lock"))
        .expect("remove copied bootstrap fixture lockfile before regeneration");

    let output = Command::new("cargo")
        .current_dir(&repository)
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(fixture.path.join("Cargo.toml"))
        .output()
        .expect("Cargo metadata must regenerate the copied bootstrap fixture lockfile");
    assert!(
        output.status.success(),
        "Cargo metadata failed while regenerating the bootstrap fixture lockfile: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = fs::read_to_string(checked_in_fixture.join("Cargo.lock"))
        .expect("read checked-in bootstrap fixture lockfile");
    let actual = fs::read_to_string(fixture.path.join("Cargo.lock"))
        .expect("read regenerated bootstrap fixture lockfile");
    assert_eq!(
        actual, expected,
        "tests/fixtures/sc-lint/bootstrap/Cargo.lock is stale; regenerate it with `cargo metadata --offline --format-version 1 --manifest-path tests/fixtures/sc-lint/bootstrap/Cargo.toml`"
    );
}

fn walk_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| matches!(name, "target" | ".git"))
        {
            continue;
        }
        if entry.file_type().expect("file type").is_dir() {
            walk_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for (source_root, extension) in source_roots(root) {
        assert!(
            source_root.is_dir(),
            "required source root missing: {}",
            source_root.display()
        );
        let mut discovered = Vec::new();
        walk_files(&source_root, &mut discovered);
        files.extend(
            discovered
                .into_iter()
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(extension)),
        );
    }
    files
}

fn source_roots(root: &Path) -> [(PathBuf, &'static str); 3] {
    [
        (root.join("crates"), "rs"),
        (root.join("bindings/python/src"), "rs"),
        (root.join("bindings/python/python"), "py"),
    ]
}

fn cargo_manifests(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_files(root, &mut files);
    files
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "Cargo.toml")
        })
        .collect()
}

#[test]
fn repo_keeps_standalone_boundary_rules() {
    let root = repo_root();
    let mut violations = Vec::new();

    assert_source_boundary_rules(&root, &mut violations);
    assert_manifest_dependency_rules(&root, &mut violations);
    assert_python_adapter_boundary_rules(&root, &mut violations);
    assert_required_dependency_rules(&root, &mut violations);

    assert!(
        violations.is_empty(),
        "standalone boundary violations:\n{}",
        violations.join("\n")
    );
}

fn assert_source_boundary_rules(root: &Path, violations: &mut Vec<String>) {
    let forbidden_env = concat!("ATM", "_HOME");
    let forbidden_atm_import = concat!("use ", "atm", "_");
    let forbidden_agent_import = concat!("use ", "agent_", "team_", "mail::");
    let forbidden_research_refs = [concat!("reverse", "_extract")];

    for path in source_files(root) {
        let contents = fs::read_to_string(&path).expect("read source");
        for (rule, pattern) in [
            ("env", forbidden_env),
            ("atm import", forbidden_atm_import),
            ("agent import", forbidden_agent_import),
        ] {
            if contents.contains(pattern) {
                violations.push(format!("{}: forbidden {} reference", path.display(), rule));
            }
        }
        if forbidden_research_refs
            .iter()
            .any(|pattern| contents.contains(pattern))
        {
            violations.push(format!(
                "{}: production source references research-only extraction artifact",
                path.display()
            ));
        }
    }
}

fn assert_manifest_dependency_rules(root: &Path, violations: &mut Vec<String>) {
    let forbidden_manifest_deps = [concat!("agent", "-team-mail"), "atm-"];

    for path in cargo_manifests(root) {
        let contents = fs::read_to_string(&path).expect("read manifest");
        for forbidden in forbidden_manifest_deps {
            if contents.contains(forbidden) {
                violations.push(format!(
                    "{}: forbidden manifest dependency {forbidden}",
                    path.display()
                ));
            }
        }

        if (path == root.join("crates/sc-composer/Cargo.toml")
            || path == root.join("crates/sc-compose/Cargo.toml"))
            && contents.contains("sc-lint")
        {
            violations.push(format!(
                "{}: forbidden sc-lint-family dependency",
                path.display()
            ));
        }

        if path == root.join("crates/sc-composer/Cargo.toml") {
            for forbidden in [
                "sc-compose =",
                "sc-compose-py =",
                "bindings/python",
                "sc-observability =",
            ] {
                if contents.contains(forbidden) {
                    violations.push(format!(
                        "{}: forbidden composer dependency {forbidden}",
                        path.display()
                    ));
                }
            }
        }
    }
}

fn assert_python_adapter_boundary_rules(root: &Path, violations: &mut Vec<String>) {
    let manifest = root.join("bindings/python/Cargo.toml");
    if !manifest.is_file() {
        return;
    }
    let contents = fs::read_to_string(&manifest).expect("read python manifest");
    for forbidden in ["sc-compose =", "sc-observability ="] {
        if contents.contains(forbidden) {
            violations.push(format!(
                "{}: forbidden Python binding dependency {forbidden}",
                manifest.display()
            ));
        }
    }
}

fn assert_required_dependency_rules(root: &Path, violations: &mut Vec<String>) {
    let manifest = root.join("bindings/python/Cargo.toml");
    let python_manifest = fs::read_to_string(&manifest).expect("python manifest");
    if !python_manifest
        .lines()
        .any(|line| line.trim_start().starts_with("sc-composer ="))
    {
        violations.push(format!(
            "{}: missing sc-composer dependency",
            manifest.display()
        ));
    }
}

fn boundary_fixture(name: &str) -> TempFixture {
    let fixture = TempFixture::new(name);
    for directory in ["crates", "bindings/python/src", "bindings/python/python"] {
        fs::create_dir_all(fixture.path.join(directory)).expect("create boundary fixture root");
    }
    fixture
}

fn write_boundary_fixture(root: &Path, relative: &str, contents: &str) {
    support::write_file(&root.join(relative), contents);
}

#[test]
fn source_boundary_rules_reject_atm_and_research_patterns() {
    let fixture = boundary_fixture("source-boundary-negative");
    write_boundary_fixture(
        &fixture.path,
        "crates/fixture.rs",
        concat!(
            "ATM",
            "_HOME\nuse ",
            "atm",
            "_adapter;\nuse ",
            "agent_",
            "team_",
            "mail::Client;\nreverse",
            "_extract"
        ),
    );

    let mut violations = Vec::new();
    assert_source_boundary_rules(&fixture.path, &mut violations);

    for expected in [
        "forbidden env reference",
        "forbidden atm import reference",
        "forbidden agent import reference",
        "research-only extraction artifact",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(expected)),
            "missing violation for {expected}: {violations:#?}"
        );
    }
}

#[test]
fn manifest_boundary_rules_reject_forbidden_and_reverse_dependencies() {
    let fixture = boundary_fixture("manifest-boundary-negative");
    write_boundary_fixture(
        &fixture.path,
        "crates/other/Cargo.toml",
        "agent-team-mail = \"1\"\natm-helper = \"1\"\n",
    );
    write_boundary_fixture(
        &fixture.path,
        "crates/sc-compose/Cargo.toml",
        "sc-lint = \"1\"\n",
    );
    write_boundary_fixture(
        &fixture.path,
        "crates/sc-composer/Cargo.toml",
        "sc-lint = \"1\"\nsc-compose = \"1\"\nsc-compose-py = \"1\"\nbindings/python = \"1\"\nsc-observability = \"1\"\n",
    );

    let mut violations = Vec::new();
    assert_manifest_dependency_rules(&fixture.path, &mut violations);

    for expected in [
        "forbidden manifest dependency agent-team-mail",
        "forbidden manifest dependency atm-",
        "forbidden sc-lint-family dependency",
        "forbidden composer dependency sc-compose =",
        "forbidden composer dependency sc-compose-py =",
        "forbidden composer dependency bindings/python",
        "forbidden composer dependency sc-observability =",
    ] {
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains(expected)),
            "missing violation for {expected}: {violations:#?}"
        );
    }
}

#[test]
fn python_adapter_boundary_rules_reject_forbidden_dependencies() {
    let fixture = boundary_fixture("python-adapter-boundary-negative");
    write_boundary_fixture(
        &fixture.path,
        "bindings/python/Cargo.toml",
        "sc-compose = \"1\"\nsc-observability = \"1\"\n",
    );

    let mut violations = Vec::new();
    assert_python_adapter_boundary_rules(&fixture.path, &mut violations);

    assert!(violations
        .iter()
        .any(|violation| violation.contains("forbidden Python binding dependency sc-compose =")));
    assert!(violations.iter().any(|violation| {
        violation.contains("forbidden Python binding dependency sc-observability =")
    }));
}

#[test]
fn required_dependency_rules_reject_missing_python_adapter_dependency() {
    let fixture = boundary_fixture("required-dependency-boundary-negative");
    write_boundary_fixture(
        &fixture.path,
        "bindings/python/Cargo.toml",
        "pyo3 = \"1\"\n",
    );

    let mut violations = Vec::new();
    assert_required_dependency_rules(&fixture.path, &mut violations);

    assert_eq!(
        violations,
        vec![format!(
            "{}: missing sc-composer dependency",
            fixture.path.join("bindings/python/Cargo.toml").display()
        )]
    );
}
