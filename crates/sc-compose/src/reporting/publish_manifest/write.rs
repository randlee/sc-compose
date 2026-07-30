use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::reporting::index::build_report_index;

use super::error::PublishManifestError;
use super::model::{PUBLISH_MANIFEST_RELATIVE_PATH, PublishManifest, PublishManifestResult};
use super::report::{build_manifest_report, should_skip_entry};

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
