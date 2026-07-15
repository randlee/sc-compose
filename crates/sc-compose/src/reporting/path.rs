use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::DiagnosticCode;

use crate::CommandError;

pub(crate) fn resolve_relative_path(
    workspace_root: &Path,
    path: &Path,
) -> Result<PathBuf, CommandError> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };
    std::fs::canonicalize(&joined).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!("failed to resolve {}", joined.display())),
            DiagnosticCode::ErrConfigParse,
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::resolve_relative_path;

    #[test]
    fn resolve_relative_path_handles_relative_inputs() {
        let root = temp_root("relative");
        let file = root.join("reports").join("smoke").join("sample.txt");
        fs::create_dir_all(file.parent().expect("sample parent")).expect("create dirs");
        fs::write(&file, "ok").expect("write sample");

        let resolved =
            resolve_relative_path(&root, PathBuf::from("reports/smoke/sample.txt").as_path())
                .expect("resolve relative path");

        assert_eq!(resolved, fs::canonicalize(file).expect("canonicalize file"));
    }

    #[test]
    fn resolve_relative_path_preserves_absolute_inputs() {
        let root = temp_root("absolute");
        let file = root.join("sample.txt");
        fs::write(&file, "ok").expect("write sample");
        let absolute = fs::canonicalize(&file).expect("canonicalize sample");

        let resolved = resolve_relative_path(&root, &absolute).expect("resolve absolute path");

        assert_eq!(resolved, absolute);
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sc-compose-reporting-path-{label}-{nanos}"));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
