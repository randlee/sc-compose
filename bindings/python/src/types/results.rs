use std::collections::BTreeMap;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use sc_composer::{
    ComposeResult, Diagnostic, ExpandedTemplate, Frontmatter, FrontmatterInitResult, InitResult,
    ParsedTemplate, RenderedArtifact, Renderer, ResolveResult, ValidationReport, VerifyResult,
};

use crate::convert::{extract_json_context, json_to_py};
use crate::enums::{diagnostic_severity_str, variable_source_str};
use crate::errors::{config_error, render_error_to_pyerr};
use crate::types::PyVariableName;
use crate::types::policy::python_bool_repr;

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
    fn pass_number(&self) -> u8 {
        self.inner.pass_number()
    }

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
    fn passes(&self) -> Vec<PyFrontmatter> {
        self.inner
            .passes()
            .iter()
            .cloned()
            .map(|inner| PyFrontmatter { inner })
            .collect()
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

    /// Return the legacy single-frontmatter view for each expanded file.
    ///
    /// This preserves the pre-multi-pass Python shape by returning only the
    /// outermost frontmatter block for each file. Use `frontmatter_passes`
    /// when callers need the full stacked-header data.
    #[getter]
    fn frontmatters(&self) -> Vec<(String, Option<PyFrontmatter>)> {
        self.inner
            .frontmatters
            .iter()
            .map(|(path, frontmatter)| {
                (
                    path.display().to_string(),
                    frontmatter
                        .first()
                        .cloned()
                        .map(|inner| PyFrontmatter { inner }),
                )
            })
            .collect()
    }

    /// Return every parsed frontmatter block for each expanded file.
    #[getter]
    fn frontmatter_passes(&self) -> Vec<(String, Vec<PyFrontmatter>)> {
        self.inner
            .frontmatters
            .iter()
            .map(|(path, frontmatters)| {
                (
                    path.display().to_string(),
                    frontmatters
                        .iter()
                        .cloned()
                        .map(|inner| PyFrontmatter { inner })
                        .collect(),
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

#[pyclass(name = "VerifyResult", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyVerifyResult {
    pub(crate) inner: VerifyResult,
}

#[pymethods]
impl PyVerifyResult {
    #[getter]
    fn clean(&self) -> bool {
        self.inner.clean
    }

    #[getter]
    fn resolved_template_path(&self) -> String {
        self.inner.resolved_template_path.display().to_string()
    }

    #[getter]
    fn deployed_path(&self) -> String {
        self.inner.deployed_path.display().to_string()
    }

    #[getter]
    fn rendered_text(&self) -> String {
        self.inner.rendered_text.clone()
    }

    #[getter]
    fn deployed_text(&self) -> String {
        self.inner.deployed_text.clone()
    }

    #[getter]
    fn diff(&self) -> Option<String> {
        self.inner.diff.clone()
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

    fn __repr__(&self) -> String {
        format!(
            "VerifyResult(clean={}, resolved_template_path={:?}, deployed_path={:?}, diff={})",
            python_bool_repr(self.inner.clean),
            self.inner.resolved_template_path.display().to_string(),
            self.inner.deployed_path.display().to_string(),
            python_bool_repr(self.inner.diff.is_some()),
        )
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
