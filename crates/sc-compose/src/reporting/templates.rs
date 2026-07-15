use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use sc_composer::{
    LoadedTemplateRequest, NamedTemplateAsset, RenderError, RenderedArtifact,
    render_loaded_template,
};
use serde_json::Value;
use toml::Value as TomlValue;

use crate::path_utils::is_normalized_relative_path;
use crate::reporting::catalog::REPORT_CATALOG_RELATIVE_PATH;
use crate::reporting::report_context::{ReportTemplateContext, stable_path_key};

const BASE_REPORT_TEMPLATE_NAME: &str = "base/report.html.j2";
const DIAGRAM_REPORT_TEMPLATE_NAME: &str = "diagram/report.html.j2";
const BASE_REPORT_TEMPLATE_TEXT: &str =
    include_str!("../../assets/reports/templates/base/report.html.j2");
const DIAGRAM_REPORT_TEMPLATE_TEXT: &str =
    include_str!("../../assets/reports/templates/diagram/report.html.j2");

const EVIDENCE_FAMILY_NAMES: &[&str] = &["lint", "test", "smoke"];
const PUBLIC_INTERFACE_FAMILY_NAMES: &[&str] = &["public_api", "cli", "icd"];
const DIAGRAM_FAMILY_NAMES: &[&str] = &["diagram", "state_machine", "sql_query"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedTemplate {
    pub(crate) template_name: String,
    pub(crate) template_text: String,
    pub(crate) supporting_templates: Vec<NamedTemplateAsset>,
    pub(crate) output_extension: String,
    pub(crate) uses_report_context: bool,
}

#[derive(Debug)]
pub(crate) enum TemplateError {
    ReadCatalog {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseCatalog {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
    InvalidCatalog(String),
    InvalidSelector(String),
    UnknownSharedFamily {
        family: String,
        supported: Vec<&'static str>,
    },
    ReadTemplate {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidTemplatePath {
        path: PathBuf,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TemplateSelector {
    Shared { family: String },
    RepoPath(PathBuf),
}

pub(crate) fn resolve_template_selector(
    root: &Path,
    selector: &str,
) -> Result<ResolvedTemplate, TemplateError> {
    match parse_template_selector(selector)? {
        TemplateSelector::Shared { family } => resolve_shared_family(&family),
        TemplateSelector::RepoPath(path) => resolve_repo_template(root, &path, false),
    }
}

pub(crate) fn resolve_template_family(
    root: &Path,
    family: &str,
) -> Result<ResolvedTemplate, TemplateError> {
    let config_path = root.join(REPORT_CATALOG_RELATIVE_PATH);
    let selector = match std::fs::read_to_string(&config_path) {
        Ok(contents) => lookup_family_selector(&config_path, &contents, family)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => TemplateSelector::Shared {
            family: family.to_owned(),
        },
        Err(source) => {
            return Err(TemplateError::ReadCatalog {
                path: config_path,
                source,
            });
        }
    };

    match selector {
        TemplateSelector::Shared { family } => resolve_shared_family(&family),
        TemplateSelector::RepoPath(path) => resolve_repo_template(root, &path, true),
    }
}

pub(crate) fn render_shared_report(
    template: &ResolvedTemplate,
    context: &ReportTemplateContext,
) -> Result<RenderedArtifact, RenderError> {
    render_loaded_template(LoadedTemplateRequest {
        template_name: template.template_name.clone(),
        template_text: template.template_text.clone(),
        context: render_context_map(context),
        supporting_templates: template.supporting_templates.clone(),
    })
}

fn parse_template_selector(selector: &str) -> Result<TemplateSelector, TemplateError> {
    if selector.is_empty() {
        return Err(TemplateError::InvalidSelector(
            "template selector must not be empty".to_owned(),
        ));
    }
    if let Some(family) = selector.strip_prefix("shared:") {
        if family.is_empty() {
            return Err(TemplateError::InvalidSelector(
                "shared template selector must include a family name".to_owned(),
            ));
        }
        return Ok(TemplateSelector::Shared {
            family: family.to_owned(),
        });
    }
    Ok(TemplateSelector::RepoPath(PathBuf::from(selector)))
}

fn lookup_family_selector(
    catalog_path: &Path,
    contents: &str,
    family: &str,
) -> Result<TemplateSelector, TemplateError> {
    let document = contents
        .parse::<TomlValue>()
        .map_err(|source| TemplateError::ParseCatalog {
            path: catalog_path.to_path_buf(),
            source: Box::new(source),
        })?;
    let Some(table) = document
        .get("reporting")
        .and_then(TomlValue::as_table)
        .and_then(|table| table.get("templates"))
        .and_then(TomlValue::as_table)
        .and_then(|table| table.get(family))
        .and_then(TomlValue::as_table)
    else {
        return Ok(TemplateSelector::Shared {
            family: family.to_owned(),
        });
    };

    let source = table.get("source").and_then(TomlValue::as_str);
    let path = table.get("path").and_then(TomlValue::as_str);
    match (source, path) {
        (Some(_), Some(_)) => Err(TemplateError::InvalidCatalog(format!(
            "reporting.templates.{family} must not define both 'source' and 'path'"
        ))),
        (Some(source), None) => parse_template_selector(source),
        (None, Some(path)) => Ok(TemplateSelector::RepoPath(PathBuf::from(path))),
        (None, None) => Err(TemplateError::InvalidCatalog(format!(
            "reporting.templates.{family} must define either 'source' or 'path'"
        ))),
    }
}

fn resolve_shared_family(family: &str) -> Result<ResolvedTemplate, TemplateError> {
    if EVIDENCE_FAMILY_NAMES.contains(&family) || PUBLIC_INTERFACE_FAMILY_NAMES.contains(&family) {
        return Ok(ResolvedTemplate {
            template_name: BASE_REPORT_TEMPLATE_NAME.to_owned(),
            template_text: BASE_REPORT_TEMPLATE_TEXT.to_owned(),
            supporting_templates: supporting_templates(),
            output_extension: "html".to_owned(),
            uses_report_context: true,
        });
    }
    if DIAGRAM_FAMILY_NAMES.contains(&family) {
        return Ok(ResolvedTemplate {
            template_name: DIAGRAM_REPORT_TEMPLATE_NAME.to_owned(),
            template_text: DIAGRAM_REPORT_TEMPLATE_TEXT.to_owned(),
            supporting_templates: supporting_templates(),
            output_extension: "html".to_owned(),
            uses_report_context: true,
        });
    }

    let mut supported = Vec::new();
    supported.extend_from_slice(EVIDENCE_FAMILY_NAMES);
    supported.extend_from_slice(PUBLIC_INTERFACE_FAMILY_NAMES);
    supported.extend_from_slice(DIAGRAM_FAMILY_NAMES);
    Err(TemplateError::UnknownSharedFamily {
        family: family.to_owned(),
        supported,
    })
}

fn resolve_repo_template(
    root: &Path,
    template_path: &Path,
    uses_report_context: bool,
) -> Result<ResolvedTemplate, TemplateError> {
    let normalized = normalized_relative_path(template_path)?;
    let absolute = root.join(&normalized);
    let template_text =
        std::fs::read_to_string(&absolute).map_err(|source| TemplateError::ReadTemplate {
            path: absolute.clone(),
            source,
        })?;
    let output_extension = output_extension_from_name(
        normalized
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| TemplateError::InvalidTemplatePath {
                path: normalized.clone(),
                message: "template path must end in a UTF-8 filename".to_owned(),
            })?,
    )?;
    Ok(ResolvedTemplate {
        template_name: format!("repo-template-{}", normalized_template_key(&normalized)),
        template_text,
        supporting_templates: supporting_templates(),
        output_extension,
        uses_report_context,
    })
}

fn normalized_relative_path(path: &Path) -> Result<PathBuf, TemplateError> {
    if !is_normalized_relative_path(path) {
        return Err(TemplateError::InvalidTemplatePath {
            path: path.to_path_buf(),
            message: "template path must be a normalized relative path".to_owned(),
        });
    }

    Ok(path.to_path_buf())
}

fn output_extension_from_name(file_name: &str) -> Result<String, TemplateError> {
    let stripped = file_name.strip_suffix(".j2").ok_or_else(|| {
        TemplateError::InvalidSelector(format!("template '{file_name}' must end with .j2"))
    })?;
    Ok(Path::new(stripped)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("out")
        .to_owned())
}

fn supporting_templates() -> Vec<NamedTemplateAsset> {
    vec![
        NamedTemplateAsset {
            template_name: BASE_REPORT_TEMPLATE_NAME.to_owned(),
            template_text: BASE_REPORT_TEMPLATE_TEXT.to_owned(),
        },
        NamedTemplateAsset {
            template_name: DIAGRAM_REPORT_TEMPLATE_NAME.to_owned(),
            template_text: DIAGRAM_REPORT_TEMPLATE_TEXT.to_owned(),
        },
    ]
}

fn render_context_map(context: &ReportTemplateContext) -> BTreeMap<String, Value> {
    let primary_panel = context.panels.first();
    let panels = context
        .panels
        .iter()
        .map(|panel| {
            let mut panel_map = serde_json::Map::new();
            panel_map.insert("panel_id".to_owned(), Value::String(panel.panel_id.clone()));
            panel_map.insert("title".to_owned(), Value::String(panel.title.clone()));
            panel_map.insert("body".to_owned(), Value::String(panel.body.clone()));
            panel_map.insert(
                "copy_text".to_owned(),
                Value::String(panel.copy_text.clone()),
            );
            panel_map.insert(
                "copy_json".to_owned(),
                panel.copy_json.clone().map_or(Value::Null, Value::String),
            );
            panel_map.insert(
                "fragment_href".to_owned(),
                panel
                    .fragment_href
                    .clone()
                    .map_or(Value::Null, Value::String),
            );
            Value::Object(panel_map)
        })
        .collect();

    let mut map = BTreeMap::new();
    map.insert("title".to_owned(), Value::String(context.title.clone()));
    map.insert("panels".to_owned(), Value::Array(panels));
    map.insert(
        "panel_body_text".to_owned(),
        primary_panel.map_or(Value::Null, |panel| Value::String(panel.body.clone())),
    );
    map.insert(
        "report_metadata".to_owned(),
        context
            .report_metadata
            .as_ref()
            .map_or(Value::Null, |metadata| serde_json::json!(metadata)),
    );
    map
}

fn normalized_template_key(path: &Path) -> String {
    stable_path_key(path)
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadCatalog { path, source } => {
                write!(
                    f,
                    "failed to read report catalog {}: {source}",
                    path.display()
                )
            }
            Self::ParseCatalog { path, source } => {
                write!(
                    f,
                    "failed to parse report catalog {}: {source}",
                    path.display()
                )
            }
            Self::InvalidCatalog(message) | Self::InvalidSelector(message) => f.write_str(message),
            Self::UnknownSharedFamily { family, supported } => write!(
                f,
                "unknown shared template family '{family}'; supported families: {}",
                supported.join(", ")
            ),
            Self::ReadTemplate { path, source } => {
                write!(
                    f,
                    "failed to read report template {}: {source}",
                    path.display()
                )
            }
            Self::InvalidTemplatePath { path, message } => {
                write!(
                    f,
                    "invalid report template path {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for TemplateError {}
