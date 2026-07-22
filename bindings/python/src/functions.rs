use std::collections::BTreeMap;
use std::path::Path;

use pyo3::prelude::*;

use crate::convert::{
    coerce_path_like, extract_json_context, extract_pass_contexts, json_to_py, py_to_json_value,
};
use crate::errors::{
    compose_error_to_pyerr, config_error, render_error_to_pyerr, validation_error,
};
use crate::types::{
    PyComposePolicy, PyComposeRequest, PyComposeResult, PyExpandedTemplate,
    PyFrontmatterInitResult, PyInitResult, PyLoadedTemplateRequest, PyParsedTemplate,
    PyRenderedArtifact, PyResolveResult, PyValidationReport, PyVariableName, PyVerifyResult,
};

/// Render a parsed multi-pass template from fully resolved per-pass contexts.
///
/// This low-level helper still applies each pass header's frontmatter defaults
/// beneath the caller-supplied per-pass context, matching the native
/// `sc_composer::render_all()` behavior. Callers that want request/policy
/// resolution, validation, and variable-source tracking should use
/// `compose()` instead.
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
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn render_all(
    parsed: PyRef<'_, PyParsedTemplate>,
    contexts: &Bound<'_, PyAny>,
) -> PyResult<String> {
    let contexts = extract_pass_contexts(contexts)?;
    sc_composer::render_all(&parsed.inner, &contexts).map_err(compose_error_to_pyerr)
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn verify(
    request: PyRef<'_, PyComposeRequest>,
    deployed_path: &Bound<'_, PyAny>,
) -> PyResult<PyVerifyResult> {
    let deployed_path = coerce_path_like(deployed_path)?;
    sc_composer::verify(&request.inner, deployed_path)
        .map(|inner| PyVerifyResult { inner })
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

#[pyfunction]
fn discover_tokens_with_brace_count(text: &str, brace_count: usize) -> Vec<PyVariableName> {
    sc_composer::discover_tokens_with_brace_count(text, brace_count)
        .into_iter()
        .map(|inner| PyVariableName { inner })
        .collect()
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn discover_all_pass_tokens(
    parsed: PyRef<'_, PyParsedTemplate>,
) -> BTreeMap<usize, Vec<PyVariableName>> {
    sc_composer::discover_all_pass_tokens(&parsed.inner)
        .into_iter()
        .map(|(pass, tokens)| {
            (
                pass,
                tokens
                    .into_iter()
                    .map(|inner| PyVariableName { inner })
                    .collect(),
            )
        })
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
    module.add_function(wrap_pyfunction!(render_all, module)?)?;
    module.add_function(wrap_pyfunction!(verify, module)?)?;
    module.add_function(wrap_pyfunction!(expand_includes, module)?)?;
    module.add_function(wrap_pyfunction!(frontmatter_init, module)?)?;
    module.add_function(wrap_pyfunction!(init_workspace, module)?)?;
    module.add_function(wrap_pyfunction!(validate_input_value, module)?)?;
    module.add_function(wrap_pyfunction!(input_value_from_yaml, module)?)?;
    module.add_function(wrap_pyfunction!(to_forward_slash, module)?)?;
    module.add_function(wrap_pyfunction!(discover_tokens, module)?)?;
    module.add_function(wrap_pyfunction!(discover_tokens_with_brace_count, module)?)?;
    module.add_function(wrap_pyfunction!(discover_all_pass_tokens, module)?)?;
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

    #[test]
    fn discover_tokens_with_brace_count_wrapper_uses_requested_delimiter_width() {
        let tokens = discover_tokens_with_brace_count("{{{ outer }}} {{ inner }}", 3);
        let values = tokens
            .into_iter()
            .map(|token| token.inner.to_string())
            .collect::<Vec<_>>();

        assert_eq!(values, vec!["outer"]);
    }

    #[test]
    fn discover_all_pass_tokens_wrapper_returns_per_pass_map() {
        let parsed = sc_composer::parse_template_document(
            "---\npass: 1\n---\n---\npass: 2\n---\n{{ name }} {{{ role }}}\n",
        )
        .unwrap();
        let wrapper = PyParsedTemplate { inner: parsed };

        Python::initialize();
        Python::attach(|py| {
            let parsed_ref = Py::new(py, wrapper.clone()).unwrap();
            let values = discover_all_pass_tokens(parsed_ref.bind(py).borrow());

            assert_eq!(
                values[&1]
                    .iter()
                    .map(|token| token.inner.to_string())
                    .collect::<Vec<_>>(),
                vec!["name"]
            );
            assert_eq!(
                values[&2]
                    .iter()
                    .map(|token| token.inner.to_string())
                    .collect::<Vec<_>>(),
                vec!["role"]
            );
        });
    }

    #[test]
    fn render_all_wrapper_renders_multi_pass_template() {
        let parsed = sc_composer::parse_template_document(
            "---\npass: 2\n---\n---\npass: 1\n---\n{{{ team }}} {{ task }}\n",
        )
        .unwrap();
        let wrapper = PyParsedTemplate { inner: parsed };

        Python::initialize();
        Python::attach(|py| {
            let contexts = pyo3::types::PyList::empty(py);
            let outer = pyo3::types::PyDict::new(py);
            outer.set_item("team", "wyvern").unwrap();
            let inner = pyo3::types::PyDict::new(py);
            inner.set_item("task", "test").unwrap();
            contexts.append((2_u8, outer)).unwrap();
            contexts.append((1_u8, inner)).unwrap();

            let parsed_ref = Py::new(py, wrapper.clone()).unwrap();
            let rendered = render_all(parsed_ref.bind(py).borrow(), contexts.as_any()).unwrap();

            assert_eq!(rendered, "wyvern test");
        });
    }
}
