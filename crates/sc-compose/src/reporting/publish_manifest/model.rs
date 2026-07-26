use std::path::PathBuf;

use serde::Serialize;

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
