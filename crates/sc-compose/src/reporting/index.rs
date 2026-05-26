use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::reporting::catalog::{CatalogError, ReportCatalog};
use crate::reporting::output::ReportMetadata;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportIndexEntry {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    pub(crate) required: bool,
    pub(crate) status: Option<String>,
    pub(crate) produced_at: Option<String>,
    pub(crate) entrypoint: PathBuf,
    pub(crate) metadata: PathBuf,
    pub(crate) artifacts: Vec<PathBuf>,
    pub(crate) missing_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportIndex {
    pub(crate) entries: Vec<ReportIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportVerifyResult {
    pub(crate) required_count: usize,
    pub(crate) verified_count: usize,
}

#[derive(Debug)]
pub(crate) enum ReportIndexError {
    Catalog(CatalogError),
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: serde_json::Error,
    },
    MissingRequiredEvidence {
        report_id: String,
        missing_paths: Vec<PathBuf>,
    },
}

pub(crate) fn build_report_index(root: &Path) -> Result<ReportIndex, ReportIndexError> {
    let catalog = ReportCatalog::load(root).map_err(ReportIndexError::Catalog)?;
    let mut entries = Vec::with_capacity(catalog.reports.len());
    for report in catalog.reports {
        entries.push(index_entry(root, report)?);
    }
    Ok(ReportIndex { entries })
}

pub(crate) fn verify_required_reports(root: &Path) -> Result<ReportVerifyResult, ReportIndexError> {
    let index = build_report_index(root)?;
    let required_entries = index.entries.iter().filter(|entry| entry.required);
    let mut required_count = 0;
    let mut verified_count = 0;
    for entry in required_entries {
        required_count += 1;
        if !entry.missing_paths.is_empty() {
            return Err(ReportIndexError::MissingRequiredEvidence {
                report_id: entry.report_id.clone(),
                missing_paths: entry.missing_paths.clone(),
            });
        }
        verified_count += 1;
    }
    Ok(ReportVerifyResult {
        required_count,
        verified_count,
    })
}

fn index_entry(
    root: &Path,
    report: crate::reporting::catalog::ReportDefinition,
) -> Result<ReportIndexEntry, ReportIndexError> {
    let mut missing_paths = Vec::new();
    if !root.join(&report.entrypoint).exists() {
        missing_paths.push(report.entrypoint.clone());
    }
    if !root.join(&report.metadata).exists() {
        missing_paths.push(report.metadata.clone());
    }

    let mut status = None;
    let mut produced_at = None;
    let mut artifacts = Vec::new();
    if missing_paths.is_empty() {
        let metadata = read_metadata(root, &report.metadata)?;
        status = Some(metadata.status);
        produced_at = Some(metadata.produced_at);
        artifacts = metadata.artifacts.into_iter().map(PathBuf::from).collect();
        for artifact in &artifacts {
            if !root.join(artifact).exists() {
                missing_paths.push(artifact.clone());
            }
        }
    }

    Ok(ReportIndexEntry {
        report_id: report.id,
        kind: report.kind,
        required: report.required,
        status,
        produced_at,
        entrypoint: report.entrypoint,
        metadata: report.metadata,
        artifacts,
        missing_paths,
    })
}

fn read_metadata(root: &Path, relative_path: &Path) -> Result<ReportMetadata, ReportIndexError> {
    let path = root.join(relative_path);
    let bytes = std::fs::read(&path).map_err(|source| ReportIndexError::ReadMetadata {
        path: path.clone(),
        source,
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|source| ReportIndexError::ParseMetadata { path, source })
}

impl fmt::Display for ReportIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(source) => write!(f, "{source}"),
            Self::ReadMetadata { path, source } => {
                write!(
                    f,
                    "failed to read report metadata {}: {source}",
                    path.display()
                )
            }
            Self::ParseMetadata { path, source } => {
                write!(
                    f,
                    "failed to parse report metadata {}: {source}",
                    path.display()
                )
            }
            Self::MissingRequiredEvidence {
                report_id,
                missing_paths,
            } => write!(
                f,
                "missing required report evidence for {report_id}: {}",
                missing_paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl std::error::Error for ReportIndexError {}
