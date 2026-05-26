use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;

pub(crate) const LATEST_ROOT_RELATIVE_PATH: &str = "reports/latest";
pub(crate) const ARCHIVE_ROOT_RELATIVE_PATH: &str = "reports/archive";

const ARCHIVE_TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]-[minute]-[second]Z");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReportMetadata {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    pub(crate) produced_at: String,
    pub(crate) status: String,
    pub(crate) entrypoint: String,
    pub(crate) artifacts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportOutputRequest {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) entrypoint: PathBuf,
    pub(crate) metadata_path: PathBuf,
    pub(crate) latest_artifacts: Vec<PathBuf>,
    pub(crate) archive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct MaterializedReport {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    pub(crate) produced_at: String,
    pub(crate) status: String,
    pub(crate) entrypoint: PathBuf,
    pub(crate) metadata: PathBuf,
    pub(crate) latest_artifacts: Vec<PathBuf>,
    pub(crate) archived_artifacts: Vec<PathBuf>,
}

#[derive(Debug)]
pub(crate) enum OutputError {
    TimeFormat(time::error::Format),
    InvalidPath {
        path: PathBuf,
        message: String,
    },
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    CopyFile {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
    WriteMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    SerializeMetadata(serde_json::Error),
}

pub(crate) fn write_report_metadata_and_archive(
    root: &Path,
    request: &ReportOutputRequest,
) -> Result<MaterializedReport, OutputError> {
    let produced_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(OutputError::TimeFormat)?;
    let archive_label = OffsetDateTime::now_utc()
        .format(ARCHIVE_TIMESTAMP_FORMAT)
        .map_err(OutputError::TimeFormat)?;

    let latest_report_root = latest_report_root(&request.entrypoint, &request.report_id)?;
    let metadata = ReportMetadata {
        report_id: request.report_id.clone(),
        kind: request.kind.clone(),
        produced_at: produced_at.clone(),
        status: request.status.clone(),
        entrypoint: request.entrypoint.display().to_string(),
        artifacts: request
            .latest_artifacts
            .iter()
            .chain(std::iter::once(&request.metadata_path))
            .map(|path| path.display().to_string())
            .collect(),
    };
    let metadata_json =
        serde_json::to_vec_pretty(&metadata).map_err(OutputError::SerializeMetadata)?;
    let metadata_absolute = root.join(&request.metadata_path);
    if let Some(parent) = metadata_absolute.parent() {
        std::fs::create_dir_all(parent).map_err(|source| OutputError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(&metadata_absolute, metadata_json).map_err(|source| {
        OutputError::WriteMetadata {
            path: metadata_absolute.clone(),
            source,
        }
    })?;

    let mut archived_artifacts = Vec::new();
    if request.archive {
        let archive_root = PathBuf::from(ARCHIVE_ROOT_RELATIVE_PATH)
            .join(&archive_label)
            .join(&request.report_id);
        for artifact in request
            .latest_artifacts
            .iter()
            .chain(std::iter::once(&request.metadata_path))
        {
            let archive_path =
                archive_root.join(artifact.strip_prefix(&latest_report_root).map_err(|_strip_error| {
                    OutputError::InvalidPath {
                        path: artifact.clone(),
                        message: format!(
                            "artifact must remain under {}",
                            latest_report_root.display()
                        ),
                    }
                })?);
            copy_relative_file(root, artifact, &archive_path)?;
            archived_artifacts.push(archive_path);
        }
    }

    Ok(MaterializedReport {
        report_id: request.report_id.clone(),
        kind: request.kind.clone(),
        produced_at,
        status: request.status.clone(),
        entrypoint: request.entrypoint.clone(),
        metadata: request.metadata_path.clone(),
        latest_artifacts: request
            .latest_artifacts
            .iter()
            .cloned()
            .chain(std::iter::once(request.metadata_path.clone()))
            .collect(),
        archived_artifacts,
    })
}

fn latest_report_root(entrypoint: &Path, report_id: &str) -> Result<PathBuf, OutputError> {
    let report_root = PathBuf::from(LATEST_ROOT_RELATIVE_PATH).join(report_id);
    if !entrypoint.starts_with(&report_root) {
        return Err(OutputError::InvalidPath {
            path: entrypoint.to_path_buf(),
            message: format!("entrypoint must remain under {}", report_root.display()),
        });
    }
    Ok(report_root)
}

fn copy_relative_file(root: &Path, from: &Path, to: &Path) -> Result<(), OutputError> {
    let absolute_from = root.join(from);
    let absolute_to = root.join(to);
    if let Some(parent) = absolute_to.parent() {
        std::fs::create_dir_all(parent).map_err(|source| OutputError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::copy(&absolute_from, &absolute_to).map_err(|source| OutputError::CopyFile {
        from: absolute_from,
        to: absolute_to,
        source,
    })?;
    Ok(())
}

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimeFormat(source) => write!(f, "failed to format report timestamp: {source}"),
            Self::InvalidPath { path, message } => {
                write!(
                    f,
                    "invalid report output path {}: {message}",
                    path.display()
                )
            }
            Self::CreateDir { path, source } => {
                write!(
                    f,
                    "failed to create report output dir {}: {source}",
                    path.display()
                )
            }
            Self::CopyFile { from, to, source } => {
                write!(
                    f,
                    "failed to copy report artifact {} to {}: {source}",
                    from.display(),
                    to.display()
                )
            }
            Self::WriteMetadata { path, source } => {
                write!(
                    f,
                    "failed to write report metadata {}: {source}",
                    path.display()
                )
            }
            Self::SerializeMetadata(source) => {
                write!(f, "failed to serialize report metadata: {source}")
            }
        }
    }
}

impl std::error::Error for OutputError {}
