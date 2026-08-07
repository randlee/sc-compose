use std::path::{Path, PathBuf};

use super::OutputError;

pub(crate) const LATEST_ROOT_RELATIVE_PATH: &str = "reports/latest";
pub(crate) const ARCHIVE_ROOT_RELATIVE_PATH: &str = "reports/archive";

pub(super) fn latest_report_root(
    entrypoint: &Path,
    report_id: &str,
) -> Result<PathBuf, OutputError> {
    let report_root = PathBuf::from(LATEST_ROOT_RELATIVE_PATH).join(report_id);
    if !entrypoint.starts_with(&report_root) {
        return Err(OutputError::InvalidPath {
            path: entrypoint.to_path_buf(),
            message: format!("entrypoint must remain under {}", report_root.display()),
        });
    }
    Ok(report_root)
}

pub(super) fn archive_root(archive_label: &str, report_id: &str) -> PathBuf {
    PathBuf::from(ARCHIVE_ROOT_RELATIVE_PATH)
        .join(archive_label)
        .join(report_id)
}

pub(super) fn relative_artifact(
    artifact: &Path,
    latest_report_root: &Path,
) -> Result<PathBuf, OutputError> {
    artifact
        .strip_prefix(latest_report_root)
        .map(PathBuf::from)
        .map_err(|_strip_error| OutputError::InvalidPath {
            path: artifact.to_path_buf(),
            message: format!(
                "artifact must remain under {}",
                latest_report_root.display()
            ),
        })
}
