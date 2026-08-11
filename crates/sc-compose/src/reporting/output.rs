use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;

mod layout;
mod materialization;
mod metadata;

pub(crate) use layout::{ARCHIVE_ROOT_RELATIVE_PATH, LATEST_ROOT_RELATIVE_PATH};
pub(crate) use metadata::ReportMetadata;

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
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) entrypoint: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) metadata: PathBuf,
    #[serde(rename = "artifacts")]
    #[serde(serialize_with = "crate::path_utils::serialize_paths")]
    pub(crate) latest_artifacts: Vec<PathBuf>,
    #[serde(serialize_with = "crate::path_utils::serialize_paths")]
    pub(crate) archived_artifacts: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FinalizeReportRequest {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) entrypoint: PathBuf,
    pub(crate) artifacts: Vec<PathBuf>,
    pub(crate) archive: bool,
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
    let (produced_at, archive_label) = metadata::timestamp_pair()?;
    let latest_report_root = layout::latest_report_root(&request.entrypoint, &request.report_id)?;
    let metadata = metadata::build(request, produced_at.clone());
    materialization::write_metadata(
        root,
        &request.metadata_path,
        metadata::serialize(&metadata)?,
    )?;
    let archived_artifacts = if request.archive {
        materialization::archive_artifacts(
            root,
            &latest_report_root,
            &layout::archive_root(&archive_label, &request.report_id),
            request
                .latest_artifacts
                .iter()
                .chain(std::iter::once(&request.metadata_path))
                .map(PathBuf::as_path),
        )?
    } else {
        Vec::new()
    };

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

pub(crate) fn finalize_report_outputs(
    root: &Path,
    request: &FinalizeReportRequest,
) -> Result<MaterializedReport, OutputError> {
    let latest_report_root = layout::latest_report_root(&request.entrypoint, &request.report_id)?;
    let mut latest_artifacts = Vec::new();
    latest_artifacts.push(request.entrypoint.clone());
    for artifact in &request.artifacts {
        if artifact == &request.entrypoint {
            continue;
        }
        if !artifact.starts_with(&latest_report_root) {
            return Err(OutputError::InvalidPath {
                path: artifact.clone(),
                message: format!(
                    "artifact must remain under {}",
                    latest_report_root.display()
                ),
            });
        }
        latest_artifacts.push(artifact.clone());
    }

    write_report_metadata_and_archive(
        root,
        &ReportOutputRequest {
            report_id: request.report_id.clone(),
            kind: request.kind.clone(),
            status: request.status.clone(),
            entrypoint: request.entrypoint.clone(),
            metadata_path: latest_report_root.join("report.json"),
            latest_artifacts,
            archive: request.archive,
        },
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        FinalizeReportRequest, OutputError, ReportOutputRequest, write_report_metadata_and_archive,
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn temp_root(label: &str) -> PathBuf {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "sc-compose-report-output-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn latest_request(archive: bool) -> ReportOutputRequest {
        ReportOutputRequest {
            report_id: "demo".to_owned(),
            kind: "smoke".to_owned(),
            status: "complete".to_owned(),
            entrypoint: PathBuf::from("reports/latest/demo/index.html"),
            metadata_path: PathBuf::from("reports/latest/demo/report.json"),
            latest_artifacts: vec![
                PathBuf::from("reports/latest/demo/index.html"),
                PathBuf::from("reports/latest/demo/assets/style.css"),
            ],
            archive,
        }
    }

    fn write_file(root: &Path, relative: &Path, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn latest_and_archive_materialization_preserves_layout_and_order() {
        let root = temp_root("layout");
        let request = latest_request(true);
        write_file(&root, &request.latest_artifacts[0], "<html>report</html>");
        write_file(&root, &request.latest_artifacts[1], "body{}");

        let materialized = write_report_metadata_and_archive(&root, &request).unwrap();

        assert_eq!(
            materialized.latest_artifacts,
            vec![
                PathBuf::from("reports/latest/demo/index.html"),
                PathBuf::from("reports/latest/demo/assets/style.css"),
                PathBuf::from("reports/latest/demo/report.json"),
            ]
        );
        assert_eq!(materialized.archived_artifacts.len(), 3);
        assert!(
            materialized
                .archived_artifacts
                .iter()
                .all(|path| path.starts_with("reports/archive/"))
        );
        assert_eq!(
            fs::read_to_string(root.join(&materialized.archived_artifacts[0])).unwrap(),
            "<html>report</html>"
        );
        assert_eq!(
            fs::read_to_string(root.join(&materialized.archived_artifacts[1])).unwrap(),
            "body{}"
        );

        let metadata: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("reports/latest/demo/report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(metadata["report_id"], "demo");
        assert_eq!(metadata["kind"], "smoke");
        assert_eq!(metadata["status"], "complete");
        assert_eq!(metadata["entrypoint"], "reports/latest/demo/index.html");
        assert_eq!(
            metadata["artifacts"],
            serde_json::json!([
                "reports/latest/demo/index.html",
                "reports/latest/demo/assets/style.css",
                "reports/latest/demo/report.json"
            ])
        );
        assert!(metadata["produced_at"].as_str().unwrap().contains('T'));
    }

    #[test]
    fn finalize_rejects_outside_artifacts_and_keeps_entrypoint_first() {
        let root = temp_root("containment");
        let request = FinalizeReportRequest {
            report_id: "demo".to_owned(),
            kind: "smoke".to_owned(),
            status: "complete".to_owned(),
            entrypoint: PathBuf::from("reports/latest/demo/index.html"),
            artifacts: vec![PathBuf::from("outside.txt")],
            archive: false,
        };
        assert!(matches!(
            super::finalize_report_outputs(&root, &request),
            Err(OutputError::InvalidPath { .. })
        ));

        let valid = FinalizeReportRequest {
            artifacts: vec![
                request.entrypoint.clone(),
                PathBuf::from("reports/latest/demo/second.html"),
            ],
            ..request
        };
        write_file(&root, &valid.entrypoint, "entrypoint");
        write_file(&root, &valid.artifacts[1], "second");
        let materialized = super::finalize_report_outputs(&root, &valid).unwrap();
        assert_eq!(
            materialized.latest_artifacts,
            vec![
                PathBuf::from("reports/latest/demo/index.html"),
                PathBuf::from("reports/latest/demo/second.html"),
                PathBuf::from("reports/latest/demo/report.json"),
            ]
        );
    }

    #[test]
    fn metadata_write_overwrites_previous_status_and_content() {
        let root = temp_root("overwrite");
        let mut request = latest_request(false);
        write_file(&root, &request.latest_artifacts[0], "v1");
        write_file(&root, &request.latest_artifacts[1], "style-v1");
        write_report_metadata_and_archive(&root, &request).unwrap();

        request.status = "replaced".to_owned();
        fs::write(root.join(&request.latest_artifacts[0]), "v2").unwrap();
        write_report_metadata_and_archive(&root, &request).unwrap();

        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(&request.metadata_path)).unwrap()).unwrap();
        assert_eq!(metadata["status"], "replaced");
        assert_eq!(metadata["artifacts"][0], "reports/latest/demo/index.html");
    }

    #[test]
    fn filesystem_failures_preserve_output_error_categories() {
        let missing_root = temp_root("copy-error");
        let mut missing = latest_request(true);
        missing.latest_artifacts[1] = PathBuf::from("reports/latest/demo/missing.css");
        write_file(&missing_root, &missing.latest_artifacts[0], "entrypoint");
        assert!(matches!(
            write_report_metadata_and_archive(&missing_root, &missing),
            Err(OutputError::CopyFile { .. })
        ));

        let create_root = temp_root("create-error");
        fs::write(create_root.join("reports"), "not-a-directory").unwrap();
        let create_request = latest_request(false);
        assert!(matches!(
            write_report_metadata_and_archive(&create_root, &create_request),
            Err(OutputError::CreateDir { .. })
        ));

        let write_root = temp_root("write-error");
        let write_request = latest_request(false);
        write_file(
            &write_root,
            &write_request.latest_artifacts[0],
            "entrypoint",
        );
        write_file(&write_root, &write_request.latest_artifacts[1], "style");
        fs::create_dir_all(write_root.join("reports/latest/demo/report.json")).unwrap();
        assert!(matches!(
            write_report_metadata_and_archive(&write_root, &write_request),
            Err(OutputError::WriteMetadata { .. })
        ));
    }

    #[test]
    fn output_error_display_covers_all_error_families() {
        let serialize_error = serde_json::from_str::<serde_json::Value>("not-json").unwrap_err();
        let format_error = time::Date::MIN
            .format(&time::macros::format_description!("[hour]"))
            .unwrap_err();
        let errors = [
            OutputError::TimeFormat(format_error),
            OutputError::InvalidPath {
                path: PathBuf::from("bad"),
                message: "outside root".to_owned(),
            },
            OutputError::CreateDir {
                path: PathBuf::from("dir"),
                source: std::io::Error::other("create"),
            },
            OutputError::CopyFile {
                from: PathBuf::from("from"),
                to: PathBuf::from("to"),
                source: std::io::Error::other("copy"),
            },
            OutputError::WriteMetadata {
                path: PathBuf::from("metadata"),
                source: std::io::Error::other("write"),
            },
            OutputError::SerializeMetadata(serialize_error),
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}
