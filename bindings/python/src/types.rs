use std::collections::BTreeMap;
use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use sc_composer::{
    ComposeMode, ComposePolicy, ComposeRequest, ComposeResult, ConfiningRoot, Diagnostic,
    ExpandedTemplate, Frontmatter, FrontmatterInitResult, InitResult, LoadedTemplateRequest,
    NamedTemplateAsset, ParsedTemplate, RenderedArtifact, Renderer, ResolveResult, ResolverPolicy,
    ValidationReport, VariableName,
};

use crate::convert::{
    coerce_path_like, extract_allowed_roots, extract_json_context, extract_profile_name,
    extract_runtime_kind, extract_string_map, extract_supporting_templates, extract_var_map,
    json_to_py,
};
use crate::enums::{
    diagnostic_severity_str, parse_profile_kind, parse_unknown_variable_policy, profile_kind_str,
    runtime_kind_str, unknown_variable_policy_str, variable_source_str,
};
use crate::errors::{
    compose_error_to_pyerr, config_error, render_error_to_pyerr, validation_error,
};

fn python_string_repr(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

#[pyclass(name = "VariableName", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyVariableName {
    pub(crate) inner: VariableName,
}

#[pymethods]
impl PyVariableName {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        VariableName::new(value)
            .map(|inner| Self { inner })
            .map_err(|error| validation_error(error.to_string(), None))
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("VariableName({:?})", self.inner.as_str())
    }
}

#[pyclass(name = "ProfileName", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyProfileName {
    pub(crate) inner: sc_composer::ProfileName,
}

#[pymethods]
impl PyProfileName {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        sc_composer::ProfileName::new(value)
            .map(|inner| Self { inner })
            .map_err(|error| config_error(error.to_string(), None))
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ProfileName({:?})", self.inner.as_str())
    }
}

#[pyclass(name = "ConfiningRoot", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyConfiningRoot {
    pub(crate) inner: ConfiningRoot,
}

#[pymethods]
impl PyConfiningRoot {
    #[new]
    fn new(path: &Bound<'_, PyAny>) -> PyResult<Self> {
        let path = coerce_path_like(path)?;
        ConfiningRoot::new(&path)
            .map(|inner| Self { inner })
            .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))
    }

    fn confine(&self, candidate: &Bound<'_, PyAny>) -> PyResult<String> {
        let candidate = coerce_path_like(candidate)?;
        let canonical = sc_composer::resolve_template_path(&ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from(candidate),
            },
            root: self.inner.clone(),
            vars_input: BTreeMap::new(),
            vars_env: BTreeMap::new(),
            vars_defaults: BTreeMap::new(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        })
        .map_err(compose_error_to_pyerr)?;
        Ok(canonical.resolved_path.display().to_string())
    }

    fn __str__(&self) -> String {
        self.inner.as_path().display().to_string()
    }
}

#[pyclass(name = "ResolverPolicy", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyResolverPolicy {
    pub(crate) inner: ResolverPolicy,
}

#[pymethods]
impl PyResolverPolicy {
    fn __repr__(&self) -> String {
        format!(
            "ResolverPolicy(candidate_directories={}, filename_probes={}, ambiguous_without_runtime_is_error={})",
            self.inner.candidate_directories.len(),
            self.inner.filename_probes.len(),
            self.inner.ambiguous_without_runtime_is_error
        )
    }
}

#[pyclass(name = "ComposeMode", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyComposeMode {
    pub(crate) inner: ComposeMode,
}

#[pymethods]
impl PyComposeMode {
    #[staticmethod]
    fn file(template_path: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: ComposeMode::File {
                template_path: PathBuf::from(coerce_path_like(template_path)?),
            },
        })
    }

    #[staticmethod]
    fn profile(kind: &Bound<'_, PyAny>, name: &Bound<'_, PyAny>) -> PyResult<Self> {
        let kind = parse_profile_kind(kind)?;
        let name = extract_profile_name(name)?;
        Ok(Self {
            inner: ComposeMode::Profile { kind, name },
        })
    }

    #[getter]
    fn template_path(&self) -> Option<String> {
        match &self.inner {
            ComposeMode::File { template_path } => Some(template_path.display().to_string()),
            ComposeMode::Profile { .. } => None,
        }
    }

    #[getter]
    fn kind(&self) -> Option<String> {
        match &self.inner {
            ComposeMode::Profile { kind, .. } => Some(profile_kind_str(*kind).to_owned()),
            ComposeMode::File { .. } => None,
        }
    }

    #[getter]
    fn name(&self) -> Option<String> {
        match &self.inner {
            ComposeMode::Profile { name, .. } => Some(name.to_string()),
            ComposeMode::File { .. } => None,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            ComposeMode::File { template_path } => {
                format!(
                    "ComposeMode.file({:?})",
                    template_path.display().to_string()
                )
            }
            ComposeMode::Profile { kind, name } => format!(
                "ComposeMode.profile(kind={}, name={})",
                python_string_repr(profile_kind_str(*kind)),
                python_string_repr(name.as_str())
            ),
        }
    }
}

#[pyclass(name = "ComposePolicy", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyComposePolicy {
    pub(crate) inner: ComposePolicy,
}

#[pymethods]
impl PyComposePolicy {
    #[new]
    #[pyo3(signature = (strict_undeclared_variables=false, unknown_variable_policy="ignore", max_include_depth=32, allowed_roots=None))]
    fn new(
        strict_undeclared_variables: bool,
        unknown_variable_policy: &str,
        max_include_depth: u16,
        allowed_roots: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: ComposePolicy {
                strict_undeclared_variables,
                unknown_variable_policy: parse_unknown_variable_policy(unknown_variable_policy)?,
                max_include_depth: sc_composer::IncludeDepth::new(max_include_depth),
                allowed_roots: extract_allowed_roots(allowed_roots)?,
                resolver_policy: ResolverPolicy::default(),
            },
        })
    }

    #[getter]
    fn strict_undeclared_variables(&self) -> bool {
        self.inner.strict_undeclared_variables
    }

    #[getter]
    fn unknown_variable_policy(&self) -> String {
        unknown_variable_policy_str(self.inner.unknown_variable_policy).to_owned()
    }

    #[getter]
    fn max_include_depth(&self) -> u16 {
        self.inner.max_include_depth.get()
    }

    #[getter]
    fn allowed_roots(&self) -> Vec<String> {
        self.inner
            .allowed_roots
            .iter()
            .map(|root| root.as_path().display().to_string())
            .collect()
    }

    #[getter]
    fn resolver_policy(&self) -> PyResolverPolicy {
        PyResolverPolicy {
            inner: self.inner.resolver_policy.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ComposePolicy(strict_undeclared_variables={}, unknown_variable_policy={}, max_include_depth={}, allowed_roots={:?}, resolver_policy={})",
            self.inner.strict_undeclared_variables,
            python_string_repr(unknown_variable_policy_str(
                self.inner.unknown_variable_policy
            )),
            self.inner.max_include_depth.get(),
            self.allowed_roots(),
            PyResolverPolicy {
                inner: self.inner.resolver_policy.clone(),
            }
            .__repr__()
        )
    }
}

#[pyclass(name = "ComposeRequest", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyComposeRequest {
    pub(crate) inner: ComposeRequest,
}

#[pymethods]
impl PyComposeRequest {
    #[new]
    #[pyo3(signature = (root, mode, vars_input=None, vars_env=None, vars_defaults=None, guidance_block=None, user_prompt=None, policy=None, runtime=None))]
    #[allow(
        clippy::too_many_arguments,
        clippy::needless_pass_by_value,
        reason = "Python constructor shape is part of the planned public API and PyO3 extracts owned PyRef arguments."
    )]
    fn new(
        root: &Bound<'_, PyAny>,
        mode: PyRef<'_, PyComposeMode>,
        vars_input: Option<&Bound<'_, PyAny>>,
        vars_env: Option<&Bound<'_, PyAny>>,
        vars_defaults: Option<&Bound<'_, PyAny>>,
        guidance_block: Option<String>,
        user_prompt: Option<String>,
        policy: Option<PyRef<'_, PyComposePolicy>>,
        runtime: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let root_string = coerce_path_like(root)?;
        Ok(Self {
            inner: ComposeRequest {
                runtime: extract_runtime_kind(runtime)?,
                mode: mode.inner.clone(),
                root: ConfiningRoot::new(&root_string)
                    .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))?,
                vars_input: extract_var_map(vars_input)?,
                vars_env: extract_var_map(vars_env)?,
                vars_defaults: extract_var_map(vars_defaults)?,
                guidance_block,
                user_prompt,
                policy: policy
                    .as_ref()
                    .map_or_else(ComposePolicy::default, |policy| policy.inner.clone()),
            },
        })
    }

    #[getter]
    fn root(&self) -> String {
        self.inner.root.as_path().display().to_string()
    }

    #[getter]
    fn runtime(&self) -> Option<String> {
        self.inner
            .runtime
            .map(|runtime| runtime_kind_str(runtime).to_owned())
    }

    #[getter]
    fn mode(&self) -> PyComposeMode {
        PyComposeMode {
            inner: self.inner.mode.clone(),
        }
    }

    #[getter]
    fn policy(&self) -> PyComposePolicy {
        PyComposePolicy {
            inner: self.inner.policy.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ComposeRequest(root={:?}, mode={}, runtime={:?}, vars_input={}, vars_env={}, vars_defaults={}, guidance_block={}, user_prompt={}, policy={})",
            self.root(),
            self.mode().__repr__(),
            self.runtime().as_deref().map(python_string_repr),
            self.inner.vars_input.len(),
            self.inner.vars_env.len(),
            self.inner.vars_defaults.len(),
            self.inner.guidance_block.is_some(),
            self.inner.user_prompt.is_some(),
            self.policy().__repr__()
        )
    }
}

#[pyclass(name = "Diagnostic", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyDiagnostic {
    pub(crate) inner: Diagnostic,
}

#[pymethods]
impl PyDiagnostic {
    #[getter]
    fn severity(&self) -> String {
        diagnostic_severity_str(self.inner.severity).to_owned()
    }

    #[getter]
    fn code(&self) -> String {
        self.inner.code.as_str().to_owned()
    }

    #[getter]
    fn message(&self) -> String {
        self.inner.message.clone()
    }

    #[getter]
    fn path(&self) -> Option<String> {
        self.inner
            .path
            .as_ref()
            .map(|path| path.display().to_string())
    }

    #[getter]
    fn line(&self) -> Option<usize> {
        self.inner.line
    }

    #[getter]
    fn column(&self) -> Option<usize> {
        self.inner.column
    }

    #[getter]
    fn include_chain(&self) -> Vec<String> {
        self.inner
            .include_chain
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    }
}

#[pyclass(name = "ResolveResult", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyResolveResult {
    pub(crate) inner: ResolveResult,
}

#[pymethods]
impl PyResolveResult {
    #[getter]
    fn resolved_path(&self) -> String {
        self.inner.resolved_path.display().to_string()
    }

    #[getter]
    fn attempted_paths(&self) -> Vec<String> {
        self.inner
            .attempted_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    }

    #[getter]
    fn ambiguity_candidates(&self) -> Vec<String> {
        self.inner
            .ambiguity_candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    }
}

#[pyclass(name = "ComposeResult", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyComposeResult {
    pub(crate) inner: ComposeResult,
}

#[pymethods]
impl PyComposeResult {
    #[getter]
    fn rendered_text(&self) -> String {
        self.inner.rendered_text.clone()
    }

    #[getter]
    fn resolved_files(&self) -> Vec<String> {
        self.inner
            .resolved_files
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    }

    #[getter]
    fn resolve_result(&self) -> PyResolveResult {
        PyResolveResult {
            inner: self.inner.resolve_result.clone(),
        }
    }

    #[getter]
    fn warnings(&self) -> Vec<PyDiagnostic> {
        self.inner
            .warnings
            .iter()
            .cloned()
            .map(|inner| PyDiagnostic { inner })
            .collect()
    }

    #[getter]
    fn variable_sources(&self) -> BTreeMap<String, String> {
        self.inner
            .variable_sources
            .iter()
            .map(|(key, value)| (key.to_string(), variable_source_str(value).to_owned()))
            .collect()
    }
}

#[pyclass(name = "ValidationReport", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyValidationReport {
    pub(crate) inner: ValidationReport,
}

#[pymethods]
impl PyValidationReport {
    #[getter]
    fn ok(&self) -> bool {
        self.inner.ok
    }

    #[getter]
    fn warnings(&self) -> Vec<PyDiagnostic> {
        self.inner
            .warnings
            .iter()
            .cloned()
            .map(|inner| PyDiagnostic { inner })
            .collect()
    }

    #[getter]
    fn errors(&self) -> Vec<PyDiagnostic> {
        self.inner
            .errors
            .iter()
            .cloned()
            .map(|inner| PyDiagnostic { inner })
            .collect()
    }

    #[getter]
    fn resolve_result(&self) -> PyResolveResult {
        PyResolveResult {
            inner: self.inner.resolve_result.clone(),
        }
    }
}

#[pyclass(name = "NamedTemplateAsset", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyNamedTemplateAsset {
    pub(crate) inner: NamedTemplateAsset,
}

#[pymethods]
impl PyNamedTemplateAsset {
    #[new]
    fn new(template_name: String, template_text: String) -> Self {
        Self {
            inner: NamedTemplateAsset {
                template_name,
                template_text,
            },
        }
    }

    #[getter]
    fn template_name(&self) -> String {
        self.inner.template_name.clone()
    }

    #[getter]
    fn template_text(&self) -> String {
        self.inner.template_text.clone()
    }
}

#[pyclass(name = "LoadedTemplateRequest", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyLoadedTemplateRequest {
    pub(crate) inner: LoadedTemplateRequest,
}

#[pymethods]
impl PyLoadedTemplateRequest {
    #[new]
    #[pyo3(signature = (template_name, template_text, context, supporting_templates=None))]
    fn new(
        template_name: String,
        template_text: String,
        context: &Bound<'_, PyAny>,
        supporting_templates: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: LoadedTemplateRequest {
                template_name,
                template_text,
                context: extract_string_map(context)?,
                supporting_templates: extract_supporting_templates(supporting_templates)?,
            },
        })
    }
}

#[pyclass(name = "RenderedArtifact", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyRenderedArtifact {
    pub(crate) inner: RenderedArtifact,
}

#[pymethods]
impl PyRenderedArtifact {
    #[getter]
    fn rendered(&self) -> String {
        self.inner.rendered.clone()
    }

    #[getter]
    fn template_name(&self) -> String {
        self.inner.template_name.clone()
    }
}

#[pyclass(name = "Frontmatter", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyFrontmatter {
    pub(crate) inner: Frontmatter,
}

#[pymethods]
impl PyFrontmatter {
    #[getter]
    fn required_variables(&self) -> Vec<PyVariableName> {
        self.inner
            .required_variables()
            .iter()
            .cloned()
            .map(|inner| PyVariableName { inner })
            .collect()
    }

    #[getter]
    fn defaults(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in self.inner.defaults() {
            dict.set_item(key.as_str(), json_to_py(py, value)?)?;
        }
        Ok(dict.into_any().unbind())
    }

    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in self.inner.metadata() {
            dict.set_item(
                key,
                json_to_py(
                    py,
                    &value.to_json_value().map_err(|error| {
                        config_error(error.to_string(), Some("ERR_CONFIG_PARSE"))
                    })?,
                )?,
            )?;
        }
        Ok(dict.into_any().unbind())
    }

    #[getter]
    fn diagnostics(&self) -> Vec<PyDiagnostic> {
        self.inner
            .diagnostics()
            .iter()
            .cloned()
            .map(|inner| PyDiagnostic { inner })
            .collect()
    }
}

#[pyclass(name = "ParsedTemplate", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyParsedTemplate {
    pub(crate) inner: ParsedTemplate,
}

#[pymethods]
impl PyParsedTemplate {
    #[getter]
    fn frontmatter(&self) -> Option<PyFrontmatter> {
        self.inner
            .frontmatter()
            .cloned()
            .map(|inner| PyFrontmatter { inner })
    }

    #[getter]
    fn body(&self) -> String {
        self.inner.body().to_owned()
    }
}

#[pyclass(name = "ExpandedTemplate", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyExpandedTemplate {
    pub(crate) inner: ExpandedTemplate,
}

#[pymethods]
impl PyExpandedTemplate {
    #[getter]
    fn text(&self) -> String {
        self.inner.text.clone()
    }

    #[getter]
    fn resolved_files(&self) -> Vec<String> {
        self.inner
            .resolved_files
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    }

    #[getter]
    fn frontmatters(&self) -> Vec<(String, Option<PyFrontmatter>)> {
        self.inner
            .frontmatters
            .iter()
            .map(|(path, frontmatter)| {
                (
                    path.display().to_string(),
                    frontmatter.clone().map(|inner| PyFrontmatter { inner }),
                )
            })
            .collect()
    }

    #[getter]
    fn include_chains(&self) -> BTreeMap<String, Vec<String>> {
        self.inner
            .include_chains
            .iter()
            .map(|(path, chain)| {
                (
                    path.display().to_string(),
                    chain
                        .iter()
                        .map(|entry| entry.display().to_string())
                        .collect(),
                )
            })
            .collect()
    }
}

#[pyclass(name = "FrontmatterInitResult", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyFrontmatterInitResult {
    pub(crate) inner: FrontmatterInitResult,
}

#[pymethods]
impl PyFrontmatterInitResult {
    #[getter]
    fn target_path(&self) -> String {
        self.inner.target_path.display().to_string()
    }

    #[getter]
    fn frontmatter_text(&self) -> String {
        self.inner.frontmatter_text.clone()
    }

    #[getter]
    fn discovered_variables(&self) -> Vec<PyVariableName> {
        self.inner
            .discovered_variables
            .iter()
            .cloned()
            .map(|inner| PyVariableName { inner })
            .collect()
    }

    #[getter]
    fn changed(&self) -> bool {
        self.inner.changed
    }

    #[getter]
    fn would_change(&self) -> bool {
        self.inner.would_change
    }
}

#[pyclass(name = "InitResult", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyInitResult {
    pub(crate) inner: InitResult,
}

#[pymethods]
impl PyInitResult {
    #[getter]
    fn prompts_dir(&self) -> String {
        self.inner.prompts_dir.display().to_string()
    }

    #[getter]
    fn gitignore_updated(&self) -> bool {
        self.inner.gitignore_updated
    }

    #[getter]
    fn scanned_templates(&self) -> Vec<String> {
        self.inner
            .scanned_templates
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    }

    #[getter]
    fn recommendations(&self) -> Vec<PyDiagnostic> {
        self.inner
            .recommendations
            .iter()
            .cloned()
            .map(|inner| PyDiagnostic { inner })
            .collect()
    }

    #[getter]
    fn validation_passed(&self) -> bool {
        self.inner.validation_passed
    }
}

#[pyclass(name = "Renderer", skip_from_py_object)]
#[derive(Debug)]
pub(crate) struct PyRenderer {
    pub(crate) inner: Renderer,
}

#[pymethods]
impl PyRenderer {
    #[new]
    fn new() -> Self {
        Self {
            inner: Renderer::new(),
        }
    }

    #[classmethod]
    fn with_delimiters(_cls: &Bound<'_, PyType>, open: &str, close: &str) -> Self {
        Self {
            inner: Renderer::with_delimiters(open, close),
        }
    }

    fn render(&self, template: &str, context: &Bound<'_, PyAny>) -> PyResult<String> {
        self.inner
            .render(template, extract_json_context(context)?)
            .map_err(render_error_to_pyerr)
    }

    fn render_named(
        &self,
        name: &str,
        template: &str,
        context: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        self.inner
            .render_named(name, template, extract_json_context(context)?)
            .map_err(render_error_to_pyerr)
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyVariableName>()?;
    module.add_class::<PyProfileName>()?;
    module.add_class::<PyConfiningRoot>()?;
    module.add_class::<PyResolverPolicy>()?;
    module.add_class::<PyComposeMode>()?;
    module.add_class::<PyComposePolicy>()?;
    module.add_class::<PyComposeRequest>()?;
    module.add_class::<PyDiagnostic>()?;
    module.add_class::<PyResolveResult>()?;
    module.add_class::<PyComposeResult>()?;
    module.add_class::<PyValidationReport>()?;
    module.add_class::<PyNamedTemplateAsset>()?;
    module.add_class::<PyLoadedTemplateRequest>()?;
    module.add_class::<PyRenderedArtifact>()?;
    module.add_class::<PyFrontmatter>()?;
    module.add_class::<PyParsedTemplate>()?;
    module.add_class::<PyExpandedTemplate>()?;
    module.add_class::<PyFrontmatterInitResult>()?;
    module.add_class::<PyInitResult>()?;
    module.add_class::<PyRenderer>()?;
    Ok(())
}
