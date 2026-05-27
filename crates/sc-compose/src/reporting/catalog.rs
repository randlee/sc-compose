use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Serialize, Serializer};
use toml::Value;

use crate::path_utils::is_normalized_relative_path;

pub(crate) const REPORT_CATALOG_RELATIVE_PATH: &str = "reports/catalog/reports.toml";
pub(crate) const REPORTS_LATEST_ROOT_RELATIVE_PATH: &str = "reports/latest";
pub(crate) const REPORTS_ARCHIVE_ROOT_RELATIVE_PATH: &str = "reports/archive";
pub(crate) const REPORT_METADATA_BASENAME: &str = "report.json";

const ALLOWED_REPORT_KINDS: &[&str] = &[
    "lint",
    "test",
    "smoke",
    "diagram",
    "state_machine",
    "sql_query",
    "custom",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportCatalog {
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) catalog_path: PathBuf,
    pub(crate) reports: Vec<ReportDefinition>,
}

impl ReportCatalog {
    pub(crate) fn load(repo_root: &Path) -> Result<Self, CatalogError> {
        load_report_catalog(repo_root)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportDefinition {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) producer: String,
    pub(crate) required: bool,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) entrypoint: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) metadata: PathBuf,
}

#[derive(Debug)]
pub(crate) enum CatalogError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    Invalid(String),
}

pub(crate) fn load_report_catalog(repo_root: &Path) -> Result<ReportCatalog, CatalogError> {
    load_report_catalog_from_path(repo_root, Path::new(REPORT_CATALOG_RELATIVE_PATH))
}

pub(crate) fn load_report_catalog_from_path(
    repo_root: &Path,
    catalog_path: &Path,
) -> Result<ReportCatalog, CatalogError> {
    let catalog_path = resolve_catalog_path(repo_root, catalog_path);
    let contents = std::fs::read_to_string(&catalog_path).map_err(|source| CatalogError::Read {
        path: catalog_path.clone(),
        source,
    })?;
    let document = contents
        .parse::<Value>()
        .map_err(|source| CatalogError::Parse {
            path: catalog_path.clone(),
            source,
        })?;

    let report_entries = document
        .get("report")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CatalogError::Invalid(format!(
                "{REPORT_CATALOG_RELATIVE_PATH} must define one or more [[report]] entries"
            ))
        })?;

    let mut seen_ids = BTreeSet::new();
    let mut reports = Vec::with_capacity(report_entries.len());
    for (index, entry) in report_entries.iter().enumerate() {
        let report = ReportDefinition::from_value(entry, index)?;
        if !seen_ids.insert(report.id.clone()) {
            return Err(CatalogError::Invalid(format!(
                "duplicate report id '{}' in {}",
                report.id, REPORT_CATALOG_RELATIVE_PATH
            )));
        }
        reports.push(report);
    }

    Ok(ReportCatalog {
        catalog_path,
        reports,
    })
}

impl ReportDefinition {
    fn from_value(value: &Value, index: usize) -> Result<Self, CatalogError> {
        let table = value.as_table().ok_or_else(|| {
            CatalogError::Invalid(format!(
                "report entry {index} in {REPORT_CATALOG_RELATIVE_PATH} must be a TOML table"
            ))
        })?;

        let id = string_field(table, index, "id")?;
        let kind = string_field(table, index, "kind")?;
        if !ALLOWED_REPORT_KINDS.contains(&kind.as_str()) {
            return Err(CatalogError::Invalid(format!(
                "report '{id}' uses unsupported kind '{kind}'"
            )));
        }

        let producer = string_field(table, index, "producer")?;
        let required = bool_field(table, index, "required", &id)?;
        let entrypoint = normalized_relative_path(
            &string_field(table, index, "entrypoint")?,
            &id,
            "entrypoint",
        )?;
        let metadata =
            normalized_relative_path(&string_field(table, index, "metadata")?, &id, "metadata")?;

        Ok(Self {
            id,
            kind,
            producer,
            required,
            entrypoint,
            metadata,
        })
    }
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    f,
                    "failed to read report catalog {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    f,
                    "failed to parse report catalog {}: {source}",
                    path.display()
                )
            }
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for CatalogError {}

fn string_field(
    table: &toml::map::Map<String, Value>,
    index: usize,
    field: &str,
) -> Result<String, CatalogError> {
    let Some(value) = table.get(field) else {
        return Err(CatalogError::Invalid(format!(
            "report entry {index} in {REPORT_CATALOG_RELATIVE_PATH} is missing required field '{field}'"
        )));
    };

    value.as_str().map(str::to_owned).ok_or_else(|| {
        CatalogError::Invalid(format!(
            "report entry {index} field '{field}' in {REPORT_CATALOG_RELATIVE_PATH} must be a string"
        ))
    })
}

fn bool_field(
    table: &toml::map::Map<String, Value>,
    index: usize,
    field: &str,
    id: &str,
) -> Result<bool, CatalogError> {
    let Some(value) = table.get(field) else {
        return Err(CatalogError::Invalid(format!(
            "report entry {index} in {REPORT_CATALOG_RELATIVE_PATH} is missing required field '{field}'"
        )));
    };

    value.as_bool().ok_or_else(|| {
        CatalogError::Invalid(format!(
            "report '{id}' field '{field}' must be true or false"
        ))
    })
}

fn normalized_relative_path(
    value: &str,
    report_id: &str,
    field: &str,
) -> Result<PathBuf, CatalogError> {
    let path = PathBuf::from(value);
    if !is_normalized_relative_path(&path) {
        return Err(CatalogError::Invalid(format!(
            "report '{report_id}' field '{field}' must be a normalized relative path"
        )));
    }

    Ok(path)
}

fn resolve_catalog_path(repo_root: &Path, catalog_path: &Path) -> PathBuf {
    if catalog_path.is_absolute() {
        catalog_path.to_path_buf()
    } else {
        repo_root.join(catalog_path)
    }
}

fn serialize_pathbuf_with_forward_slashes<S>(path: &Path, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let normalized = path.to_string_lossy().replace('\\', "/");
    serializer.serialize_str(&normalized)
}
