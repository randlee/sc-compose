use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

type ParsedMetadata = (BTreeMap<String, Value>, Option<Vec<String>>, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceEntry {
    pub(crate) record: SourceEntryRecord,
    pub(crate) raw_source: String,
    pub(crate) body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SourceEntryRecord {
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) source_path: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) output_path: PathBuf,
    pub(crate) metadata: BTreeMap<String, Value>,
    pub(crate) sets: Option<Vec<String>>,
}

#[derive(Debug)]
pub(crate) enum SourceEntryError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidMetadata {
        path: PathBuf,
        message: String,
    },
}

impl SourceEntry {
    pub(crate) fn load(
        absolute_source_path: &Path,
        source_path: &Path,
        output_path: PathBuf,
    ) -> Result<Self, SourceEntryError> {
        let raw_source = std::fs::read_to_string(absolute_source_path).map_err(|source| {
            SourceEntryError::Read {
                path: source_path.to_path_buf(),
                source,
            }
        })?;
        let (metadata, sets, body) = parse_metadata(source_path, &raw_source)?;

        Ok(Self {
            record: SourceEntryRecord {
                source_path: source_path.to_path_buf(),
                output_path,
                metadata,
                sets,
            },
            raw_source,
            body,
        })
    }
}

impl fmt::Display for SourceEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    f,
                    "failed to read source entry {}: {source}",
                    path.display()
                )
            }
            Self::InvalidMetadata { path, message } => {
                write!(f, "invalid metadata in {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for SourceEntryError {}

fn parse_metadata(
    source_path: &Path,
    raw_source: &str,
) -> Result<ParsedMetadata, SourceEntryError> {
    if let Some((metadata_text, body)) = parse_block_comment_metadata(raw_source) {
        return finalize_metadata(source_path, metadata_text, body);
    }
    if let Some((metadata_text, body)) = parse_comment_prefix_metadata(raw_source) {
        return finalize_metadata(source_path, metadata_text, body);
    }
    Ok((BTreeMap::new(), None, raw_source.to_owned()))
}

fn parse_block_comment_metadata(raw_source: &str) -> Option<(&str, String)> {
    let trimmed = raw_source.trim_start_matches(['\n', '\r', ' ', '\t']);
    let prefix_len = raw_source.len().checked_sub(trimmed.len())?;
    let after_prefix = &raw_source[prefix_len..];
    let block = after_prefix.strip_prefix("/*")?;
    let end = block.find("*/")?;
    let metadata_text = &block[..end];
    let body = block[end + 2..].trim_start_matches(['\n', '\r']).to_owned();
    Some((metadata_text, body))
}

fn parse_comment_prefix_metadata(raw_source: &str) -> Option<(String, String)> {
    let mut metadata_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_metadata = true;

    for line in raw_source.lines() {
        if in_metadata {
            if let Some(stripped) = strip_comment_prefix(line) {
                metadata_lines.push(stripped.to_owned());
                continue;
            }
            in_metadata = false;
        }
        body_lines.push(line.to_owned());
    }

    if metadata_lines.is_empty() {
        None
    } else {
        Some((metadata_lines.join("\n"), body_lines.join("\n")))
    }
}

fn strip_comment_prefix(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for prefix in ["#", "//", "--", ";"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest.strip_prefix(' ').unwrap_or(rest));
        }
    }
    None
}

fn finalize_metadata(
    source_path: &Path,
    metadata_text: impl AsRef<str>,
    body: String,
) -> Result<ParsedMetadata, SourceEntryError> {
    let metadata_text = metadata_text.as_ref().trim();
    if metadata_text.is_empty() {
        return Ok((BTreeMap::new(), None, body));
    }

    let value = serde_yaml::from_str::<serde_yaml::Value>(metadata_text).map_err(|error| {
        SourceEntryError::InvalidMetadata {
            path: source_path.to_path_buf(),
            message: error.to_string(),
        }
    })?;
    let serde_yaml::Value::Mapping(mapping) = value else {
        return Err(SourceEntryError::InvalidMetadata {
            path: source_path.to_path_buf(),
            message: "metadata block must be a YAML object".to_owned(),
        });
    };

    let mut metadata = BTreeMap::new();
    let mut sets = None;
    for (key, value) in mapping {
        let key = key
            .as_str()
            .ok_or_else(|| SourceEntryError::InvalidMetadata {
                path: source_path.to_path_buf(),
                message: "metadata keys must be strings".to_owned(),
            })?;
        if key == "sets" {
            let serde_yaml::Value::Sequence(sequence) = value else {
                return Err(SourceEntryError::InvalidMetadata {
                    path: source_path.to_path_buf(),
                    message: "metadata field 'sets' must be a YAML sequence of strings".to_owned(),
                });
            };
            let parsed_sets = sequence
                .into_iter()
                .map(|item| {
                    item.as_str().map(str::to_owned).ok_or_else(|| {
                        SourceEntryError::InvalidMetadata {
                            path: source_path.to_path_buf(),
                            message: "metadata field 'sets' must contain only strings".to_owned(),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            sets = Some(parsed_sets);
            continue;
        }

        let json_value =
            serde_json::to_value(value).map_err(|error| SourceEntryError::InvalidMetadata {
                path: source_path.to_path_buf(),
                message: error.to_string(),
            })?;
        metadata.insert(key.to_owned(), json_value);
    }

    Ok((metadata, sets, body))
}
