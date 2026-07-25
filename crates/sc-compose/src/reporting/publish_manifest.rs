use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::path_utils::normalize_relative_path;
use crate::reporting::index::{ReportIndexEntry, ReportIndexError, build_report_index};
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

pub(crate) fn write_publish_manifest(
    root: &Path,
) -> Result<PublishManifestResult, PublishManifestError> {
    let index = build_report_index(root)
        .map_err(Box::new)
        .map_err(PublishManifestError::Index)?;

    let mut reports = Vec::new();
    for entry in index.entries {
        if should_skip_entry(&entry) {
            continue;
        }

        reports.push(build_manifest_report(root, entry)?);
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

fn should_skip_entry(entry: &ReportIndexEntry) -> bool {
    !entry.required && !entry.missing_paths.is_empty()
}

fn build_manifest_report(
    root: &Path,
    entry: ReportIndexEntry,
) -> Result<PublishManifestReport, PublishManifestError> {
    if !entry.missing_paths.is_empty() {
        return Err(missing_required_evidence_error(&entry));
    }

    let files = build_manifest_files(&entry)?;
    let archive_root = latest_archive_root(root, &entry.report_id)?;

    Ok(PublishManifestReport {
        archive_root,
        report_id: entry.report_id,
        kind: entry.kind,
        entrypoint: entry.entrypoint,
        files,
    })
}

fn build_manifest_files(
    entry: &ReportIndexEntry,
) -> Result<Vec<PublishManifestFile>, PublishManifestError> {
    let latest_report_root = PathBuf::from(LATEST_ROOT_RELATIVE_PATH).join(&entry.report_id);
    let publish_root = PathBuf::from("reports").join(&entry.report_id);

    entry
        .artifacts
        .iter()
        .map(|artifact| {
            Ok(PublishManifestFile {
                role: artifact_role(&entry.entrypoint, &entry.metadata, artifact),
                path: artifact.clone(),
                publish_to: artifact_publish_path(
                    &entry.report_id,
                    artifact,
                    &latest_report_root,
                    &publish_root,
                )?,
            })
        })
        .collect()
}

fn artifact_publish_path(
    report_id: &str,
    artifact: &Path,
    latest_report_root: &Path,
    publish_root: &Path,
) -> Result<PathBuf, PublishManifestError> {
    let relative = artifact
        .strip_prefix(latest_report_root)
        .map_err(|error| invalid_artifact_path(report_id, artifact, latest_report_root, error))?;
    let relative = normalize_relative_path(relative).map_err(|message| {
        invalid_artifact_path(report_id, artifact, latest_report_root, message)
    })?;
    Ok(publish_root.join(relative))
}

fn invalid_artifact_path(
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

fn missing_required_evidence_error(entry: &ReportIndexEntry) -> PublishManifestError {
    PublishManifestError::Index(Box::new(ReportIndexError::MissingRequiredEvidence {
        report_id: entry.report_id.clone(),
        missing_paths: entry.missing_paths.clone(),
    }))
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::reporting::index::ReportIndexEntry;

    use super::{
        ARCHIVE_ROOT_RELATIVE_PATH, artifact_publish_path, build_manifest_files,
        latest_archive_root,
    };

    #[test]
    fn artifact_publish_path_rejects_parent_escaped_artifact() {
        let latest_report_root = Path::new("reports/latest").join("smoke");
        let publish_root = Path::new("reports").join("smoke");

        let error = artifact_publish_path(
            "smoke",
            Path::new("reports/latest/../escape.html"),
            &latest_report_root,
            &publish_root,
        )
        .unwrap_err();

        let message = error.to_string();
        assert!(message.contains("invalid publish-manifest artifact path for smoke"));
        assert!(message.contains(&format!(
            "artifact must remain under {}",
            latest_report_root.display()
        )));
    }

    #[test]
    fn build_manifest_files_preserves_roles_and_publish_paths() {
        let entry = ReportIndexEntry {
            report_id: "smoke".to_owned(),
            kind: "smoke".to_owned(),
            required: true,
            status: Some("ok".to_owned()),
            produced_at: Some("2026-07-25T00:00:00Z".to_owned()),
            entrypoint: PathBuf::from("reports/latest/smoke/index.html"),
            metadata: PathBuf::from("reports/latest/smoke/metadata.json"),
            artifacts: vec![
                PathBuf::from("reports/latest/smoke/index.html"),
                PathBuf::from("reports/latest/smoke/metadata.json"),
                PathBuf::from("reports/latest/smoke/panels/chart.html"),
            ],
            missing_paths: Vec::new(),
        };

        let files = build_manifest_files(&entry).expect("manifest files");

        assert_eq!(files.len(), 3);
        assert_eq!(files[0].role, "entrypoint");
        assert_eq!(
            files[0].publish_to,
            PathBuf::from("reports/smoke/index.html")
        );
        assert_eq!(files[1].role, "metadata");
        assert_eq!(
            files[1].publish_to,
            PathBuf::from("reports/smoke/metadata.json")
        );
        assert_eq!(files[2].role, "artifact");
        assert_eq!(
            files[2].publish_to,
            PathBuf::from("reports/smoke/panels/chart.html")
        );
    }

    #[test]
    fn latest_archive_root_selects_lexically_latest_archive_directory() {
        let root = temp_root("publish-manifest-latest-archive-root");
        create_dir(
            &root
                .join(ARCHIVE_ROOT_RELATIVE_PATH)
                .join("2026-07-14T01-00-00Z")
                .join("sc-lint"),
        );
        create_dir(
            &root
                .join(ARCHIVE_ROOT_RELATIVE_PATH)
                .join("2026-07-15T09-00-00Z")
                .join("sc-lint"),
        );

        let archive_root = latest_archive_root(&root, "sc-lint")
            .unwrap()
            .expect("archive root");

        assert_eq!(
            archive_root,
            PathBuf::from(ARCHIVE_ROOT_RELATIVE_PATH)
                .join("2026-07-15T09-00-00Z")
                .join("sc-lint")
        );
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn create_dir(path: &Path) {
        fs::create_dir_all(path).expect("create dir");
    }
}
