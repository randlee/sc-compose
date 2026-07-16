use std::collections::BTreeMap;
use std::path::PathBuf;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sc_composer::{
    ComposeError, ComposeMode, ComposePolicy, ComposeRequest, ComposeResult, ConfiningRoot,
    UnknownVariablePolicy, VariableName, compose,
};

create_exception!(sc_compose, ScComposeError, PyException);

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

    #[getter]
    fn template_path(&self) -> Option<String> {
        match &self.inner {
            ComposeMode::File { template_path } => Some(template_path.display().to_string()),
            ComposeMode::Profile { .. } => None,
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
            ComposeMode::Profile { .. } => "ComposeMode.profile(...)".to_owned(),
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
    #[pyo3(signature = (strict_undeclared_variables=false))]
    fn new(strict_undeclared_variables: bool) -> Self {
        Self {
            inner: ComposePolicy {
                strict_undeclared_variables,
                unknown_variable_policy: UnknownVariablePolicy::Ignore,
                ..ComposePolicy::default()
            },
        }
    }

    #[getter]
    fn strict_undeclared_variables(&self) -> bool {
        self.inner.strict_undeclared_variables
    }

    fn __repr__(&self) -> String {
        format!(
            "ComposePolicy(strict_undeclared_variables={})",
            self.inner.strict_undeclared_variables
        )
    }
}

#[pyclass(name = "ComposeRequest", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyComposeRequest {
    inner: ComposeRequest,
    root: String,
    mode: PyComposeMode,
}

#[pymethods]
impl PyComposeRequest {
    #[new]
    #[pyo3(signature = (root, mode, vars_input=None, vars_env=None, vars_defaults=None, guidance_block=None, user_prompt=None, policy=None))]
    #[allow(
        clippy::too_many_arguments,
        reason = "Python constructor shape is part of the planned public API."
    )]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "PyO3 extracts borrowed Python-owned values into PyRef parameters."
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
    ) -> PyResult<Self> {
        let root_string = coerce_path_like(root)?;
        let inner = ComposeRequest {
            runtime: None,
            mode: mode.inner.clone(),
            root: ConfiningRoot::new(&root_string).map_err(|error| {
                sc_compose_error(format!(
                    "failed to canonicalize root {root_string}: {error}"
                ))
            })?,
            vars_input: extract_var_map(vars_input)?,
            vars_env: extract_var_map(vars_env)?,
            vars_defaults: extract_var_map(vars_defaults)?,
            guidance_block,
            user_prompt,
            policy: policy
                .as_ref()
                .map_or_else(ComposePolicy::default, |policy| policy.inner.clone()),
        };

        Ok(Self {
            inner,
            root: root_string,
            mode: mode.clone(),
        })
    }

    #[getter]
    fn root(&self) -> String {
        self.root.clone()
    }

    #[getter]
    fn mode(&self) -> PyComposeMode {
        self.mode.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ComposeRequest(root={:?}, mode={})",
            self.root,
            self.mode.__repr__()
        )
    }
}

#[pyclass(name = "ComposeResult", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyComposeResult {
    #[pyo3(get)]
    rendered_text: String,
    #[pyo3(get)]
    resolved_files: Vec<String>,
    #[pyo3(get)]
    warnings: Vec<String>,
}

impl From<ComposeResult> for PyComposeResult {
    fn from(value: ComposeResult) -> Self {
        Self {
            rendered_text: value.rendered_text,
            resolved_files: value
                .resolved_files
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            warnings: value
                .warnings
                .into_iter()
                .map(|warning| warning.message)
                .collect(),
        }
    }
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts Python-owned arguments into PyRef values."
)]
fn compose_file(request: PyRef<'_, PyComposeRequest>) -> PyResult<PyComposeResult> {
    compose(&request.inner)
        .map(PyComposeResult::from)
        .map_err(compose_error_to_pyerr)
}

#[pymodule]
#[pyo3(name = "_native")]
fn native(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("ScComposeError", py.get_type::<ScComposeError>())?;
    module.add_class::<PyComposeMode>()?;
    module.add_class::<PyComposePolicy>()?;
    module.add_class::<PyComposeRequest>()?;
    module.add_class::<PyComposeResult>()?;
    module.add_function(wrap_pyfunction!(compose_file, module)?)?;
    Ok(())
}

fn compose_error_to_pyerr(error: ComposeError) -> PyErr {
    let message = match error {
        ComposeError::Resolve(error) => error.message().to_owned(),
        ComposeError::Include(error) => error.message().to_owned(),
        ComposeError::Validation(error) => error.message().to_owned(),
        ComposeError::Render(error) => error.message().to_owned(),
        ComposeError::Config(error) => error.message().to_owned(),
    };
    ScComposeError::new_err(message)
}

fn sc_compose_error(message: impl Into<String>) -> PyErr {
    ScComposeError::new_err(message.into())
}

fn coerce_path_like(value: &Bound<'_, PyAny>) -> PyResult<String> {
    let os = value.py().import("os")?;
    os.call_method1("fspath", (value,))?.extract::<String>()
}

fn extract_var_map(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<BTreeMap<VariableName, serde_json::Value>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let dict = value
        .cast::<PyDict>()
        .map_err(|_error| sc_compose_error("variable maps must be Python dict instances"))?;
    let mut vars = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key: String = key
            .extract::<String>()
            .map_err(|_error| sc_compose_error("variable names must be strings"))?;
        let variable = VariableName::new(key.clone())
            .map_err(|error| sc_compose_error(format!("invalid variable name `{key}`: {error}")))?;
        let input = py_to_json_value(&value)?;
        sc_composer::validate_input_value(&input)
            .map_err(|error| sc_compose_error(error.message().to_owned()))?;
        vars.insert(variable, input);
    }
    Ok(vars)
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
        let number = serde_json::Number::from_f64(value)
            .ok_or_else(|| sc_compose_error("floating-point values must be finite"))?;
        return Ok(serde_json::Value::Number(number));
    }
    if let Ok(value) = value.extract::<String>() {
        return Ok(serde_json::Value::String(value));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let mut object = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key: String = key
                .extract::<String>()
                .map_err(|_error| sc_compose_error("object keys must be strings"))?;
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

    Err(sc_compose_error(format!(
        "unsupported Python value type for compose input: {}",
        value.get_type().name()?
    )))
}
