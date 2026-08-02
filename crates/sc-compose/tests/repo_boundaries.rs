use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
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
    let forbidden_manifest_dep = concat!("agent", "-team-mail", "-");
    let forbidden_research_refs = [
        concat!("prototype/", "reverse", "_extract"),
        concat!("prototype::", "reverse", "_extract"),
        concat!("reverse", "_extract"),
    ];
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
        if contents.contains(forbidden_manifest_dep) {
            violations.push(format!(
                "{}: forbidden manifest dependency family",
                path.display()
            ));
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
    for forbidden in [
        "sc-compose =",
        "sc-observability =",
        "agent-team-mail",
        "atm-",
    ] {
        if python_manifest.contains(forbidden) {
            violations.push(format!(
                "bindings/python/Cargo.toml: forbidden dependency {forbidden}"
            ));
        }
    }

    let composer_manifest =
        fs::read_to_string(root.join("crates/sc-composer/Cargo.toml")).expect("composer manifest");
    for forbidden in [
        "sc-compose =",
        "sc-compose-py =",
        "bindings/python",
        "sc-observability =",
    ] {
        if composer_manifest.contains(forbidden) {
            violations.push(format!(
                "crates/sc-composer/Cargo.toml: forbidden dependency {forbidden}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "standalone boundary violations:\n{}",
        violations.join("\n")
    );
}
