//! Typed Python adapter for the versioned `sc-composer-beads` contract.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyTuple, PyType};
use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeError as RustBeadComposeError, BeadComposeReceipt,
    BeadComposeRequest, BeadOperation, BeadOutcome, BeadPourMode, BeadStage, BeadStageOutcome,
    BeadStageReceipt, PourAuthorization, execute_bead_request,
};
use serde_json::Value;

const REQUEST_STAGE: &str = "request";
const RECEIPT_STAGE: &str = "receipt";
const RECEIPT_DESERIALIZATION_FAILED_CODE: &str = "BEADS_RECEIPT_DESERIALIZATION_FAILED";

#[pyclass(extends = PyException, name = "BeadComposeError")]
#[derive(Debug)]
struct PyBeadComposeError {
    #[pyo3(get)]
    code: String,
    #[pyo3(get)]
    stage: Option<String>,
    #[pyo3(get)]
    message: String,
    #[pyo3(get)]
    details: Option<BTreeMap<String, String>>,
}

#[pymethods]
impl PyBeadComposeError {
    #[new]
    #[pyo3(signature = (code, message, stage=None, details=None))]
    fn new(
        code: String,
        message: String,
        stage: Option<String>,
        details: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            code,
            stage,
            message,
            details,
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
        RustBeadComposeError::REQUEST_DESERIALIZATION_FAILED_CODE,
        Some(REQUEST_STAGE),
        message,
    )
}

fn receipt_deserialization_error(py: Python<'_>, message: impl Into<String>) -> PyErr {
    error(
        py,
        RECEIPT_DESERIALIZATION_FAILED_CODE,
        Some(RECEIPT_STAGE),
        message,
    )
}

fn rust_error_stage(error_kind: &RustBeadComposeError) -> &'static str {
    match error_kind {
        RustBeadComposeError::RenderFailed { .. } => BeadStage::Render.as_str(),
        RustBeadComposeError::ProcessOutputLimitExceeded { stage, .. } => stage.as_str(),
        RustBeadComposeError::GraphIdInvalid { .. }
        | RustBeadComposeError::CookFailed { .. }
        | RustBeadComposeError::BdUnavailable { .. }
        | RustBeadComposeError::ProcessArgumentInvalid { .. } => BeadStage::Validate.as_str(),
        RustBeadComposeError::ActiveRegistryResolutionFailed { .. }
        | RustBeadComposeError::FormulaOutsideActiveRegistry { .. }
        | RustBeadComposeError::FormulaRegistryAmbiguous { .. } => {
            BeadStage::ResolveActiveRegistry.as_str()
        }
        RustBeadComposeError::PreviewPourFailed { .. } => BeadStage::PreviewPour.as_str(),
        RustBeadComposeError::PourFailed { .. } => BeadStage::Pour.as_str(),
        // Graph-stage failures are returned in receipts, not through this
        // request-error conversion. Preserve their code if directly supplied.
        RustBeadComposeError::GraphParentNotFound { .. }
        | RustBeadComposeError::GraphScopeMismatch { .. }
        | RustBeadComposeError::GraphFormulaUnsupported { .. }
        | RustBeadComposeError::GraphRelationInvalid { .. }
        | RustBeadComposeError::GraphConflict { .. }
        | RustBeadComposeError::GraphEdgeConflict { .. }
        | RustBeadComposeError::GraphEdgeMissing { .. }
        | RustBeadComposeError::GraphReadFailed { .. }
        | RustBeadComposeError::GraphApplyFailed { .. }
        | RustBeadComposeError::RelationEndpointInvalid { .. }
        | RustBeadComposeError::RequestReadFailed { .. }
        | RustBeadComposeError::RequestDeserializationFailed { .. }
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
        | RustBeadComposeError::BeadVariableValueInvalid { .. }
        | RustBeadComposeError::FormulaNameRequired
        | RustBeadComposeError::PourAuthorizationRequired
        | RustBeadComposeError::PourAuthorizationInvalid => REQUEST_STAGE,
    }
}

fn rust_error_to_pyerr(py: Python<'_>, error_kind: &RustBeadComposeError) -> PyErr {
    if let RustBeadComposeError::GraphIdInvalid { field, value } = error_kind {
        let details = BTreeMap::from([
            ("field".to_owned(), field.as_str().to_owned()),
            ("value".to_owned(), value.clone()),
        ]);
        return PyErr::from_type(
            py.get_type::<PyBeadComposeError>(),
            (
                error_kind.code(),
                error_kind.to_string(),
                rust_error_stage(error_kind),
                details,
            ),
        );
    }
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
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_error| {
        request_error(
            py,
            "operation must be render, validate, preview_pour, pour, preview_attach, or attach",
        )
    })
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
    const RENDER: &'static str = BeadOperation::Render.as_str();
    #[classattr]
    const VALIDATE: &'static str = BeadOperation::Validate.as_str();
    #[classattr]
    const PREVIEW_POUR: &'static str = BeadOperation::PreviewPour.as_str();
    #[classattr]
    const POUR: &'static str = BeadOperation::Pour.as_str();
    #[classattr]
    const PREVIEW_ATTACH: &'static str = BeadOperation::PreviewAttach.as_str();
    #[classattr]
    const ATTACH: &'static str = BeadOperation::Attach.as_str();
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
    const RENDER: &'static str = BeadStage::Render.as_str();
    #[classattr]
    const VALIDATE: &'static str = BeadStage::Validate.as_str();
    #[classattr]
    const RESOLVE_ACTIVE_REGISTRY: &'static str = BeadStage::ResolveActiveRegistry.as_str();
    #[classattr]
    const PREVIEW_POUR: &'static str = BeadStage::PreviewPour.as_str();
    #[classattr]
    const POUR: &'static str = BeadStage::Pour.as_str();
    #[classattr]
    const PREVIEW_ATTACH: &'static str = BeadStage::PreviewAttach.as_str();
    #[classattr]
    const ATTACH: &'static str = BeadStage::Attach.as_str();
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
#[derive(Debug)]
struct PyBeadComposeReceipt {
    wire: String,
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
    #[pyo3(get)]
    pour_mode: Option<String>,
    #[pyo3(get)]
    graph: Option<Py<PyAny>>,
}

#[pymethods]
impl PyBeadComposeReceipt {
    /// Decode a versioned Rust receipt into the Python receipt surface.
    #[classmethod]
    fn from_json(class: &Bound<'_, PyType>, receipt_json: &str) -> PyResult<Self> {
        let receipt =
            serde_json::from_str::<BeadComposeReceipt>(receipt_json).map_err(|error| {
                receipt_deserialization_error(
                    class.py(),
                    format!("failed to decode receipt: {error}"),
                )
            })?;
        Self::from_rust(class.py(), receipt)
    }

    /// Return the canonical Rust receipt JSON as Python JSON data.
    fn to_json(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let value = serde_json::from_str(&self.wire).map_err(|error| {
            receipt_deserialization_error(
                py,
                format!("failed to decode receipt for JSON conversion: {error}"),
            )
        })?;
        json_to_py(py, &value)
    }
}

fn stage_outcome(inner: &BeadStageOutcome) -> PyBeadStageOutcome {
    match inner {
        BeadStageOutcome::Succeeded | BeadStageOutcome::Skipped => PyBeadStageOutcome {
            kind: inner.as_str().to_owned(),
            code: None,
        },
        BeadStageOutcome::Failed { code } => PyBeadStageOutcome {
            kind: inner.as_str().to_owned(),
            code: Some(code.clone()),
        },
    }
}

fn stage_receipt(inner: &BeadStageReceipt) -> PyBeadStageReceipt {
    PyBeadStageReceipt {
        stage: inner.stage.as_str().to_owned(),
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
            kind: inner.as_str().to_owned(),
            code: None,
        },
        BeadOutcome::Refused { code } | BeadOutcome::Failed { code } => PyBeadOutcome {
            kind: inner.as_str().to_owned(),
            code: Some(code.clone()),
        },
    }
}

impl PyBeadComposeReceipt {
    fn from_rust(py: Python<'_>, inner: BeadComposeReceipt) -> PyResult<Self> {
        let wire =
            serde_json::to_string(&inner).map_err(|error| request_error(py, error.to_string()))?;
        let graph = inner
            .graph
            .as_ref()
            .map(|graph| {
                let value = serde_json::to_value(graph)
                    .map_err(|error| request_error(py, error.to_string()))?;
                json_to_py(py, &value)
            })
            .transpose()?;
        Ok(Self {
            wire,
            schema: inner.schema,
            operation: inner.operation.as_str().to_owned(),
            rendered_formula: inner.rendered_formula.display().to_string(),
            stages: inner.stages.iter().map(stage_receipt).collect(),
            outcome: outcome(&inner.outcome),
            pour_mode: inner.pour_mode.map(serde_variant_name),
            graph,
        })
    }
}

fn serde_variant_name(mode: BeadPourMode) -> String {
    serde_json::to_value(mode)
        .expect("serializing a serde enum to a JSON value must succeed")
        .as_str()
        .expect("serde enum representation must be a string")
        .to_owned()
}

#[pyclass(name = "BeadComposeRequest", skip_from_py_object)]
#[derive(Clone, Debug)]
struct PyBeadComposeRequest {
    inner: BeadComposeRequest,
}

#[pymethods]
impl PyBeadComposeRequest {
    #[new]
    #[pyo3(signature = (working_directory, template, rendered_formula, compose_variables, *, operation=BeadOperation::Render.as_str(), formula_name=None, bead_variables=None, bd_executable=None, pour_authorization=None, parent=None, r#ref=None, relations=None, schema=BEADS_SCHEMA_V1))]
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
        parent: Option<String>,
        r#ref: Option<String>,
        relations: Option<&Bound<'_, PyAny>>,
        schema: &str,
    ) -> PyResult<Self> {
        let compose_variables = py_to_json(py, compose_variables)?;
        let compose_variables = compose_variables
            .as_object()
            .cloned()
            .ok_or_else(|| request_error(py, "compose_variables must be a string-keyed mapping"))?;
        let pour_authorization = pour_authorization
            .map(PourAuthorization::try_from)
            .transpose()
            .map_err(|error| rust_error_to_pyerr(py, &error))?;
        Ok(Self {
            inner: BeadComposeRequest {
                schema: schema.to_owned(),
                operation: operation_from_str(py, operation)?,
                working_directory: coerce_path(py, working_directory, "working_directory")?,
                template: coerce_path(py, template, "template")?,
                rendered_formula: coerce_path(py, rendered_formula, "rendered_formula")?,
                compose_variables,
                formula_name: formula_name
                    .map(sc_composer_beads::FormulaName::new)
                    .transpose()
                    .map_err(|error| request_error(py, error.to_string()))?,
                bead_variables: parse_bead_variables(py, bead_variables)?,
                bd_executable: bd_executable
                    .map(|value| coerce_path(py, value, "bd_executable"))
                    .transpose()?,
                pour_authorization,
                parent: parent
                    .map(sc_composer_beads::BeadId::new)
                    .transpose()
                    .map_err(|error| request_error(py, error.to_string()))?,
                ref_: r#ref
                    .map(sc_composer_beads::GraphRef::new)
                    .transpose()
                    .map_err(|error| rust_error_to_pyerr(py, &error))?,
                relations: relations
                    .map(|value| {
                        py_to_json(py, value).and_then(|value| {
                            sc_composer_beads::parse_relations(value)
                                .map_err(|error| rust_error_to_pyerr(py, &error))
                        })
                    })
                    .transpose()?
                    .unwrap_or_default(),
            },
        })
    }

    #[getter]
    fn schema(&self) -> String {
        self.inner.schema.clone()
    }

    #[getter]
    fn operation(&self) -> &'static str {
        self.inner.operation.as_str()
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
        self.inner.formula_name.as_ref().map(ToString::to_string)
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

    #[getter]
    fn parent(&self) -> Option<String> {
        self.inner.parent.as_ref().map(ToString::to_string)
    }

    #[getter]
    fn r#ref(&self) -> Option<String> {
        self.inner.ref_.as_ref().map(ToString::to_string)
    }

    #[getter]
    fn relations(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let relations = serde_json::to_value(&self.inner.relations)
            .map_err(|error| request_error(py, error.to_string()))?;
        json_to_py(py, &relations)
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
    let receipt = py
        .detach(|| execute_bead_request(&request))
        .map_err(|error_kind| rust_error_to_pyerr(py, &error_kind))?;
    PyBeadComposeReceipt::from_rust(py, receipt)
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

#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts the Python-owned request through a PyRef argument."
)]
fn preview_attach(
    py: Python<'_>,
    request: PyRef<'_, PyBeadComposeRequest>,
) -> PyResult<PyBeadComposeReceipt> {
    execute_with_operation(py, &request, Some(BeadOperation::PreviewAttach))
}
#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracts the Python-owned request through a PyRef argument."
)]
fn attach(
    py: Python<'_>,
    request: PyRef<'_, PyBeadComposeRequest>,
) -> PyResult<PyBeadComposeReceipt> {
    execute_with_operation(py, &request, Some(BeadOperation::Attach))
}

#[pymodule]
#[pyo3(name = "_native")]
fn native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("BEADS_SCHEMA_V1", BEADS_SCHEMA_V1)?;
    for code in [
        "BEADS_GRAPH_PARENT_NOT_FOUND",
        "BEADS_GRAPH_ID_INVALID",
        "BEADS_GRAPH_SCOPE_MISMATCH",
        "BEADS_GRAPH_FORMULA_UNSUPPORTED",
        "BEADS_GRAPH_RELATION_INVALID",
        "BEADS_GRAPH_CONFLICT",
        "BEADS_GRAPH_EDGE_CONFLICT",
        "BEADS_GRAPH_EDGE_MISSING",
        "BEADS_GRAPH_READ_FAILED",
        "BEADS_GRAPH_APPLY_FAILED",
    ] {
        module.add(code, code)?;
    }
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
    module.add_function(wrap_pyfunction!(preview_attach, module)?)?;
    module.add_function(wrap_pyfunction!(attach, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use serde_json::json;

    #[test]
    fn serde_variant_name_matches_contract_wire_values() {
        for (mode, expected) in [
            (BeadPourMode::Registry, "registry"),
            (BeadPourMode::Graph, "graph"),
        ] {
            let serde_value = serde_json::to_value(mode).expect("pour mode serializes");
            assert_eq!(
                serde_variant_name(mode),
                serde_value
                    .as_str()
                    .expect("pour mode serializes as a string")
            );
            assert_eq!(serde_value.as_str(), Some(expected));
        }
    }

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
            parent: None,
            ref_: None,
            relations: Vec::new(),
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
            assert_eq!(python_receipt.operation, rust_receipt.operation.as_str());
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
