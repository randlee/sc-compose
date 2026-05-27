use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::{CompositionObserver, DiagnosticCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::CommandError;
use crate::reporting::init::ReportsSmokeResult;
use crate::reporting::mermaid::render_mermaid;
use crate::reporting::output::{ReportOutputRequest, write_report_metadata_and_archive};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SpecHeader {
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) renderer_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct ReportSpecMetadata {
    #[serde(default)]
    pub(crate) owners: Vec<String>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) renderer_targets: Vec<String>,
    #[serde(default)]
    pub(crate) sets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateMachineState {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateMachineTransition {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) event: Option<String>,
    pub(crate) guard: Option<String>,
    pub(crate) actor: Option<String>,
    pub(crate) effect: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateMachineNamedValue {
    pub(crate) id: String,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateMachineSpec {
    pub(crate) spec: SpecHeader,
    pub(crate) states: Vec<StateMachineState>,
    #[serde(default)]
    pub(crate) transitions: Vec<StateMachineTransition>,
    #[serde(default)]
    pub(crate) events: Vec<StateMachineNamedValue>,
    #[serde(default)]
    pub(crate) guards: Vec<StateMachineNamedValue>,
    #[serde(default)]
    pub(crate) actors: Vec<StateMachineNamedValue>,
    #[serde(default)]
    pub(crate) effects: Vec<StateMachineNamedValue>,
    #[serde(default)]
    pub(crate) metadata: ReportSpecMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SqlQuerySpec {
    pub(crate) spec: SpecHeader,
    pub(crate) purpose: String,
    pub(crate) tables_read: Vec<String>,
    pub(crate) tables_written: Vec<String>,
    pub(crate) filters: Vec<String>,
    pub(crate) ordering: Vec<String>,
    pub(crate) cardinality: String,
    pub(crate) transactional_assumptions: Vec<String>,
    #[serde(default)]
    pub(crate) metadata: ReportSpecMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RawSqlQuerySpec {
    spec: SpecHeader,
    sql_query: SqlQueryFields,
    #[serde(default)]
    metadata: ReportSpecMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SqlQueryFields {
    purpose: String,
    #[serde(default)]
    tables_read: Vec<String>,
    #[serde(default)]
    tables_written: Vec<String>,
    #[serde(default)]
    filters: Vec<String>,
    #[serde(default)]
    ordering: Vec<String>,
    cardinality: String,
    #[serde(default)]
    transactional_assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ReportSpec {
    StateMachine(StateMachineSpec),
    SqlQuery(SqlQuerySpec),
}

pub(crate) type ReportsRenderSpecResult = ReportsSmokeResult;

#[derive(Debug)]
pub(crate) enum ReportSpecError {
    ParseToml(toml::de::Error),
    UnsupportedKind(String),
    SerializeJson(serde_json::Error),
}

pub(crate) fn parse_report_spec(raw_source: &str) -> Result<Option<ReportSpec>, ReportSpecError> {
    let document = raw_source
        .parse::<toml::Value>()
        .map_err(ReportSpecError::ParseToml)?;
    let Some(spec_table) = document.get("spec").and_then(toml::Value::as_table) else {
        return Ok(None);
    };
    let Some(kind) = spec_table.get("kind").and_then(toml::Value::as_str) else {
        return Ok(None);
    };

    match kind {
        "state_machine" => toml::from_str::<StateMachineSpec>(raw_source)
            .map(ReportSpec::StateMachine)
            .map(Some)
            .map_err(ReportSpecError::ParseToml),
        "sql_query" => toml::from_str::<RawSqlQuerySpec>(raw_source)
            .map(|raw| {
                ReportSpec::SqlQuery(SqlQuerySpec {
                    spec: raw.spec,
                    purpose: raw.sql_query.purpose,
                    tables_read: raw.sql_query.tables_read,
                    tables_written: raw.sql_query.tables_written,
                    filters: raw.sql_query.filters,
                    ordering: raw.sql_query.ordering,
                    cardinality: raw.sql_query.cardinality,
                    transactional_assumptions: raw.sql_query.transactional_assumptions,
                    metadata: raw.metadata,
                })
            })
            .map(Some)
            .map_err(ReportSpecError::ParseToml),
        other => Err(ReportSpecError::UnsupportedKind(other.to_owned())),
    }
}

pub(crate) fn run_render_spec_report(
    root: &Path,
    spec_path: &Path,
    archive: bool,
    _observer: &mut dyn CompositionObserver,
) -> Result<ReportsRenderSpecResult, CommandError> {
    let workspace_root = std::fs::canonicalize(root).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to canonicalize workspace root {}",
                root.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let absolute_spec_path = resolve_relative_path(&workspace_root, spec_path)?;
    let raw_source = std::fs::read_to_string(&absolute_spec_path).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to read semantic spec {}",
                absolute_spec_path.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let Some(spec) = parse_report_spec(&raw_source).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to parse semantic spec {}",
                absolute_spec_path.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?
    else {
        return Err(CommandError::usage_with_code(
            anyhow!("semantic spec must define [spec] with supported kind"),
            DiagnosticCode::ErrConfigParse,
        ));
    };

    let mermaid = render_mermaid(&spec).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to render Mermaid from {}",
                absolute_spec_path.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let entrypoint = PathBuf::from("reports")
        .join("latest")
        .join(spec.report_id())
        .join("index.mmd");
    let absolute_output = workspace_root.join(&entrypoint);
    if let Some(parent) = absolute_output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!(
                    "failed to create semantic spec output dir {}",
                    parent.display()
                )),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
    }
    std::fs::write(&absolute_output, &mermaid).map_err(|error| {
        CommandError::render_write(
            anyhow!(error).context(format!("failed to write {}", absolute_output.display())),
        )
    })?;

    let materialized = write_report_metadata_and_archive(
        &workspace_root,
        &ReportOutputRequest {
            report_id: spec.report_id().to_owned(),
            kind: spec.kind_name().to_owned(),
            status: "pass".to_owned(),
            entrypoint: entrypoint.clone(),
            metadata_path: PathBuf::from("reports")
                .join("latest")
                .join(spec.report_id())
                .join("report.json"),
            latest_artifacts: vec![entrypoint],
            archive,
        },
    )
    .map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context("failed to materialize semantic spec outputs"),
            DiagnosticCode::ErrConfigParse,
        )
    })?;

    Ok(ReportsRenderSpecResult {
        report_id: materialized.report_id,
        kind: materialized.kind,
        produced_at: materialized.produced_at,
        status: materialized.status,
        entrypoint: materialized.entrypoint,
        metadata: materialized.metadata,
        artifacts: materialized.latest_artifacts,
        archived_artifacts: materialized.archived_artifacts,
        warnings: Vec::new(),
    })
}

impl ReportSpec {
    pub(crate) fn report_id(&self) -> &str {
        match self {
            Self::StateMachine(spec) => &spec.spec.id,
            Self::SqlQuery(spec) => &spec.spec.id,
        }
    }

    pub(crate) fn title(&self) -> &str {
        match self {
            Self::StateMachine(spec) => &spec.spec.title,
            Self::SqlQuery(spec) => &spec.spec.title,
        }
    }

    pub(crate) fn kind_name(&self) -> &'static str {
        match self {
            Self::StateMachine(_) => "state_machine",
            Self::SqlQuery(_) => "sql_query",
        }
    }

    pub(crate) fn renderer_targets(&self) -> Vec<String> {
        match self {
            Self::StateMachine(spec) => merge_renderer_targets(&spec.spec, &spec.metadata),
            Self::SqlQuery(spec) => merge_renderer_targets(&spec.spec, &spec.metadata),
        }
    }

    pub(crate) fn report_metadata(&self) -> Result<BTreeMap<String, Value>, ReportSpecError> {
        let mut metadata = BTreeMap::new();
        metadata.insert("title".to_owned(), Value::String(self.title().to_owned()));
        metadata.insert(
            "spec_kind".to_owned(),
            Value::String(self.kind_name().to_owned()),
        );
        metadata.insert(
            "spec_id".to_owned(),
            Value::String(self.report_id().to_owned()),
        );
        metadata.insert(
            "renderer_targets".to_owned(),
            serde_json::to_value(self.renderer_targets())
                .map_err(ReportSpecError::SerializeJson)?,
        );
        let spec_json = serde_json::to_value(self).map_err(ReportSpecError::SerializeJson)?;
        metadata.insert("copy_json".to_owned(), spec_json.clone());
        metadata.insert("spec".to_owned(), spec_json);
        Ok(metadata)
    }

    pub(crate) fn sets(&self) -> Option<Vec<String>> {
        let sets = match self {
            Self::StateMachine(spec) => &spec.metadata.sets,
            Self::SqlQuery(spec) => &spec.metadata.sets,
        };
        if sets.is_empty() {
            None
        } else {
            Some(sets.clone())
        }
    }
}

impl fmt::Display for ReportSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseToml(source) => write!(f, "failed to parse semantic spec TOML: {source}"),
            Self::UnsupportedKind(kind) => {
                write!(f, "unsupported semantic spec kind '{kind}'")
            }
            Self::SerializeJson(source) => {
                write!(f, "failed to serialize semantic spec JSON: {source}")
            }
        }
    }
}

impl std::error::Error for ReportSpecError {}

fn resolve_relative_path(root: &Path, relative: &Path) -> Result<PathBuf, CommandError> {
    let candidate = root.join(relative);
    std::fs::canonicalize(&candidate).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!("failed to canonicalize {}", candidate.display())),
            DiagnosticCode::ErrConfigParse,
        )
    })
}

fn merge_renderer_targets(spec: &SpecHeader, metadata: &ReportSpecMetadata) -> Vec<String> {
    if !metadata.renderer_targets.is_empty() {
        return metadata.renderer_targets.clone();
    }
    spec.renderer_targets.clone()
}
