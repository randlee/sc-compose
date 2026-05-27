use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use glob::glob;
use sc_composer::{LoadedTemplateRequest, RenderError, RenderedArtifact, render_loaded_template};
use serde::Serialize;
use serde_json::Value;

use crate::path_utils::to_forward_slash;
use crate::reporting::source_entry::{SourceEntry, SourceEntryError};
use crate::reporting::templates::{
    ResolvedTemplate, TemplateError, context_from_source_entry, entry_title, render_shared_report,
    resolve_template_family, resolve_template_selector,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceSetDefinition {
    pub(crate) id: String,
    pub(crate) glob: String,
    pub(crate) template_selector: String,
    pub(crate) template_family: Option<String>,
    pub(crate) output_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderManyRequest {
    pub(crate) root: PathBuf,
    pub(crate) source_set: SourceSetDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RenderManyManifestEntry {
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) source_path: PathBuf,
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) output_path: PathBuf,
    pub(crate) metadata: BTreeMap<String, Value>,
    pub(crate) sets: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RenderManyResult {
    #[serde(serialize_with = "crate::path_utils::serialize_path")]
    pub(crate) manifest_path: PathBuf,
    pub(crate) entries: Vec<RenderManyManifestEntry>,
    #[serde(serialize_with = "crate::path_utils::serialize_paths")]
    pub(crate) generated_outputs: Vec<PathBuf>,
}

#[derive(Debug)]
pub(crate) enum RenderManyError {
    InvalidGlob {
        glob: String,
        message: String,
    },
    GlobWalk {
        glob: String,
        message: String,
    },
    InvalidTemplatePath {
        path: PathBuf,
        message: String,
    },
    CreateOutputDir {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteOutput {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteManifest {
        path: PathBuf,
        source: std::io::Error,
    },
    SerializeManifest {
        source: serde_json::Error,
    },
    Render {
        source_path: PathBuf,
        source: RenderError,
    },
    SourceEntry(SourceEntryError),
    Template(TemplateError),
}

impl RenderManyRequest {
    pub(crate) fn output_extension(&self) -> Result<String, RenderManyError> {
        let template = self.resolve_template()?;
        Ok(template.output_extension)
    }

    fn resolve_template(&self) -> Result<ResolvedTemplate, RenderManyError> {
        if let Some(family) = &self.source_set.template_family {
            return resolve_template_family(&self.root, family).map_err(RenderManyError::Template);
        }
        resolve_template_selector(&self.root, &self.source_set.template_selector)
            .map_err(RenderManyError::Template)
    }
}

pub(crate) fn render_many(
    request: &RenderManyRequest,
) -> Result<RenderManyResult, RenderManyError> {
    let resolved_template = request.resolve_template()?;
    let output_root = request.root.join(&request.source_set.output_dir);
    std::fs::create_dir_all(&output_root).map_err(|source| RenderManyError::CreateOutputDir {
        path: output_root.clone(),
        source,
    })?;

    let mut discovered = discover_sources(request)?;
    discovered.sort_by(|left, right| left.source_path.cmp(&right.source_path));

    let mut entries = Vec::with_capacity(discovered.len());
    let mut generated_outputs = Vec::with_capacity(discovered.len());
    for entry in discovered {
        let rendered =
            render_entry(&entry, &request.source_set.id, &resolved_template).map_err(|source| {
                RenderManyError::Render {
                    source_path: entry.source_path.clone(),
                    source,
                }
            })?;
        let absolute_output = request.root.join(&entry.output_path);
        if let Some(parent) = absolute_output.parent() {
            std::fs::create_dir_all(parent).map_err(|source| RenderManyError::CreateOutputDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(&absolute_output, rendered.rendered).map_err(|source| {
            RenderManyError::WriteOutput {
                path: absolute_output.clone(),
                source,
            }
        })?;
        generated_outputs.push(entry.output_path.clone());
        entries.push(RenderManyManifestEntry {
            source_path: entry.source_path.clone(),
            output_path: entry.output_path.clone(),
            metadata: entry.metadata.clone(),
            sets: entry.sets.clone(),
        });
    }

    let manifest_path = request.source_set.output_dir.join("manifest.json");
    let manifest_bytes = serde_json::to_vec_pretty(&entries)
        .map_err(|source| RenderManyError::SerializeManifest { source })?;
    std::fs::write(request.root.join(&manifest_path), manifest_bytes).map_err(|source| {
        RenderManyError::WriteManifest {
            path: request.root.join(&manifest_path),
            source,
        }
    })?;
    generated_outputs.push(manifest_path.clone());

    Ok(RenderManyResult {
        manifest_path,
        entries,
        generated_outputs,
    })
}

fn discover_sources(request: &RenderManyRequest) -> Result<Vec<SourceEntry>, RenderManyError> {
    let pattern = to_forward_slash(&request.root.join(&request.source_set.glob));
    let paths = glob(&pattern).map_err(|error| RenderManyError::InvalidGlob {
        glob: request.source_set.glob.clone(),
        message: error.to_string(),
    })?;
    let output_extension = request.output_extension()?;

    let mut entries = Vec::new();
    for path in paths {
        let absolute_source = path.map_err(|error| RenderManyError::GlobWalk {
            glob: request.source_set.glob.clone(),
            message: error.to_string(),
        })?;
        let relative_source = absolute_source
            .strip_prefix(&request.root)
            .map_err(|error| RenderManyError::InvalidTemplatePath {
                path: absolute_source.clone(),
                message: error.to_string(),
            })?;
        let output_path = derive_output_path(
            &request.source_set.output_dir,
            relative_source,
            &output_extension,
        );
        entries.push(
            SourceEntry::load(&absolute_source, relative_source, output_path)
                .map_err(RenderManyError::SourceEntry)?,
        );
    }
    Ok(entries)
}

fn derive_output_path(output_dir: &Path, source_path: &Path, extension: &str) -> PathBuf {
    let mut output_path = output_dir.join(source_path);
    output_path.set_extension(extension);
    output_path
}

fn render_entry(
    entry: &SourceEntry,
    template_name: &str,
    template: &ResolvedTemplate,
) -> Result<RenderedArtifact, RenderError> {
    if template.uses_report_context {
        return render_shared_report(
            template,
            &context_from_source_entry(entry, Some(entry_title(entry))),
        );
    }

    let mut context = BTreeMap::new();
    context.insert(
        "source_path".to_owned(),
        Value::String(to_forward_slash(&entry.source_path)),
    );
    context.insert(
        "output_path".to_owned(),
        Value::String(to_forward_slash(&entry.output_path)),
    );
    context.insert("metadata".to_owned(), serde_json::json!(entry.metadata));
    context.insert(
        "sets".to_owned(),
        entry
            .sets
            .clone()
            .map_or(Value::Null, |sets| serde_json::json!(sets)),
    );
    context.insert(
        "raw_source".to_owned(),
        Value::String(entry.raw_source.clone()),
    );
    context.insert("body".to_owned(), Value::String(entry.body.clone()));

    render_loaded_template(LoadedTemplateRequest {
        template_name: if template.template_name.is_empty() {
            template_name.to_owned()
        } else {
            template.template_name.clone()
        },
        template_text: template.template_text.clone(),
        context,
        supporting_templates: template.supporting_templates.clone(),
    })
}

impl fmt::Display for RenderManyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGlob { glob, message } => {
                write!(f, "invalid source glob '{glob}': {message}")
            }
            Self::GlobWalk { glob, message } => {
                write!(f, "failed to walk source glob '{glob}': {message}")
            }
            Self::InvalidTemplatePath { path, message } => {
                write!(
                    f,
                    "invalid render-many template path {}: {message}",
                    path.display()
                )
            }
            Self::CreateOutputDir { path, source } => {
                write!(
                    f,
                    "failed to create render-many output dir {}: {source}",
                    path.display()
                )
            }
            Self::WriteOutput { path, source } => {
                write!(
                    f,
                    "failed to write render-many output {}: {source}",
                    path.display()
                )
            }
            Self::WriteManifest { path, source } => {
                write!(
                    f,
                    "failed to write render-many manifest {}: {source}",
                    path.display()
                )
            }
            Self::SerializeManifest { source } => {
                write!(f, "failed to serialize render-many manifest: {source}")
            }
            Self::Render {
                source_path,
                source,
            } => {
                write!(
                    f,
                    "failed to render source entry {}: {source}",
                    source_path.display()
                )
            }
            Self::SourceEntry(source) => write!(f, "{source}"),
            Self::Template(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for RenderManyError {}
