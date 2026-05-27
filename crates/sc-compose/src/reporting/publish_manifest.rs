use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::reporting::index::{ReportIndexError, build_report_index};
use crate::reporting::output::{ARCHIVE_ROOT_RELATIVE_PATH, LATEST_ROOT_RELATIVE_PATH};

pub(crate) const PUBLISH_MANIFEST_RELATIVE_PATH: &str = "reports/latest/publish-manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PublishManifestFile {
    pub(crate) role: String,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) path: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) publish_to: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PublishManifestReport {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) entrypoint: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_opt_path")]
    pub(crate) archive_root: Option<PathBuf>,
    pub(crate) files: Vec<PublishManifestFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PublishManifest {
    pub(crate) generated_at: String,
    pub(crate) reports: Vec<PublishManifestReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PublishManifestResult {
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) manifest_path: PathBuf,
    pub(crate) report_count: usize,
    pub(crate) manifest: PublishManifest,
}

#[derive(Debug)]
pub(crate) enum PublishManifestError {
    Index(ReportIndexError),
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

pub(crate) fn write_publish_manifest(
    root: &Path,
) -> Result<PublishManifestResult, PublishManifestError> {
    let index = build_report_index(root).map_err(PublishManifestError::Index)?;

    let mut reports = Vec::new();
    for entry in index.entries {
        if !entry.missing_paths.is_empty() {
            if entry.required {
                return Err(PublishManifestError::Index(
                    ReportIndexError::MissingRequiredEvidence {
                        report_id: entry.report_id,
                        missing_paths: entry.missing_paths,
                    },
                ));
            }
            continue;
        }

        let latest_report_root = PathBuf::from(LATEST_ROOT_RELATIVE_PATH).join(&entry.report_id);
        let publish_root = PathBuf::from("reports").join(&entry.report_id);
        let mut files = Vec::with_capacity(entry.artifacts.len());
        for artifact in &entry.artifacts {
            let relative = artifact
                .strip_prefix(&latest_report_root)
                .map_err(|error| PublishManifestError::InvalidArtifactPath {
                    report_id: entry.report_id.clone(),
                    path: artifact.clone(),
                    message: format!(
                        "artifact must remain under {}: {error}",
                        latest_report_root.display()
                    ),
                })?;
            files.push(PublishManifestFile {
                role: artifact_role(&entry.entrypoint, &entry.metadata, artifact),
                path: artifact.clone(),
                publish_to: publish_root.join(relative),
            });
        }

        reports.push(PublishManifestReport {
            archive_root: latest_archive_root(root, &entry.report_id)?,
            report_id: entry.report_id,
            kind: entry.kind,
            entrypoint: entry.entrypoint,
            files,
        });
    }

    let manifest = PublishManifest {
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(PublishManifestError::TimeFormat)?,
        reports,
    };
    let manifest_path = PathBuf::from(PUBLISH_MANIFEST_RELATIVE_PATH);
    let manifest_absolute = root.join(&manifest_path);
    if let Some(parent) = manifest_absolute.parent() {
        std::fs::create_dir_all(parent).map_err(|source| PublishManifestError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(PublishManifestError::Serialize)?;
    std::fs::write(&manifest_absolute, bytes).map_err(|source| PublishManifestError::Write {
        path: manifest_absolute.clone(),
        source,
    })?;

    Ok(PublishManifestResult {
        manifest_path,
        report_count: manifest.reports.len(),
        manifest,
    })
}

fn artifact_role(entrypoint: &Path, metadata: &Path, artifact: &Path) -> String {
    if artifact == entrypoint {
        return "entrypoint".to_owned();
    }
    if artifact == metadata {
        return "metadata".to_owned();
    }
    "artifact".to_owned()
}

fn latest_archive_root(
    root: &Path,
    report_id: &str,
) -> Result<Option<PathBuf>, PublishManifestError> {
    let archive_root = root.join(ARCHIVE_ROOT_RELATIVE_PATH);
    if !archive_root.exists() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(&archive_root).map_err(|source| {
        PublishManifestError::ReadArchiveRoot {
            path: archive_root.clone(),
            source,
        }
    })? {
        let entry = entry.map_err(|source| PublishManifestError::ReadArchiveRoot {
            path: archive_root.clone(),
            source,
        })?;
        let report_root = entry.path().join(report_id);
        if report_root.is_dir() {
            candidates.push(
                PathBuf::from(ARCHIVE_ROOT_RELATIVE_PATH)
                    .join(entry.file_name())
                    .join(report_id),
            );
        }
    }
    candidates.sort();
    Ok(candidates.pop())
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
