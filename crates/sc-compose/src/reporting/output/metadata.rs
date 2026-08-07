use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;

use crate::path_utils::to_forward_slash;

use super::{OutputError, ReportOutputRequest};

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

pub(super) fn timestamp_pair() -> Result<(String, String), OutputError> {
    let now = OffsetDateTime::now_utc();
    let produced_at = now.format(&Rfc3339).map_err(OutputError::TimeFormat)?;
    let archive_label = now
        .format(ARCHIVE_TIMESTAMP_FORMAT)
        .map_err(OutputError::TimeFormat)?;
    Ok((produced_at, archive_label))
}

pub(super) fn build(request: &ReportOutputRequest, produced_at: String) -> ReportMetadata {
    ReportMetadata {
        report_id: request.report_id.clone(),
        kind: request.kind.clone(),
        produced_at,
        status: request.status.clone(),
        entrypoint: to_forward_slash(&request.entrypoint),
        artifacts: request
            .latest_artifacts
            .iter()
            .chain(std::iter::once(&request.metadata_path))
            .map(|path| to_forward_slash(path))
            .collect(),
    }
}

pub(super) fn serialize(metadata: &ReportMetadata) -> Result<Vec<u8>, OutputError> {
    serde_json::to_vec_pretty(metadata).map_err(OutputError::SerializeMetadata)
}
