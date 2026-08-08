use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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
fn sc_lint_discovers_repository_and_fixture_roots_without_config_error() {
    for root in [
        repo_root(),
        repo_root().join("tests/fixtures/sc-lint/bootstrap"),
    ] {
        let value = sc_lint_json(&root, &["lint", "sc-boundary"]);
        assert_eq!(value["ok"], true, "unexpected top-level failure: {value}");
        assert_ne!(
            value["error"]["code"], "CLI.CONFIG_ERROR",
            "root discovery must not fail configuration: {value}"
        );
        assert_eq!(value["data"]["version"], "0.4.0");
        assert!(value["data"]["scanned_crates"].as_u64().is_some());
    }
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
    let forbidden_env = concat!("ATM", "_HOME");
    let forbidden_atm_import = concat!("use ", "atm", "_");
    let forbidden_agent_import = concat!("use ", "agent_", "team_", "mail::");
    let forbidden_manifest_deps = [concat!("agent", "-team-mail"), "atm-", "sc-lint"];
    let forbidden_research_refs = [concat!("reverse", "_extract")];
    let mut violations = Vec::new();

    for path in source_files(&root) {
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

    for path in cargo_manifests(&root) {
        let contents = fs::read_to_string(&path).expect("read manifest");
        for forbidden in forbidden_manifest_deps {
            if contents.contains(forbidden) {
                violations.push(format!(
                    "{}: forbidden manifest dependency {forbidden}",
                    path.display()
                ));
            }
        }

        if path == root.join("bindings/python/Cargo.toml") {
            for forbidden in ["sc-compose =", "sc-observability ="] {
                if contents.contains(forbidden) {
                    violations.push(format!(
                        "{}: forbidden Python binding dependency {forbidden}",
                        path.display()
                    ));
                }
            }
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

    let python_manifest =
        fs::read_to_string(root.join("bindings/python/Cargo.toml")).expect("python manifest");
    if !python_manifest
        .lines()
        .any(|line| line.trim_start().starts_with("sc-composer ="))
    {
        violations.push("bindings/python/Cargo.toml: missing sc-composer dependency".to_owned());
    }

    assert!(
        violations.is_empty(),
        "standalone boundary violations:\n{}",
        violations.join("\n")
    );
}
