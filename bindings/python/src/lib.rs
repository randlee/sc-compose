use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pyo3::PyTypeInfo;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyType};
use sc_composer::{
    BUILTIN_VARIABLE_NAMES, ComposeError, ComposeMode, ComposePolicy, ComposeRequest,
    ComposeResult, ConfigError, ConfiningRoot, Diagnostic, DiagnosticCode, DiagnosticSeverity,
    ExpandedTemplate, Frontmatter, FrontmatterInitResult, IncludeError, InitResult, InputValue,
    LoadedTemplateRequest, NamedTemplateAsset, ParsedTemplate, ProfileKind, ProfileName,
    RenderError, RenderedArtifact, Renderer, ResolveError, ResolveResult, ResolverPolicy,
    RuntimeKind, UnknownVariablePolicy, ValidationError, ValidationReport, VariableName,
    VariableSource,
};

create_exception!(sc_compose, ScComposeError, PyException);
create_exception!(sc_compose, ScRenderError, ScComposeError);
create_exception!(sc_compose, ScValidationError, ScComposeError);
create_exception!(sc_compose, ScResolveError, ScComposeError);
create_exception!(sc_compose, ScIncludeError, ScComposeError);
create_exception!(sc_compose, ScConfigError, ScComposeError);

#[pyclass(name = "RuntimeKind")]
struct PyRuntimeKind;

#[pymethods]
impl PyRuntimeKind {
    #[classattr]
    const CLAUDE: &'static str = "claude";
    #[classattr]
    const CODEX: &'static str = "codex";
    #[classattr]
    const GEMINI: &'static str = "gemini";
    #[classattr]
    const OPENCODE: &'static str = "opencode";
}

#[pyclass(name = "ProfileKind")]
struct PyProfileKind;

#[pymethods]
impl PyProfileKind {
    #[classattr]
    const AGENT: &'static str = "agent";
    #[classattr]
    const COMMAND: &'static str = "command";
    #[classattr]
    const SKILL: &'static str = "skill";
}

#[pyclass(name = "UnknownVariablePolicy")]
struct PyUnknownVariablePolicy;

#[pymethods]
impl PyUnknownVariablePolicy {
    #[classattr]
    const ERROR: &'static str = "error";
    #[classattr]
    const WARN: &'static str = "warn";
    #[classattr]
    const IGNORE: &'static str = "ignore";
}

#[pyclass(name = "VariableSource")]
struct PyVariableSource;

#[pymethods]
impl PyVariableSource {
    #[classattr]
    const EXPLICIT_INPUT: &'static str = "explicit_input";
    #[classattr]
    const ENVIRONMENT: &'static str = "environment";
    #[classattr]
    const BUILTIN: &'static str = "builtin";
    #[classattr]
    const TEMPLATE_INPUT_DEFAULT: &'static str = "template_input_default";
    #[classattr]
    const FRONTMATTER_DEFAULT: &'static str = "frontmatter_default";
    #[classattr]
    const INCLUDED_DEFAULT: &'static str = "included_default";
}

#[pyclass(name = "DiagnosticSeverity")]
struct PyDiagnosticSeverity;

#[pymethods]
impl PyDiagnosticSeverity {
    #[classattr]
    const ERROR: &'static str = "error";
    #[classattr]
    const WARNING: &'static str = "warning";
    #[classattr]
    const INFO: &'static str = "info";
}

#[pyclass(name = "DiagnosticCode")]
struct PyDiagnosticCode;

#[pymethods]
impl PyDiagnosticCode {
    #[classattr]
    const ERR_RESOLVE_NOT_FOUND: &'static str = "ERR_RESOLVE_NOT_FOUND";
    #[classattr]
    const ERR_RESOLVE_AMBIGUOUS: &'static str = "ERR_RESOLVE_AMBIGUOUS";
    #[classattr]
    const ERR_INCLUDE_ESCAPE: &'static str = "ERR_INCLUDE_ESCAPE";
    #[classattr]
    const ERR_INCLUDE_NOT_FOUND: &'static str = "ERR_INCLUDE_NOT_FOUND";
    #[classattr]
    const ERR_INCLUDE_CYCLE: &'static str = "ERR_INCLUDE_CYCLE";
    #[classattr]
    const ERR_INCLUDE_DEPTH: &'static str = "ERR_INCLUDE_DEPTH";
    #[classattr]
    const ERR_VAL_OBJECT_SHAPE: &'static str = "ERR_VAL_OBJECT_SHAPE";
    #[classattr]
    const ERR_VAL_NESTED_ARRAY_UNSUPPORTED: &'static str = "ERR_VAL_NESTED_ARRAY_UNSUPPORTED";
    #[classattr]
    const ERR_VAL_DUPLICATE: &'static str = "ERR_VAL_DUPLICATE";
    #[classattr]
    const WARN_VAL_CONFLICTING_DEFAULT_SECTIONS: &'static str =
        "WARN_VAL_CONFLICTING_DEFAULT_SECTIONS";
    #[classattr]
    const ERR_VAL_EMPTY: &'static str = "ERR_VAL_EMPTY";
    #[classattr]
    const ERR_VAL_MISSING_FRONTMATTER: &'static str = "ERR_VAL_MISSING_FRONTMATTER";
    #[classattr]
    const ERR_VAL_MISSING_REQUIRED: &'static str = "ERR_VAL_MISSING_REQUIRED";
    #[classattr]
    const ERR_VAL_MISSING_NESTED_FIELD: &'static str = "ERR_VAL_MISSING_NESTED_FIELD";
    #[classattr]
    const ERR_VAL_SHAPE_MISMATCH: &'static str = "ERR_VAL_SHAPE_MISMATCH";
    #[classattr]
    const ERR_VAL_UNDECLARED_TOKEN: &'static str = "ERR_VAL_UNDECLARED_TOKEN";
    #[classattr]
    const ERR_VAL_EXTRA_INPUT: &'static str = "ERR_VAL_EXTRA_INPUT";
    #[classattr]
    const INFO_VAL_DEFAULT_USED: &'static str = "INFO_VAL_DEFAULT_USED";
    #[classattr]
    const ERR_RENDER_STDIN_DOUBLE_READ: &'static str = "ERR_RENDER_STDIN_DOUBLE_READ";
    #[classattr]
    const ERR_RENDER_WRITE: &'static str = "ERR_RENDER_WRITE";
    #[classattr]
    const ERR_CONFIG_READONLY: &'static str = "ERR_CONFIG_READONLY";
    #[classattr]
    const ERR_CONFIG_MODE: &'static str = "ERR_CONFIG_MODE";
    #[classattr]
    const ERR_CONFIG_PARSE: &'static str = "ERR_CONFIG_PARSE";
    #[classattr]
    const ERR_CONFIG_VARFILE: &'static str = "ERR_CONFIG_VARFILE";
    #[classattr]
    const ERR_CONFIG_PACK_NOT_FOUND: &'static str = "ERR_CONFIG_PACK_NOT_FOUND";
    #[classattr]
    const ERR_CONFIG_PACK_NOT_RENDERABLE: &'static str = "ERR_CONFIG_PACK_NOT_RENDERABLE";
    #[classattr]
    const ERR_CONFIG_TEMPLATE_EXISTS: &'static str = "ERR_CONFIG_TEMPLATE_EXISTS";
}

#[pyclass(name = "VariableName", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyVariableName {
    inner: VariableName,
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
struct PyProfileName {
    inner: ProfileName,
}

#[pymethods]
impl PyProfileName {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        ProfileName::new(value)
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
struct PyConfiningRoot {
    inner: ConfiningRoot,
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
struct PyResolverPolicy {
    inner: ResolverPolicy,
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
struct PyComposeMode {
    inner: ComposeMode,
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
}

#[pyclass(name = "ComposePolicy", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyComposePolicy {
    inner: ComposePolicy,
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
}

#[pyclass(name = "ComposeRequest", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyComposeRequest {
    inner: ComposeRequest,
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
}

#[pyclass(name = "Diagnostic", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyDiagnostic {
    inner: Diagnostic,
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
struct PyResolveResult {
    inner: ResolveResult,
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
struct PyComposeResult {
    inner: ComposeResult,
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
struct PyValidationReport {
    inner: ValidationReport,
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
struct PyNamedTemplateAsset {
    inner: NamedTemplateAsset,
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
struct PyLoadedTemplateRequest {
    inner: LoadedTemplateRequest,
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
struct PyRenderedArtifact {
    inner: RenderedArtifact,
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
struct PyFrontmatter {
    inner: Frontmatter,
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
struct PyParsedTemplate {
    inner: ParsedTemplate,
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
struct PyExpandedTemplate {
    inner: ExpandedTemplate,
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
struct PyFrontmatterInitResult {
    inner: FrontmatterInitResult,
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
struct PyInitResult {
    inner: InitResult,
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
struct PyRenderer {
    inner: Renderer,
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

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn compose(request: PyRef<'_, PyComposeRequest>) -> PyResult<PyComposeResult> {
    sc_composer::compose(&request.inner)
        .map(|inner| PyComposeResult { inner })
        .map_err(compose_error_to_pyerr)
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn compose_file(request: PyRef<'_, PyComposeRequest>) -> PyResult<PyComposeResult> {
    compose(request)
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn validate(request: PyRef<'_, PyComposeRequest>) -> PyResult<PyValidationReport> {
    sc_composer::validate(&request.inner)
        .map(|inner| PyValidationReport { inner })
        .map_err(compose_error_to_pyerr)
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn resolve_template_path(request: PyRef<'_, PyComposeRequest>) -> PyResult<PyResolveResult> {
    sc_composer::resolve_template_path(&request.inner)
        .map(|inner| PyResolveResult { inner })
        .map_err(compose_error_to_pyerr)
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn resolve_profile(request: PyRef<'_, PyComposeRequest>) -> PyResult<PyResolveResult> {
    sc_composer::resolve_profile(&request.inner)
        .map(|inner| PyResolveResult { inner })
        .map_err(compose_error_to_pyerr)
}

#[pyfunction]
fn render_template(template: &str, context: &Bound<'_, PyAny>) -> PyResult<String> {
    sc_composer::render_template(template, extract_json_context(context)?)
        .map_err(render_error_to_pyerr)
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn render_loaded_template(
    request: PyRef<'_, PyLoadedTemplateRequest>,
) -> PyResult<PyRenderedArtifact> {
    sc_composer::render_loaded_template(request.inner.clone())
        .map(|inner| PyRenderedArtifact { inner })
        .map_err(render_error_to_pyerr)
}

#[pyfunction]
fn parse_template_document(input: &str) -> PyResult<PyParsedTemplate> {
    sc_composer::parse_template_document(input)
        .map(|inner| PyParsedTemplate { inner })
        .map_err(compose_error_to_pyerr)
}

#[pyfunction]
#[pyo3(signature = (template_path, root, policy=None))]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn expand_includes(
    template_path: &Bound<'_, PyAny>,
    root: &Bound<'_, PyAny>,
    policy: Option<PyRef<'_, PyComposePolicy>>,
) -> PyResult<PyExpandedTemplate> {
    let template_path = coerce_path_like(template_path)?;
    let root = ConfiningRoot::new(coerce_path_like(root)?)
        .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))?;
    let policy = policy
        .as_ref()
        .map_or_else(ComposePolicy::default, |policy| policy.inner.clone());
    sc_composer::expand_includes(Path::new(&template_path), &root, &policy)
        .map(|inner| PyExpandedTemplate { inner })
        .map_err(compose_error_to_pyerr)
}

#[pyfunction]
#[pyo3(signature = (path, force=false, dry_run=false))]
fn frontmatter_init(
    path: &Bound<'_, PyAny>,
    force: bool,
    dry_run: bool,
) -> PyResult<PyFrontmatterInitResult> {
    let path = coerce_path_like(path)?;
    sc_composer::frontmatter_init(path, force, dry_run)
        .map(|inner| PyFrontmatterInitResult { inner })
        .map_err(compose_error_to_pyerr)
}

#[pyfunction]
#[pyo3(signature = (root, dry_run=false))]
fn init_workspace(root: &Bound<'_, PyAny>, dry_run: bool) -> PyResult<PyInitResult> {
    let root = coerce_path_like(root)?;
    sc_composer::init_workspace(root, dry_run)
        .map(|inner| PyInitResult { inner })
        .map_err(compose_error_to_pyerr)
}

#[pyfunction]
fn validate_input_value(value: &Bound<'_, PyAny>) -> PyResult<()> {
    let value = py_to_json_value(value)?;
    sc_composer::validate_input_value(&value)
        .map_err(|error| validation_error(error.message().to_owned(), Some(error.code().as_str())))
}

#[pyfunction]
fn input_value_from_yaml(input: &str, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let yaml = serde_yaml::from_str::<serde_yaml::Value>(input)
        .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))?;
    let value = sc_composer::input_value_from_yaml(yaml).map_err(|error| {
        validation_error(error.message().to_owned(), Some(error.code().as_str()))
    })?;
    json_to_py(py, &value)
}

#[pyfunction]
fn to_forward_slash(path: &Bound<'_, PyAny>) -> PyResult<String> {
    Ok(sc_composer::to_forward_slash(Path::new(&coerce_path_like(
        path,
    )?)))
}

#[pyfunction]
fn discover_tokens(text: &str) -> Vec<PyVariableName> {
    sc_composer::discover_tokens(text)
        .into_iter()
        .map(|inner| PyVariableName { inner })
        .collect()
}

#[pymodule]
#[pyo3(name = "_native")]
fn native(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add(
        "BUILTIN_VARIABLE_NAMES",
        PyList::new(py, BUILTIN_VARIABLE_NAMES)?,
    )?;

    module.add("ScComposeError", py.get_type::<ScComposeError>())?;
    module.add("ScRenderError", py.get_type::<ScRenderError>())?;
    module.add("ScValidationError", py.get_type::<ScValidationError>())?;
    module.add("ScResolveError", py.get_type::<ScResolveError>())?;
    module.add("ScIncludeError", py.get_type::<ScIncludeError>())?;
    module.add("ScConfigError", py.get_type::<ScConfigError>())?;

    module.add_class::<PyRuntimeKind>()?;
    module.add_class::<PyProfileKind>()?;
    module.add_class::<PyUnknownVariablePolicy>()?;
    module.add_class::<PyVariableSource>()?;
    module.add_class::<PyDiagnosticSeverity>()?;
    module.add_class::<PyDiagnosticCode>()?;
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

    module.add_function(wrap_pyfunction!(compose, module)?)?;
    module.add_function(wrap_pyfunction!(compose_file, module)?)?;
    module.add_function(wrap_pyfunction!(validate, module)?)?;
    module.add_function(wrap_pyfunction!(resolve_template_path, module)?)?;
    module.add_function(wrap_pyfunction!(resolve_profile, module)?)?;
    module.add_function(wrap_pyfunction!(render_template, module)?)?;
    module.add_function(wrap_pyfunction!(render_loaded_template, module)?)?;
    module.add_function(wrap_pyfunction!(parse_template_document, module)?)?;
    module.add_function(wrap_pyfunction!(expand_includes, module)?)?;
    module.add_function(wrap_pyfunction!(frontmatter_init, module)?)?;
    module.add_function(wrap_pyfunction!(init_workspace, module)?)?;
    module.add_function(wrap_pyfunction!(validate_input_value, module)?)?;
    module.add_function(wrap_pyfunction!(input_value_from_yaml, module)?)?;
    module.add_function(wrap_pyfunction!(to_forward_slash, module)?)?;
    module.add_function(wrap_pyfunction!(discover_tokens, module)?)?;
    Ok(())
}

fn extract_supporting_templates(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<NamedTemplateAsset>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut assets = Vec::new();
    for item in value.try_iter()? {
        let item = item?;
        let asset = item.extract::<PyRef<'_, PyNamedTemplateAsset>>()?;
        assets.push(asset.inner.clone());
    }
    Ok(assets)
}

fn extract_profile_name(value: &Bound<'_, PyAny>) -> PyResult<ProfileName> {
    if let Ok(profile_name) = value.extract::<PyRef<'_, PyProfileName>>() {
        return Ok(profile_name.inner.clone());
    }
    let value = value.extract::<String>()?;
    ProfileName::new(value).map_err(|error| config_error(error.to_string(), None))
}

fn extract_runtime_kind(value: Option<&Bound<'_, PyAny>>) -> PyResult<Option<RuntimeKind>> {
    value.map(parse_runtime_kind).transpose()
}

fn extract_allowed_roots(value: Option<&Bound<'_, PyAny>>) -> PyResult<Vec<ConfiningRoot>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut roots = Vec::new();
    for item in value.try_iter()? {
        let item = item?;
        if let Ok(root) = item.extract::<PyRef<'_, PyConfiningRoot>>() {
            roots.push(root.inner.clone());
        } else {
            let path = coerce_path_like(&item)?;
            roots.push(
                ConfiningRoot::new(path)
                    .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))?,
            );
        }
    }
    Ok(roots)
}

fn extract_string_map(value: &Bound<'_, PyAny>) -> PyResult<BTreeMap<String, InputValue>> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_error| validation_error("context must be a Python dict".to_owned(), None))?;
    let mut vars = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key = key
            .extract::<String>()
            .map_err(|_error| validation_error("context keys must be strings".to_owned(), None))?;
        let value = py_to_json_value(&value)?;
        sc_composer::validate_input_value(&value).map_err(|error| {
            validation_error(error.message().to_owned(), Some(error.code().as_str()))
        })?;
        vars.insert(key, value);
    }
    Ok(vars)
}

fn extract_var_map(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<BTreeMap<VariableName, InputValue>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let dict = value.cast::<PyDict>().map_err(|_error| {
        validation_error(
            "variable maps must be Python dict instances".to_owned(),
            None,
        )
    })?;
    let mut vars = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key = key.extract::<String>().map_err(|_error| {
            validation_error("variable names must be strings".to_owned(), None)
        })?;
        let variable = VariableName::new(key.clone()).map_err(|error| {
            validation_error(format!("invalid variable name `{key}`: {error}"), None)
        })?;
        let input = py_to_json_value(&value)?;
        sc_composer::validate_input_value(&input).map_err(|error| {
            validation_error(error.message().to_owned(), Some(error.code().as_str()))
        })?;
        vars.insert(variable, input);
    }
    Ok(vars)
}

fn extract_json_context(value: &Bound<'_, PyAny>) -> PyResult<InputValue> {
    let json = py_to_json_value(value)?;
    sc_composer::validate_input_value(&json).map_err(|error| {
        validation_error(error.message().to_owned(), Some(error.code().as_str()))
    })?;
    Ok(json)
}

fn coerce_path_like(value: &Bound<'_, PyAny>) -> PyResult<String> {
    let os = value.py().import("os")?;
    os.call_method1("fspath", (value,))?.extract::<String>()
}

fn parse_runtime_kind(value: &Bound<'_, PyAny>) -> PyResult<RuntimeKind> {
    match value.extract::<String>()?.as_str() {
        "claude" => Ok(RuntimeKind::Claude),
        "codex" => Ok(RuntimeKind::Codex),
        "gemini" => Ok(RuntimeKind::Gemini),
        "opencode" => Ok(RuntimeKind::Opencode),
        other => Err(config_error(format!("unknown runtime kind: {other}"), None)),
    }
}

fn parse_profile_kind(value: &Bound<'_, PyAny>) -> PyResult<ProfileKind> {
    match value.extract::<String>()?.as_str() {
        "agent" => Ok(ProfileKind::Agent),
        "command" => Ok(ProfileKind::Command),
        "skill" => Ok(ProfileKind::Skill),
        other => Err(config_error(format!("unknown profile kind: {other}"), None)),
    }
}

fn parse_unknown_variable_policy(value: &str) -> PyResult<UnknownVariablePolicy> {
    match value {
        "error" => Ok(UnknownVariablePolicy::Error),
        "warn" => Ok(UnknownVariablePolicy::Warn),
        "ignore" => Ok(UnknownVariablePolicy::Ignore),
        other => Err(config_error(
            format!("unknown unknown-variable policy: {other}"),
            None,
        )),
    }
}

fn runtime_kind_str(value: RuntimeKind) -> &'static str {
    match value {
        RuntimeKind::Claude => "claude",
        RuntimeKind::Codex => "codex",
        RuntimeKind::Gemini => "gemini",
        RuntimeKind::Opencode => "opencode",
    }
}

fn profile_kind_str(value: ProfileKind) -> &'static str {
    match value {
        ProfileKind::Agent => "agent",
        ProfileKind::Command => "command",
        ProfileKind::Skill => "skill",
    }
}

fn unknown_variable_policy_str(value: UnknownVariablePolicy) -> &'static str {
    match value {
        UnknownVariablePolicy::Error => "error",
        UnknownVariablePolicy::Warn => "warn",
        UnknownVariablePolicy::Ignore => "ignore",
    }
}

fn variable_source_str(value: &VariableSource) -> &'static str {
    match value {
        VariableSource::ExplicitInput => "explicit_input",
        VariableSource::Environment => "environment",
        VariableSource::Builtin => "builtin",
        VariableSource::TemplateInputDefault => "template_input_default",
        VariableSource::FrontmatterDefault => "frontmatter_default",
        VariableSource::IncludedDefault => "included_default",
    }
}

fn diagnostic_severity_str(value: DiagnosticSeverity) -> &'static str {
    match value {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
    }
}

fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(value) => {
            Ok(PyBool::new(py, *value).to_owned().into_any().unbind())
        }
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(value.into_pyobject(py)?.unbind().into_any())
            } else if let Some(value) = value.as_u64() {
                Ok(value.into_pyobject(py)?.unbind().into_any())
            } else if let Some(value) = value.as_f64() {
                Ok(value.into_pyobject(py)?.unbind().into_any())
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(value) => Ok(value.into_pyobject(py)?.unbind().into_any()),
        serde_json::Value::Array(values) => {
            let list = PyList::empty(py);
            for value in values {
                list.append(json_to_py(py, value)?)?;
            }
            Ok(list.into_any().unbind())
        }
        serde_json::Value::Object(values) => {
            let dict = PyDict::new(py);
            for (key, value) in values {
                dict.set_item(key, json_to_py(py, value)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

fn py_to_json_value(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if value.is_none() {
        return Ok(serde_json::Value::Null);
    }
    if let Ok(value) = value.extract::<bool>() {
        return Ok(serde_json::Value::Bool(value));
    }
    if let Ok(value) = value.extract::<i64>() {
        return Ok(serde_json::Value::Number(value.into()));
    }
    if let Ok(value) = value.extract::<u64>() {
        return Ok(serde_json::Value::Number(value.into()));
    }
    if let Ok(value) = value.extract::<f64>() {
        let number = serde_json::Number::from_f64(value).ok_or_else(|| {
            validation_error("floating-point values must be finite".to_owned(), None)
        })?;
        return Ok(serde_json::Value::Number(number));
    }
    if let Ok(value) = value.extract::<String>() {
        return Ok(serde_json::Value::String(value));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let mut object = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key = key.extract::<String>().map_err(|_error| {
                validation_error("object keys must be strings".to_owned(), None)
            })?;
            object.insert(key, py_to_json_value(&value)?);
        }
        return Ok(serde_json::Value::Object(object));
    }
    if let Ok(sequence) = value.try_iter() {
        let mut items = Vec::new();
        for item in sequence {
            items.push(py_to_json_value(&item?)?);
        }
        return Ok(serde_json::Value::Array(items));
    }

    Err(validation_error(
        format!(
            "unsupported Python value type for compose input: {}",
            value.get_type().name()?
        ),
        None,
    ))
}

fn compose_error_to_pyerr(error: ComposeError) -> PyErr {
    match error {
        ComposeError::Resolve(error) => resolve_error_to_pyerr(error),
        ComposeError::Include(error) => include_error_to_pyerr(error),
        ComposeError::Validation(error) => validation_error_to_pyerr(*error),
        ComposeError::Render(error) => render_error_to_pyerr(error),
        ComposeError::Config(error) => config_error_to_pyerr(error),
    }
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
fn resolve_error_to_pyerr(error: ResolveError) -> PyErr {
    exception_with_attrs::<ScResolveError>(error.message(), Some(error.code().as_str()))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
fn include_error_to_pyerr(error: IncludeError) -> PyErr {
    exception_with_attrs::<ScIncludeError>(error.message(), Some(error.code().as_str()))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
fn validation_error_to_pyerr(error: ValidationError) -> PyErr {
    exception_with_attrs::<ScValidationError>(error.message(), Some(error.code().as_str()))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
fn render_error_to_pyerr(error: RenderError) -> PyErr {
    exception_with_attrs::<ScRenderError>(error.message(), error.code().map(DiagnosticCode::as_str))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
fn config_error_to_pyerr(error: ConfigError) -> PyErr {
    exception_with_attrs::<ScConfigError>(error.message(), Some(error.code().as_str()))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "Wrapper helpers often build dynamic String messages before constructing Python exceptions."
)]
fn validation_error(message: String, code: Option<&str>) -> PyErr {
    exception_with_attrs::<ScValidationError>(&message, code)
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "Wrapper helpers often build dynamic String messages before constructing Python exceptions."
)]
fn config_error(message: String, code: Option<&str>) -> PyErr {
    exception_with_attrs::<ScConfigError>(&message, code)
}

fn exception_with_attrs<T>(message: &str, code: Option<&str>) -> PyErr
where
    T: PyTypeInfo,
{
    Python::attach(|py| {
        let message = message.to_owned();
        let ty = py.get_type::<T>();
        let instance = ty
            .call1((message.clone(),))
            .expect("exception construction");
        instance
            .setattr("message", message.clone())
            .expect("message attribute");
        match code {
            Some(code) => instance.setattr("code", code).expect("code attribute"),
            None => instance.setattr("code", py.None()).expect("code attribute"),
        }
        PyErr::from_value(instance.into_any())
    })
}
