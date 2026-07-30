use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use glob::glob;
use sc_composer::{LoadedTemplateRequest, RenderError, RenderedArtifact, render_loaded_template};
use serde::Serialize;
use serde_json::Value;

use crate::path_utils::to_forward_slash;
use crate::reporting::report_context::{context_from_source_entry, entry_title};
use crate::reporting::source_entry::{SourceEntry, SourceEntryError};
use crate::reporting::templates::{
    ResolvedTemplate, TemplateError, render_shared_report, resolve_template_family,
    resolve_template_selector,
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
struct RenderJob {
    source_entry: SourceEntry,
    rendered: RenderedArtifact,
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
        source: Box<RenderError>,
    },
    SourceEntry(SourceEntryError),
    Template(Box<TemplateError>),
}

impl RenderManyRequest {
    fn resolve_template(&self) -> Result<ResolvedTemplate, RenderManyError> {
        if let Some(family) = &self.source_set.template_family {
            return resolve_template_family(&self.root, family)
                .map_err(Box::new)
                .map_err(RenderManyError::Template);
        }
        resolve_template_selector(&self.root, &self.source_set.template_selector)
            .map_err(Box::new)
            .map_err(RenderManyError::Template)
    }
}

// This source-collection and render-many path is generic across text inputs.
// It is not Mermaid-specific and is intended to support Mermaid, SVG, Markdown,
// SQL, and other text assets that carry embedded metadata.
pub(crate) fn render_many(
    request: &RenderManyRequest,
) -> Result<RenderManyResult, RenderManyError> {
    let resolved_template = request.resolve_template()?;
    let output_root = request.root.join(&request.source_set.output_dir);
    std::fs::create_dir_all(&output_root).map_err(|source| RenderManyError::CreateOutputDir {
        path: output_root.clone(),
        source,
    })?;

    let mut discovered = discover_sources(request, &resolved_template.output_extension)?;
    discovered.sort_by(|left, right| left.record.source_path.cmp(&right.record.source_path));

    let mut entries = Vec::with_capacity(discovered.len());
    let mut generated_outputs = Vec::with_capacity(discovered.len() + 1);

    for source_entry in discovered {
        let job = render_job(source_entry, &request.source_set.id, &resolved_template)?;
        write_rendered_entry(&request.root, &job)?;
        generated_outputs.push(job.source_entry.record.output_path.clone());
        entries.push(manifest_entry_from_source(&job.source_entry));
    }

    let manifest_path = write_manifest(&request.root, &request.source_set.output_dir, &entries)?;
    generated_outputs.push(manifest_path.clone());

    Ok(RenderManyResult {
        manifest_path,
        entries,
        generated_outputs,
    })
}

fn discover_sources(
    request: &RenderManyRequest,
    output_extension: &str,
) -> Result<Vec<SourceEntry>, RenderManyError> {
    let canonical_root =
        request
            .root
            .canonicalize()
            .map_err(|error| RenderManyError::InvalidTemplatePath {
                path: request.root.clone(),
                message: error.to_string(),
            })?;
    let pattern = to_forward_slash(&canonical_root.join(&request.source_set.glob));
    let paths = glob(&pattern).map_err(|error| RenderManyError::InvalidGlob {
        glob: request.source_set.glob.clone(),
        message: error.to_string(),
    })?;

    let mut entries = Vec::new();
    for path in paths {
        let source_path = path.map_err(|error| RenderManyError::GlobWalk {
            glob: request.source_set.glob.clone(),
            message: error.to_string(),
        })?;
        let absolute_source = canonical_source_path(&canonical_root, &source_path)?;
        let relative_source = relative_source_path(&canonical_root, &absolute_source)?;
        let output_path = derive_output_path(
            &request.source_set.output_dir,
            relative_source,
            output_extension,
        );
        entries.push(
            SourceEntry::load(&absolute_source, relative_source, output_path)
                .map_err(RenderManyError::SourceEntry)?,
        );
    }
    Ok(entries)
}

fn render_job(
    source_entry: SourceEntry,
    template_name: &str,
    template: &ResolvedTemplate,
) -> Result<RenderJob, RenderManyError> {
    let rendered = render_entry(&source_entry, template_name, template).map_err(|source| {
        RenderManyError::Render {
            source_path: source_entry.record.source_path.clone(),
            source: Box::new(source),
        }
    })?;
    Ok(RenderJob {
        source_entry,
        rendered,
    })
}

fn write_rendered_entry(root: &Path, job: &RenderJob) -> Result<(), RenderManyError> {
    let absolute_output = root.join(&job.source_entry.record.output_path);
    if let Some(parent) = absolute_output.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RenderManyError::CreateOutputDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(&absolute_output, &job.rendered.rendered).map_err(|source| {
        RenderManyError::WriteOutput {
            path: absolute_output,
            source,
        }
    })
}

fn manifest_entry_from_source(source_entry: &SourceEntry) -> RenderManyManifestEntry {
    RenderManyManifestEntry {
        source_path: source_entry.record.source_path.clone(),
        output_path: source_entry.record.output_path.clone(),
        metadata: source_entry.record.metadata.clone(),
        sets: source_entry.record.sets.clone(),
    }
}

fn write_manifest(
    root: &Path,
    output_dir: &Path,
    entries: &[RenderManyManifestEntry],
) -> Result<PathBuf, RenderManyError> {
    let manifest_path = output_dir.join("manifest.json");
    let manifest_bytes = serde_json::to_vec_pretty(entries)
        .map_err(|source| RenderManyError::SerializeManifest { source })?;
    std::fs::write(root.join(&manifest_path), manifest_bytes).map_err(|source| {
        RenderManyError::WriteManifest {
            path: root.join(&manifest_path),
            source,
        }
    })?;
    Ok(manifest_path)
}

fn derive_output_path(output_dir: &Path, source_path: &Path, extension: &str) -> PathBuf {
    let mut output_path = output_dir.join(source_path);
    output_path.set_extension(extension);
    output_path
}

fn canonical_source_path(
    canonical_root: &Path,
    source_path: &Path,
) -> Result<PathBuf, RenderManyError> {
    let absolute_source = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        canonical_root.join(source_path)
    };
    absolute_source
        .canonicalize()
        .map_err(|error| RenderManyError::InvalidTemplatePath {
            path: source_path.to_path_buf(),
            message: error.to_string(),
        })
}

fn relative_source_path<'a>(
    canonical_root: &'a Path,
    absolute_source: &'a Path,
) -> Result<&'a Path, RenderManyError> {
    absolute_source
        .strip_prefix(canonical_root)
        .map_err(|error| RenderManyError::InvalidTemplatePath {
            path: absolute_source.to_path_buf(),
            message: error.to_string(),
        })
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
        Value::String(to_forward_slash(&entry.record.source_path)),
    );
    context.insert(
        "output_path".to_owned(),
        Value::String(to_forward_slash(&entry.record.output_path)),
    );
    context.insert(
        "metadata".to_owned(),
        serde_json::json!(entry.record.metadata),
    );
    context.insert(
        "sets".to_owned(),
        entry
            .record
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
                    to_forward_slash(source_path)
                )
            }
            Self::SourceEntry(source) => write!(f, "{source}"),
            Self::Template(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for RenderManyError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{RenderManyRequest, SourceSetDefinition, render_many};

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sc-compose-render-many-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn request(root: &Path, template_selector: &str) -> RenderManyRequest {
        RenderManyRequest {
            root: root.to_path_buf(),
            source_set: SourceSetDefinition {
                id: "panels".to_owned(),
                glob: "docs/*.txt".to_owned(),
                template_selector: template_selector.to_owned(),
                template_family: None,
                output_dir: PathBuf::from("reports/latest/panels"),
            },
        }
    }

    #[test]
    fn template_resolution_failure_does_not_create_output_root() {
        let root = temp_root("template-resolution-failure");
        write_file(
            &root.join("docs").join("a.txt"),
            "# title: Alpha\nalpha body\n",
        );

        let error = render_many(&request(&root, "reports/templates/missing.html.j2")).unwrap_err();

        assert!(error.to_string().contains("failed to read report template"));
        assert!(!root.join("reports").join("latest").join("panels").exists());
    }

    #[test]
    fn render_failure_preserves_already_written_outputs() {
        let root = temp_root("partial-write-on-render-failure");
        write_file(
            &root.join("reports").join("templates").join("panel.html.j2"),
            "<article>{{ metadata.title }}|{% include metadata.partial %}|{{ body }}</article>\n",
        );
        write_file(
            &root.join("docs").join("a.txt"),
            "# title: Alpha\n# partial: base/report.html.j2\nalpha body\n",
        );
        write_file(
            &root.join("docs").join("b.txt"),
            "# title: Bravo\n# partial: missing/report.html.j2\nbravo body\n",
        );

        let error = render_many(&request(&root, "reports/templates/panel.html.j2")).unwrap_err();

        assert!(error.to_string().contains(&format!(
            "failed to render source entry {}",
            crate::path_utils::to_forward_slash(&Path::new("docs").join("b.txt"))
        )));
        let first_output = fs::read_to_string(
            root.join("reports")
                .join("latest")
                .join("panels")
                .join("docs")
                .join("a.html"),
        )
        .unwrap();
        assert!(first_output.contains("Alpha"));
        assert!(first_output.contains("alpha body"));
        assert!(
            !root
                .join("reports")
                .join("latest")
                .join("panels")
                .join("docs")
                .join("b.html")
                .exists()
        );
        assert!(
            !root
                .join("reports")
                .join("latest")
                .join("panels")
                .join("manifest.json")
                .exists()
        );
    }
}
