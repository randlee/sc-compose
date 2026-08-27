//! Typed Python adapter for the versioned `sc-composer-beads` contract.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyTuple};
use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeError as RustBeadComposeError, BeadComposeReceipt,
    BeadComposeRequest, BeadOperation, BeadOutcome, BeadStage, BeadStageOutcome, BeadStageReceipt,
    PourAuthorization, execute_bead_request,
};
use serde_json::Value;

const REQUEST_STAGE: &str = "request";

#[pyclass(extends = PyException, name = "BeadComposeError")]
#[derive(Debug)]
struct PyBeadComposeError {
    #[pyo3(get)]
    code: String,
    #[pyo3(get)]
    stage: Option<String>,
    #[pyo3(get)]
    message: String,
}

#[pymethods]
impl PyBeadComposeError {
    #[new]
    #[pyo3(signature = (code, message, stage=None))]
    fn new(code: String, message: String, stage: Option<String>) -> Self {
        Self {
            code,
            stage,
            message,
        }
    }

    fn __str__(&self) -> &str {
        &self.message
    }
}

fn error(
    py: Python<'_>,
    code: impl Into<String>,
    stage: Option<&str>,
    message: impl Into<String>,
) -> PyErr {
    PyErr::from_type(
        py.get_type::<PyBeadComposeError>(),
        (code.into(), message.into(), stage.map(str::to_owned)),
    )
}

fn request_error(py: Python<'_>, message: impl Into<String>) -> PyErr {
    error(
        py,
        RustBeadComposeError::RequestDeserializationFailed {
            message: String::new(),
        }
        .code(),
        Some(REQUEST_STAGE),
        message,
    )
}

fn rust_error_stage(error_kind: &RustBeadComposeError) -> &'static str {
    match error_kind {
        RustBeadComposeError::RenderFailed { .. } => "render",
        RustBeadComposeError::ProcessOutputLimitExceeded { stage, .. } => stage_name(*stage),
        RustBeadComposeError::CookFailed { .. } | RustBeadComposeError::BdUnavailable { .. } => {
            "validate"
        }
        RustBeadComposeError::ActiveRegistryResolutionFailed { .. }
        | RustBeadComposeError::FormulaOutsideActiveRegistry { .. }
        | RustBeadComposeError::FormulaRegistryAmbiguous { .. } => "resolve_active_registry",
        RustBeadComposeError::PreviewPourFailed { .. } => "preview_pour",
        RustBeadComposeError::PourFailed { .. } => "pour",
        RustBeadComposeError::RequestDeserializationFailed { .. }
        | RustBeadComposeError::UnknownSchema { .. }
        | RustBeadComposeError::FormulaPathNotFile { .. }
        | RustBeadComposeError::FormulaExtensionUnsupported { .. }
        | RustBeadComposeError::TemplatePathInvalid { .. }
        | RustBeadComposeError::TemplateOutsideWorkingDirectory { .. }
        | RustBeadComposeError::OutputOutsideWorkingDirectory { .. }
        | RustBeadComposeError::OutputPathSymlink { .. }
        | RustBeadComposeError::PathNotUtf8 { .. }
        | RustBeadComposeError::BeadVariableKeyInvalid { .. }
        | RustBeadComposeError::BeadVariableKeyDuplicate { .. }
        | RustBeadComposeError::FormulaNameRequired
        | RustBeadComposeError::PourAuthorizationRequired
        | RustBeadComposeError::PourAuthorizationInvalid => REQUEST_STAGE,
    }
}

fn rust_error_to_pyerr(py: Python<'_>, error_kind: &RustBeadComposeError) -> PyErr {
    error(
        py,
        error_kind.code(),
        Some(rust_error_stage(error_kind)),
        error_kind.to_string(),
    )
}

fn coerce_path(py: Python<'_>, value: &Bound<'_, PyAny>, field: &str) -> PyResult<PathBuf> {
    let os = py.import("os")?;
    let path = os
        .call_method1("fspath", (value,))?
        .extract::<String>()
        .map_err(|_error| request_error(py, format!("{field} must be a path-like string")))?;
    Ok(PathBuf::from(path))
}

fn validate_json_input(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<()> {
    if let Ok(dict) = value.cast::<PyDict>() {
        for (key, item) in dict.iter() {
            key.extract::<String>().map_err(|_error| {
                request_error(py, "compose_variables object keys must be strings")
            })?;
            validate_json_input(py, &item)?;
        }
    } else if let Ok(items) = value.cast::<PyList>() {
        for item in items.iter() {
            validate_json_input(py, &item)?;
        }
    } else if let Ok(items) = value.cast::<PyTuple>() {
        for item in items.iter() {
            validate_json_input(py, &item)?;
        }
    } else if let Ok(number) = value.cast::<PyFloat>()
        && !number.value().is_finite()
    {
        return Err(request_error(
            py,
            "compose_variables floating-point values must be finite",
        ));
    }
    Ok(())
}

fn py_to_json(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Value> {
    validate_json_input(py, value)?;
    let json = py
        .import("json")
        .map_err(|error| request_error(py, error.to_string()))?;
    let serialized = json
        .call_method1("dumps", (value,))
        .map_err(|error| request_error(py, error.to_string()))?
        .extract::<String>()
        .map_err(|error| request_error(py, error.to_string()))?;
    serde_json::from_str(&serialized).map_err(|error| request_error(py, error.to_string()))
}

fn json_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    let serialized =
        serde_json::to_string(value).map_err(|error| request_error(py, error.to_string()))?;
    let json_module = py
        .import("json")
        .map_err(|error| request_error(py, error.to_string()))?;
    let json = json_module
        .call_method1("loads", (serialized,))
        .map_err(|error| request_error(py, error.to_string()))?
        .unbind();
    Ok(json)
}

fn operation_from_str(py: Python<'_>, value: &str) -> PyResult<BeadOperation> {
    match value {
        "render" => Ok(BeadOperation::Render),
        "validate" => Ok(BeadOperation::Validate),
        "preview_pour" => Ok(BeadOperation::PreviewPour),
        "pour" => Ok(BeadOperation::Pour),
        _ => Err(request_error(
            py,
            "operation must be render, validate, preview_pour, or pour",
        )),
    }
}

fn operation_name(operation: BeadOperation) -> &'static str {
    match operation {
        BeadOperation::Render => "render",
        BeadOperation::Validate => "validate",
        BeadOperation::PreviewPour => "preview_pour",
        BeadOperation::Pour => "pour",
    }
}

fn stage_name(stage: BeadStage) -> &'static str {
    match stage {
        BeadStage::Render => "render",
        BeadStage::Validate => "validate",
        BeadStage::ResolveActiveRegistry => "resolve_active_registry",
        BeadStage::PreviewPour => "preview_pour",
        BeadStage::Pour => "pour",
    }
}

fn parse_bead_variables(
    py: Python<'_>,
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<BTreeMap<String, String>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let dict = value
        .cast::<PyDict>()
        .map_err(|_error| request_error(py, "bead_variables must be a string mapping"))?;
    let mut variables = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key = key
            .extract::<String>()
            .map_err(|_error| request_error(py, "bead_variables keys must be strings"))?;
        let value = value
            .extract::<String>()
            .map_err(|_error| request_error(py, "bead_variables values must be strings"))?;
        variables.insert(key, value);
    }
    Ok(variables)
}

#[pyclass(name = "BeadOperation")]
struct PyBeadOperation;

#[pymethods]
impl PyBeadOperation {
    #[classattr]
    const RENDER: &'static str = "render";
    #[classattr]
    const VALIDATE: &'static str = "validate";
    #[classattr]
    const PREVIEW_POUR: &'static str = "preview_pour";
    #[classattr]
    const POUR: &'static str = "pour";
}

#[pyclass(name = "PourAuthorization")]
struct PyPourAuthorization;

#[pymethods]
impl PyPourAuthorization {
    #[classattr]
    const CREATE_PERSISTENT_BEADS: &'static str = "CreatePersistentBeads";
}

#[pyclass(name = "BeadStage")]
struct PyBeadStage;

#[pymethods]
impl PyBeadStage {
    #[classattr]
    const RENDER: &'static str = "render";
    #[classattr]
    const VALIDATE: &'static str = "validate";
    #[classattr]
    const RESOLVE_ACTIVE_REGISTRY: &'static str = "resolve_active_registry";
    #[classattr]
    const PREVIEW_POUR: &'static str = "preview_pour";
    #[classattr]
    const POUR: &'static str = "pour";
}

#[pyclass(name = "BeadStageOutcome", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyBeadStageOutcome {
    #[pyo3(get)]
    kind: String,
    #[pyo3(get)]
    code: Option<String>,
}

#[pyclass(name = "BeadOutcome", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyBeadOutcome {
    #[pyo3(get)]
    kind: String,
    #[pyo3(get)]
    code: Option<String>,
}

#[pyclass(name = "BeadStageReceipt", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyBeadStageReceipt {
    #[pyo3(get)]
    stage: String,
    #[pyo3(get)]
    argv: Vec<String>,
    #[pyo3(get)]
    exit_status: Option<i32>,
    #[pyo3(get)]
    elapsed_ms: u64,
    #[pyo3(get)]
    stdout_excerpt: String,
    #[pyo3(get)]
    stderr_excerpt: String,
    #[pyo3(get)]
    outcome: PyBeadStageOutcome,
}

#[pyclass(name = "BeadComposeReceipt", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyBeadComposeReceipt {
    #[pyo3(get)]
    schema: String,
    #[pyo3(get)]
    operation: String,
    #[pyo3(get)]
    rendered_formula: String,
    #[pyo3(get)]
    stages: Vec<PyBeadStageReceipt>,
    #[pyo3(get)]
    outcome: PyBeadOutcome,
}

fn stage_outcome(inner: &BeadStageOutcome) -> PyBeadStageOutcome {
    match inner {
        BeadStageOutcome::Succeeded => PyBeadStageOutcome {
            kind: "succeeded".to_owned(),
            code: None,
        },
        BeadStageOutcome::Skipped => PyBeadStageOutcome {
            kind: "skipped".to_owned(),
            code: None,
        },
        BeadStageOutcome::Failed { code } => PyBeadStageOutcome {
            kind: "failed".to_owned(),
            code: Some(code.clone()),
        },
    }
}

fn stage_receipt(inner: &BeadStageReceipt) -> PyBeadStageReceipt {
    PyBeadStageReceipt {
        stage: stage_name(inner.stage).to_owned(),
        argv: inner.argv.clone(),
        exit_status: inner.exit_status,
        elapsed_ms: inner.elapsed_ms,
        stdout_excerpt: inner.stdout_excerpt.clone(),
        stderr_excerpt: inner.stderr_excerpt.clone(),
        outcome: stage_outcome(&inner.outcome),
    }
}

fn outcome(inner: &BeadOutcome) -> PyBeadOutcome {
    match inner {
        BeadOutcome::Succeeded => PyBeadOutcome {
            kind: "succeeded".to_owned(),
            code: None,
        },
        BeadOutcome::Refused { code } => PyBeadOutcome {
            kind: "refused".to_owned(),
            code: Some(code.clone()),
        },
        BeadOutcome::Failed { code } => PyBeadOutcome {
            kind: "failed".to_owned(),
            code: Some(code.clone()),
        },
    }
}

impl From<BeadComposeReceipt> for PyBeadComposeReceipt {
    fn from(inner: BeadComposeReceipt) -> Self {
        Self {
            schema: inner.schema,
            operation: operation_name(inner.operation).to_owned(),
            rendered_formula: inner.rendered_formula.display().to_string(),
            stages: inner.stages.iter().map(stage_receipt).collect(),
            outcome: outcome(&inner.outcome),
        }
    }
}

#[pyclass(name = "BeadComposeRequest", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyBeadComposeRequest {
    inner: BeadComposeRequest,
}

#[pymethods]
impl PyBeadComposeRequest {
    #[new]
    #[pyo3(signature = (working_directory, template, rendered_formula, compose_variables, *, operation="render", formula_name=None, bead_variables=None, bd_executable=None, pour_authorization=None, schema=BEADS_SCHEMA_V1))]
    #[allow(
        clippy::too_many_arguments,
        reason = "The Python constructor mirrors the complete versioned Rust request contract."
    )]
    fn new(
        py: Python<'_>,
        working_directory: &Bound<'_, PyAny>,
        template: &Bound<'_, PyAny>,
        rendered_formula: &Bound<'_, PyAny>,
        compose_variables: &Bound<'_, PyAny>,
        operation: &str,
        formula_name: Option<String>,
        bead_variables: Option<&Bound<'_, PyAny>>,
        bd_executable: Option<&Bound<'_, PyAny>>,
        pour_authorization: Option<&str>,
        schema: &str,
    ) -> PyResult<Self> {
        let compose_variables = py_to_json(py, compose_variables)?;
        let compose_variables = compose_variables
            .as_object()
            .cloned()
            .ok_or_else(|| request_error(py, "compose_variables must be a string-keyed mapping"))?;
        let pour_authorization = match pour_authorization {
            None => None,
            Some("CreatePersistentBeads") => Some(PourAuthorization::CreatePersistentBeads),
            Some(_) => {
                return Err(request_error(
                    py,
                    "pour_authorization must be CreatePersistentBeads when supplied",
                ));
            }
        };
        Ok(Self {
            inner: BeadComposeRequest {
                schema: schema.to_owned(),
                operation: operation_from_str(py, operation)?,
                working_directory: coerce_path(py, working_directory, "working_directory")?,
                template: coerce_path(py, template, "template")?,
                rendered_formula: coerce_path(py, rendered_formula, "rendered_formula")?,
                compose_variables,
                formula_name,
                bead_variables: parse_bead_variables(py, bead_variables)?,
                bd_executable: bd_executable
                    .map(|value| coerce_path(py, value, "bd_executable"))
                    .transpose()?,
                pour_authorization,
            },
        })
    }

    #[getter]
    fn schema(&self) -> String {
        self.inner.schema.clone()
    }

    #[getter]
    fn operation(&self) -> &'static str {
        operation_name(self.inner.operation)
    }

    #[getter]
    fn working_directory(&self) -> String {
        self.inner.working_directory.display().to_string()
    }

    #[getter]
    fn template(&self) -> String {
        self.inner.template.display().to_string()
    }

    #[getter]
    fn rendered_formula(&self) -> String {
        self.inner.rendered_formula.display().to_string()
    }

    #[getter]
    fn compose_variables(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_to_py(py, &Value::Object(self.inner.compose_variables.clone()))
    }

    #[getter]
    fn formula_name(&self) -> Option<String> {
        self.inner.formula_name.clone()
    }

    #[getter]
    fn bead_variables(&self) -> BTreeMap<String, String> {
        self.inner.bead_variables.clone()
    }

    #[getter]
    fn bd_executable(&self) -> Option<String> {
        self.inner
            .bd_executable
            .as_ref()
            .map(|path| path.display().to_string())
    }

    #[getter]
    fn pour_authorization(&self) -> Option<&'static str> {
        self.inner
            .pour_authorization
            .map(|_| "CreatePersistentBeads")
    }
}

fn execute_with_operation(
    py: Python<'_>,
    request: &PyBeadComposeRequest,
    operation: Option<BeadOperation>,
) -> PyResult<PyBeadComposeReceipt> {
    let mut request = request.inner.clone();
    if let Some(operation) = operation {
        request.operation = operation;
    }
    py.detach(|| execute_bead_request(&request))
        .map(PyBeadComposeReceipt::from)
        .map_err(|error_kind| rust_error_to_pyerr(py, &error_kind))
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts the Python-owned request through a PyRef argument."
)]
fn execute(
    py: Python<'_>,
    request: PyRef<'_, PyBeadComposeRequest>,
) -> PyResult<PyBeadComposeReceipt> {
    execute_with_operation(py, &request, Some(request.inner.operation))
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts the Python-owned request through a PyRef argument."
)]
fn render(
    py: Python<'_>,
    request: PyRef<'_, PyBeadComposeRequest>,
) -> PyResult<PyBeadComposeReceipt> {
    execute_with_operation(py, &request, Some(BeadOperation::Render))
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts the Python-owned request through a PyRef argument."
)]
fn validate(
    py: Python<'_>,
    request: PyRef<'_, PyBeadComposeRequest>,
) -> PyResult<PyBeadComposeReceipt> {
    execute_with_operation(py, &request, Some(BeadOperation::Validate))
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts the Python-owned request through a PyRef argument."
)]
fn preview_pour(
    py: Python<'_>,
    request: PyRef<'_, PyBeadComposeRequest>,
) -> PyResult<PyBeadComposeReceipt> {
    execute_with_operation(py, &request, Some(BeadOperation::PreviewPour))
}

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts the Python-owned request through a PyRef argument."
)]
fn pour(
    py: Python<'_>,
    request: PyRef<'_, PyBeadComposeRequest>,
) -> PyResult<PyBeadComposeReceipt> {
    execute_with_operation(py, &request, Some(BeadOperation::Pour))
}

#[pymodule]
#[pyo3(name = "_native")]
fn native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("BEADS_SCHEMA_V1", BEADS_SCHEMA_V1)?;
    module.add_class::<PyBeadComposeError>()?;
    module.add_class::<PyBeadOperation>()?;
    module.add_class::<PyPourAuthorization>()?;
    module.add_class::<PyBeadStage>()?;
    module.add_class::<PyBeadStageOutcome>()?;
    module.add_class::<PyBeadOutcome>()?;
    module.add_class::<PyBeadStageReceipt>()?;
    module.add_class::<PyBeadComposeReceipt>()?;
    module.add_class::<PyBeadComposeRequest>()?;
    module.add_function(wrap_pyfunction!(execute, module)?)?;
    module.add_function(wrap_pyfunction!(render, module)?)?;
    module.add_function(wrap_pyfunction!(validate, module)?)?;
    module.add_function(wrap_pyfunction!(preview_pour, module)?)?;
    module.add_function(wrap_pyfunction!(pour, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use serde_json::json;

    fn temporary_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("sc-composer-beads-python-{nonce}"))
    }

    #[test]
    fn adapter_matches_the_in_process_rust_render_receipt() {
        let root = temporary_root();
        let templates = root.join("templates");
        let template = templates.join("toml-workflow.formula.toml.j2");
        let rendered_formula = root.join(".beads/formulas/toml-workflow.formula.toml");
        fs::create_dir_all(&templates).expect("test template directory must be created");
        fs::create_dir_all(
            rendered_formula
                .parent()
                .expect("rendered formula must have a parent directory"),
        )
        .expect("test formula directory must be created");
        fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../crates/sc-composer-beads/tests/fixtures/beads/toml-workflow.formula.toml.j2"
            ),
            &template,
        )
        .expect("canonical Beads fixture must be copied");

        let request = BeadComposeRequest {
            schema: BEADS_SCHEMA_V1.to_owned(),
            operation: BeadOperation::Render,
            working_directory: root.clone(),
            template,
            rendered_formula,
            compose_variables: json!({
                "project": {"name": "sc-compose", "notes": "in-process parity"},
                "reviewers": [{"id": "ada", "name": "Ada"}],
            })
            .as_object()
            .expect("JSON fixture must be an object")
            .clone(),
            formula_name: None,
            bead_variables: BTreeMap::new(),
            bd_executable: None,
            pour_authorization: None,
        };

        let rust_receipt = execute_bead_request(&request).expect("direct Rust render must succeed");
        Python::initialize();
        Python::attach(|py| {
            let python_receipt = execute_with_operation(
                py,
                &PyBeadComposeRequest {
                    inner: request.clone(),
                },
                Some(BeadOperation::Render),
            )
            .expect("Python adapter render must succeed");

            assert_eq!(python_receipt.schema, rust_receipt.schema);
            assert_eq!(
                python_receipt.operation,
                operation_name(rust_receipt.operation)
            );
            assert_eq!(
                python_receipt.rendered_formula,
                rust_receipt.rendered_formula.display().to_string()
            );
            assert_eq!(python_receipt.stages.len(), rust_receipt.stages.len());
            assert_eq!(python_receipt.outcome.kind, "succeeded");
        });

        fs::remove_dir_all(root).expect("test workspace must be removed");
    }
}
