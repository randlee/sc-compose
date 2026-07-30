use std::fmt;
use std::path::{Path, PathBuf};

use crate::reporting::index::{ReportIndexEntry, ReportIndexError};

#[derive(Debug)]
pub(crate) enum PublishManifestError {
    Index(Box<ReportIndexError>),
    TimeFormat(time::error::Format),
    InvalidArtifactPath {
        report_id: String,
        path: PathBuf,
        message: String,
    },
    ReadArchiveRoot {
        path: PathBuf,
        source: std::io::Error,
    },
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    Serialize(serde_json::Error),
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

pub(super) fn invalid_artifact_path(
    report_id: &str,
    path: &Path,
    latest_report_root: &Path,
    message: impl fmt::Display,
) -> PublishManifestError {
    PublishManifestError::InvalidArtifactPath {
        report_id: report_id.to_owned(),
        path: path.to_path_buf(),
        message: format!(
            "artifact must remain under {}: {message}",
            latest_report_root.display()
        ),
    }
}

pub(super) fn missing_required_evidence_error(entry: &ReportIndexEntry) -> PublishManifestError {
    PublishManifestError::Index(Box::new(ReportIndexError::MissingRequiredEvidence {
        report_id: entry.report_id.clone(),
        missing_paths: entry.missing_paths.clone(),
    }))
}

impl fmt::Display for PublishManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Index(source) => write!(f, "{source}"),
            Self::TimeFormat(source) => {
                write!(f, "failed to format publish-manifest timestamp: {source}")
            }
            Self::InvalidArtifactPath {
                report_id,
                path,
                message,
            } => write!(
                f,
                "invalid publish-manifest artifact path for {report_id} {}: {message}",
                path.display()
            ),
            Self::ReadArchiveRoot { path, source } => write!(
                f,
                "failed to inspect archive root {}: {source}",
                path.display()
            ),
            Self::CreateDir { path, source } => write!(
                f,
                "failed to create publish-manifest dir {}: {source}",
                path.display()
            ),
            Self::Serialize(source) => write!(f, "failed to serialize publish manifest: {source}"),
            Self::Write { path, source } => write!(
                f,
                "failed to write publish manifest {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for PublishManifestError {}
