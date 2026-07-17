use std::path::Path;

use pyo3::prelude::*;

use crate::convert::{coerce_path_like, extract_json_context, json_to_py, py_to_json_value};
use crate::errors::{
    compose_error_to_pyerr, config_error, render_error_to_pyerr, validation_error,
};
use crate::types::{
    PyComposePolicy, PyComposeRequest, PyComposeResult, PyExpandedTemplate,
    PyFrontmatterInitResult, PyInitResult, PyLoadedTemplateRequest, PyParsedTemplate,
    PyRenderedArtifact, PyResolveResult, PyValidationReport, PyVariableName,
};

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
    let root = sc_composer::ConfiningRoot::new(coerce_path_like(root)?)
        .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))?;
    let policy = policy
        .as_ref()
        .map_or_else(sc_composer::ComposePolicy::default, |policy| {
            policy.inner.clone()
        });
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

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_tokens_wrapper_returns_python_variable_names() {
        let tokens = discover_tokens("{{ name }} {{ report.title }}");
        let values = tokens
            .into_iter()
            .map(|token| token.inner.to_string())
            .collect::<Vec<_>>();

        assert_eq!(values, vec!["name", "report.title"]);
    }
}
