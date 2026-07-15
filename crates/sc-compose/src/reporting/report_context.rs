use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::path_utils::to_forward_slash;
use crate::reporting::source_entry::SourceEntry;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportPanel {
    pub(crate) panel_id: String,
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) copy_text: String,
    pub(crate) copy_json: Option<String>,
    pub(crate) fragment_href: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportTemplateContext {
    pub(crate) title: String,
    pub(crate) panels: Vec<ReportPanel>,
    pub(crate) report_metadata: Option<BTreeMap<String, Value>>,
}

pub(crate) fn context_from_source_entry(
    entry: &SourceEntry,
    report_title: Option<String>,
) -> ReportTemplateContext {
    let title = report_title.unwrap_or_else(|| entry_title(entry));
    let panel_title = entry_title(entry);
    let mut report_metadata = BTreeMap::new();
    report_metadata.insert(
        "source_path".to_owned(),
        Value::String(to_forward_slash(&entry.record.source_path)),
    );
    report_metadata.insert(
        "output_path".to_owned(),
        Value::String(to_forward_slash(&entry.record.output_path)),
    );
    if let Some(sets) = entry.record.sets.clone() {
        report_metadata.insert("sets".to_owned(), serde_json::json!(sets));
    }

    ReportTemplateContext {
        title,
        panels: vec![ReportPanel {
            panel_id: stable_path_key(&entry.record.source_path),
            title: panel_title,
            body: entry.body.clone(),
            copy_text: entry.body.clone(),
            copy_json: panel_copy_json(entry),
            fragment_href: panel_fragment_href(entry),
        }],
        report_metadata: Some(report_metadata),
    }
}

pub(crate) fn entry_title(entry: &SourceEntry) -> String {
    entry
        .record
        .metadata
        .get("title")
        .and_then(Value::as_str)
        .map_or_else(
            || {
                entry
                    .record
                    .source_path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("report-panel")
                    .to_owned()
            },
            str::to_owned,
        )
}

pub(crate) fn stable_path_key(path: &Path) -> String {
    to_forward_slash(path)
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '-',
        })
        .collect()
}

fn panel_copy_json(entry: &SourceEntry) -> Option<String> {
    entry
        .record
        .metadata
        .get("copy_json")
        .map(|value| serde_json::to_string(value).unwrap_or_else(|_| String::from("null")))
}

fn panel_fragment_href(entry: &SourceEntry) -> Option<String> {
    entry
        .record
        .metadata
        .get("fragment_href")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            entry
                .record
                .metadata
                .get("fragment")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use serde_json::Value;

    use super::{context_from_source_entry, stable_path_key};
    use crate::reporting::source_entry::{SourceEntry, SourceEntryRecord};

    #[test]
    fn stable_path_key_normalizes_separator_styles() {
        assert_eq!(
            stable_path_key(Path::new("reports/inputs/lint/whitespace.md")),
            stable_path_key(Path::new(r"reports\inputs\lint\whitespace.md"))
        );
    }

    #[test]
    fn context_from_source_entry_uses_stable_panel_ids() {
        let entry = SourceEntry {
            record: SourceEntryRecord {
                source_path: PathBuf::from(r"reports\inputs\lint\whitespace.md"),
                output_path: PathBuf::from("reports/latest/sc-lint/panels/whitespace.html"),
                metadata: BTreeMap::from([(
                    "title".to_owned(),
                    Value::String("Whitespace".to_owned()),
                )]),
                sets: None,
            },
            raw_source: "body".to_owned(),
            body: "body".to_owned(),
        };

        let context = context_from_source_entry(&entry, None);
        assert_eq!(
            context.panels[0].panel_id,
            "reports-inputs-lint-whitespace-md"
        );
    }
}
