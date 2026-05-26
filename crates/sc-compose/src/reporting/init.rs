use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::{
    ComposeMode, ComposePolicy, ComposeRequest, CompositionObserver, ConfiningRoot, Diagnostic,
    DiagnosticCode, InputValue, VariableName, compose_with_observer, input_value_from_yaml,
    validate_input_value,
};
use serde::Serialize;

use crate::CommandError;
use crate::reporting::output::{
    MaterializedReport, ReportOutputRequest, write_report_metadata_and_archive,
};

pub(crate) const STARTER_CATALOG_RELATIVE_PATH: &str = "reports/catalog/reports.toml";
pub(crate) const STARTER_LATEST_RELATIVE_PATH: &str = "reports/latest";
pub(crate) const STARTER_ARCHIVE_RELATIVE_PATH: &str = "reports/archive";
pub(crate) const STARTER_TEMPLATES_RELATIVE_PATH: &str = "reports/templates";
pub(crate) const STARTER_SMOKE_DIR_RELATIVE_PATH: &str = "reports/smoke";
pub(crate) const STARTER_SMOKE_FIXTURE_RELATIVE_PATH: &str =
    "reports/smoke/reference-template.html.j2";
pub(crate) const STARTER_SMOKE_VARS_RELATIVE_PATH: &str = "reports/smoke/sample-vars.json";
pub(crate) const SMOKE_ENTRYPOINT_RELATIVE_PATH: &str = "reports/latest/smoke/index.html";
pub(crate) const SMOKE_METADATA_RELATIVE_PATH: &str = "reports/latest/smoke/report.json";

const STARTER_REPORTS_TOML: &str = r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#;

const STARTER_SMOKE_TEMPLATE: &str = r#"---
required_variables:
  - title
  - summary
---
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>{{ title }}</title>
</head>
<body>
  <main>
    <h1>{{ title }}</h1>
    <p>{{ summary }}</p>
  </main>
</body>
</html>
"#;

const STARTER_SMOKE_VARS: &str = r#"{
  "title": "Smoke Report",
  "summary": "Shared reporting scaffold smoke test."
}
"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportsInitResult {
    pub(crate) workspace_root: PathBuf,
    pub(crate) created_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReportsSmokeResult {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    pub(crate) produced_at: String,
    pub(crate) status: String,
    pub(crate) entrypoint: PathBuf,
    pub(crate) metadata: PathBuf,
    pub(crate) artifacts: Vec<PathBuf>,
    pub(crate) archived_artifacts: Vec<PathBuf>,
    pub(crate) warnings: Vec<Diagnostic>,
}

pub(crate) fn init_report_scaffold(root: &Path) -> Result<ReportsInitResult, CommandError> {
    let workspace_root = fs::canonicalize(root).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to canonicalize workspace root {}",
                root.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;

    let mut created_paths = Vec::new();
    ensure_dir(
        &workspace_root,
        STARTER_LATEST_RELATIVE_PATH,
        &mut created_paths,
    )?;
    ensure_dir(
        &workspace_root,
        STARTER_ARCHIVE_RELATIVE_PATH,
        &mut created_paths,
    )?;
    ensure_dir(
        &workspace_root,
        STARTER_TEMPLATES_RELATIVE_PATH,
        &mut created_paths,
    )?;
    ensure_dir(
        &workspace_root,
        STARTER_SMOKE_DIR_RELATIVE_PATH,
        &mut created_paths,
    )?;
    write_if_missing(
        &workspace_root,
        STARTER_CATALOG_RELATIVE_PATH,
        STARTER_REPORTS_TOML,
        &mut created_paths,
    )?;
    write_if_missing(
        &workspace_root,
        STARTER_SMOKE_FIXTURE_RELATIVE_PATH,
        STARTER_SMOKE_TEMPLATE,
        &mut created_paths,
    )?;
    write_if_missing(
        &workspace_root,
        STARTER_SMOKE_VARS_RELATIVE_PATH,
        STARTER_SMOKE_VARS,
        &mut created_paths,
    )?;

    Ok(ReportsInitResult {
        workspace_root,
        created_paths,
    })
}

pub(crate) fn run_smoke_report(
    root: &Path,
    fixture: &Path,
    vars: &Path,
    archive: bool,
    observer: &mut dyn CompositionObserver,
) -> Result<ReportsSmokeResult, CommandError> {
    let workspace_root = fs::canonicalize(root).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to canonicalize workspace root {}",
                root.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let fixture_path = resolve_relative_path(&workspace_root, fixture)?;
    let vars_path = resolve_relative_path(&workspace_root, vars)?;
    let template_path = fixture_path
        .strip_prefix(&workspace_root)
        .map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!(
                    "smoke fixture {} must remain under workspace root {}",
                    fixture_path.display(),
                    workspace_root.display()
                )),
                DiagnosticCode::ErrConfigParse,
            )
        })?;

    let request = ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: template_path.to_path_buf(),
        },
        root: ConfiningRoot::new(&workspace_root).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!(
                    "failed to canonicalize workspace root {}",
                    workspace_root.display()
                )),
                DiagnosticCode::ErrConfigParse,
            )
        })?,
        vars_input: load_var_file(&vars_path)?,
        vars_env: BTreeMap::new(),
        vars_defaults: BTreeMap::new(),
        guidance_block: None,
        user_prompt: None,
        policy: ComposePolicy::default(),
    };

    let result = compose_with_observer(&request, observer).map_err(CommandError::compose)?;
    let entrypoint = workspace_root.join(SMOKE_ENTRYPOINT_RELATIVE_PATH);
    if let Some(parent) = entrypoint.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!(
                    "failed to create smoke output dir {}",
                    parent.display()
                )),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
    }
    fs::write(&entrypoint, &result.rendered_text).map_err(|error| {
        CommandError::render_write(
            anyhow!(error).context(format!("failed to write {}", entrypoint.display())),
        )
    })?;

    let artifacts = vec![PathBuf::from(SMOKE_ENTRYPOINT_RELATIVE_PATH)];
    let materialized = write_smoke_outputs(&workspace_root, archive, artifacts)?;

    Ok(ReportsSmokeResult {
        report_id: materialized.report_id,
        kind: materialized.kind,
        produced_at: materialized.produced_at,
        status: materialized.status,
        entrypoint: materialized.entrypoint,
        metadata: materialized.metadata,
        artifacts: materialized.latest_artifacts,
        archived_artifacts: materialized.archived_artifacts,
        warnings: result.warnings,
    })
}

fn write_smoke_outputs(
    workspace_root: &Path,
    archive: bool,
    latest_artifacts: Vec<PathBuf>,
) -> Result<MaterializedReport, CommandError> {
    write_report_metadata_and_archive(
        workspace_root,
        &ReportOutputRequest {
            report_id: "smoke".to_owned(),
            kind: "smoke".to_owned(),
            status: "pass".to_owned(),
            entrypoint: PathBuf::from(SMOKE_ENTRYPOINT_RELATIVE_PATH),
            metadata_path: PathBuf::from(SMOKE_METADATA_RELATIVE_PATH),
            latest_artifacts,
            archive,
        },
    )
    .map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context("failed to materialize smoke report outputs"),
            DiagnosticCode::ErrConfigParse,
        )
    })
}

fn ensure_dir(
    workspace_root: &Path,
    relative: &str,
    created_paths: &mut Vec<String>,
) -> Result<(), CommandError> {
    let path = workspace_root.join(relative);
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!("failed to create {}", path.display())),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
        created_paths.push(format!("{relative}/"));
    }
    Ok(())
}

fn write_if_missing(
    workspace_root: &Path,
    relative: &str,
    contents: &str,
    created_paths: &mut Vec<String>,
) -> Result<(), CommandError> {
    let path = workspace_root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!("failed to create {}", parent.display())),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
    }
    if !path.exists() {
        fs::write(&path, contents).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(error).context(format!("failed to write {}", path.display())),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
        created_paths.push(relative.to_owned());
    }
    Ok(())
}

fn resolve_relative_path(workspace_root: &Path, path: &Path) -> Result<PathBuf, CommandError> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };
    fs::canonicalize(&joined).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!("failed to resolve {}", joined.display())),
            DiagnosticCode::ErrConfigParse,
        )
    })
}

fn load_var_file(path: &Path) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let contents = fs::read_to_string(path).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!("failed to read var-file {}", path.display())),
            DiagnosticCode::ErrConfigParse,
        )
    })?;

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) {
        return parse_object_value(&value);
    }
    let value = serde_yaml::from_str::<serde_yaml::Value>(&contents).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context("var-file must be valid JSON or YAML"),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let serde_yaml::Value::Mapping(object) = value else {
        return Err(CommandError::usage_with_code(
            anyhow!("var-file must be a JSON or YAML object"),
            DiagnosticCode::ErrConfigVarfile,
        ));
    };

    let mut vars = BTreeMap::new();
    for (key, value) in object {
        let key = key.as_str().ok_or_else(|| {
            CommandError::usage_with_code(
                anyhow!("var-file keys must be strings"),
                DiagnosticCode::ErrConfigVarfile,
            )
        })?;
        vars.insert(
            VariableName::new(key.to_owned()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file key `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
            input_value_from_yaml(value).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file value for `{key}`: {error}"),
                    error.code(),
                )
            })?,
        );
    }
    Ok(vars)
}

fn parse_object_value(
    value: &serde_json::Value,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let object = value.as_object().ok_or_else(|| {
        CommandError::usage_with_code(
            anyhow!("var-file must be a JSON object"),
            DiagnosticCode::ErrConfigVarfile,
        )
    })?;
    let mut vars = BTreeMap::new();
    for (key, value) in object {
        vars.insert(
            VariableName::new(key.clone()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file key `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
            {
                validate_input_value(value).map_err(|error| {
                    CommandError::usage_with_code(
                        anyhow!("invalid var-file value for `{key}`: {error}"),
                        error.code(),
                    )
                })?;
                value.clone()
            },
        );
    }
    Ok(vars)
}
