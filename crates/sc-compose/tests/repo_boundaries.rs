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
    walk_files(&root.join("crates"), &mut files);
    files
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .collect()
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

    assert!(
        violations.is_empty(),
        "standalone boundary violations:\n{}",
        violations.join("\n")
    );
}
