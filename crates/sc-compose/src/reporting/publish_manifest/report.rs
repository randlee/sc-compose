use std::path::Path;

use crate::reporting::index::ReportIndexEntry;

use super::archive::latest_archive_root;
use super::error::{PublishManifestError, missing_required_evidence_error};
use super::files::build_manifest_files;
use super::model::PublishManifestReport;

pub(super) fn should_skip_entry(entry: &ReportIndexEntry) -> bool {
    !entry.required && !entry.missing_paths.is_empty()
}

pub(super) fn build_manifest_report(
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
