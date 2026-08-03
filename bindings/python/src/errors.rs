use pyo3::PyTypeInfo;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use sc_composer::{
    ComposeError, ConfigError, DiagnosticCode, ExtractError, IncludeError, RecoveryHintKind,
    RenderError, ResolveError, ValidationError,
};

#[derive(Default)]
struct ExceptionDetails {
    recovery_hints: Vec<String>,
    diagnostic_kind: Option<String>,
    diagnostic_message: Option<String>,
    diagnostic_occurrence: Option<usize>,
}

fn exception_details(
    recovery_hints: Option<Vec<String>>,
    diagnostic_kind: Option<String>,
    diagnostic_message: Option<String>,
    diagnostic_occurrence: Option<usize>,
) -> ExceptionDetails {
    ExceptionDetails {
        recovery_hints: recovery_hints.unwrap_or_default(),
        diagnostic_kind,
        diagnostic_message,
        diagnostic_occurrence,
    }
}

#[pyclass(extends=PyException, subclass, name = "ScComposeError")]
#[derive(Debug)]
pub(crate) struct ScComposeError {
    #[pyo3(get)]
    message: String,
    #[pyo3(get)]
    code: Option<String>,
    #[pyo3(get)]
    recovery_hints: Vec<String>,
    #[pyo3(get)]
    diagnostic_kind: Option<String>,
    #[pyo3(get)]
    diagnostic_message: Option<String>,
    #[pyo3(get)]
    diagnostic_occurrence: Option<usize>,
}

impl ScComposeError {
    fn new_inner(message: String, code: Option<String>, details: ExceptionDetails) -> Self {
        Self {
            message,
            code,
            recovery_hints: details.recovery_hints,
            diagnostic_kind: details.diagnostic_kind,
            diagnostic_message: details.diagnostic_message,
            diagnostic_occurrence: details.diagnostic_occurrence,
        }
    }
}

#[pymethods]
impl ScComposeError {
    #[new]
    #[pyo3(signature = (message, code=None, recovery_hints=None, diagnostic_kind=None, diagnostic_message=None, diagnostic_occurrence=None))]
    fn new(
        message: String,
        code: Option<String>,
        recovery_hints: Option<Vec<String>>,
        diagnostic_kind: Option<String>,
        diagnostic_message: Option<String>,
        diagnostic_occurrence: Option<usize>,
    ) -> Self {
        Self::new_inner(
            message,
            code,
            exception_details(
                recovery_hints,
                diagnostic_kind,
                diagnostic_message,
                diagnostic_occurrence,
            ),
        )
    }
}

#[pyclass(extends=ScComposeError, name = "ScRenderError")]
#[derive(Debug)]
pub(crate) struct ScRenderError;

#[pymethods]
impl ScRenderError {
    #[new]
    #[pyo3(signature = (message, code=None, recovery_hints=None, diagnostic_kind=None, diagnostic_message=None, diagnostic_occurrence=None))]
    fn new(
        message: String,
        code: Option<String>,
        recovery_hints: Option<Vec<String>>,
        diagnostic_kind: Option<String>,
        diagnostic_message: Option<String>,
        diagnostic_occurrence: Option<usize>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(
            message,
            code,
            exception_details(
                recovery_hints,
                diagnostic_kind,
                diagnostic_message,
                diagnostic_occurrence,
            ),
        ))
        .add_subclass(Self)
    }
}

#[pyclass(extends=ScComposeError, name = "ScValidationError")]
#[derive(Debug)]
pub(crate) struct ScValidationError;

#[pymethods]
impl ScValidationError {
    #[new]
    #[pyo3(signature = (message, code=None, recovery_hints=None, diagnostic_kind=None, diagnostic_message=None, diagnostic_occurrence=None))]
    fn new(
        message: String,
        code: Option<String>,
        recovery_hints: Option<Vec<String>>,
        diagnostic_kind: Option<String>,
        diagnostic_message: Option<String>,
        diagnostic_occurrence: Option<usize>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(
            message,
            code,
            exception_details(
                recovery_hints,
                diagnostic_kind,
                diagnostic_message,
                diagnostic_occurrence,
            ),
        ))
        .add_subclass(Self)
    }
}

#[pyclass(extends=ScComposeError, name = "ScResolveError")]
#[derive(Debug)]
pub(crate) struct ScResolveError;

#[pymethods]
impl ScResolveError {
    #[new]
    #[pyo3(signature = (message, code=None, recovery_hints=None, diagnostic_kind=None, diagnostic_message=None, diagnostic_occurrence=None))]
    fn new(
        message: String,
        code: Option<String>,
        recovery_hints: Option<Vec<String>>,
        diagnostic_kind: Option<String>,
        diagnostic_message: Option<String>,
        diagnostic_occurrence: Option<usize>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(
            message,
            code,
            exception_details(
                recovery_hints,
                diagnostic_kind,
                diagnostic_message,
                diagnostic_occurrence,
            ),
        ))
        .add_subclass(Self)
    }
}

#[pyclass(extends=ScComposeError, name = "ScIncludeError")]
#[derive(Debug)]
pub(crate) struct ScIncludeError;

#[pymethods]
impl ScIncludeError {
    #[new]
    #[pyo3(signature = (message, code=None, recovery_hints=None, diagnostic_kind=None, diagnostic_message=None, diagnostic_occurrence=None))]
    fn new(
        message: String,
        code: Option<String>,
        recovery_hints: Option<Vec<String>>,
        diagnostic_kind: Option<String>,
        diagnostic_message: Option<String>,
        diagnostic_occurrence: Option<usize>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(
            message,
            code,
            exception_details(
                recovery_hints,
                diagnostic_kind,
                diagnostic_message,
                diagnostic_occurrence,
            ),
        ))
        .add_subclass(Self)
    }
}

#[pyclass(extends=ScComposeError, name = "ScConfigError")]
#[derive(Debug)]
pub(crate) struct ScConfigError;

#[pymethods]
impl ScConfigError {
    #[new]
    #[pyo3(signature = (message, code=None, recovery_hints=None, diagnostic_kind=None, diagnostic_message=None, diagnostic_occurrence=None))]
    fn new(
        message: String,
        code: Option<String>,
        recovery_hints: Option<Vec<String>>,
        diagnostic_kind: Option<String>,
        diagnostic_message: Option<String>,
        diagnostic_occurrence: Option<usize>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(
            message,
            code,
            exception_details(
                recovery_hints,
                diagnostic_kind,
                diagnostic_message,
                diagnostic_occurrence,
            ),
        ))
        .add_subclass(Self)
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ScComposeError>()?;
    module.add_class::<ScRenderError>()?;
    module.add_class::<ScValidationError>()?;
    module.add_class::<ScResolveError>()?;
    module.add_class::<ScIncludeError>()?;
    module.add_class::<ScConfigError>()?;
    Ok(())
}

pub(crate) fn compose_error_to_pyerr(error: ComposeError) -> PyErr {
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
pub(crate) fn resolve_error_to_pyerr(error: ResolveError) -> PyErr {
    exception_with_attrs::<ScResolveError>(error.message(), Some(error.code().as_str()))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
pub(crate) fn include_error_to_pyerr(error: IncludeError) -> PyErr {
    exception_with_attrs::<ScIncludeError>(error.message(), Some(error.code().as_str()))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
pub(crate) fn validation_error_to_pyerr(error: ValidationError) -> PyErr {
    exception_with_attrs::<ScValidationError>(error.message(), Some(error.code().as_str()))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
pub(crate) fn render_error_to_pyerr(error: RenderError) -> PyErr {
    exception_with_attrs::<ScRenderError>(error.message(), error.code().map(DiagnosticCode::as_str))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
pub(crate) fn config_error_to_pyerr(error: ConfigError) -> PyErr {
    exception_with_attrs::<ScConfigError>(error.message(), Some(error.code().as_str()))
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "Wrapper helpers often build dynamic String messages before constructing Python exceptions."
)]
pub(crate) fn validation_error(message: String, code: Option<&str>) -> PyErr {
    exception_with_attrs::<ScValidationError>(&message, code)
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "Wrapper helpers often build dynamic String messages before constructing Python exceptions."
)]
pub(crate) fn config_error(message: String, code: Option<&str>) -> PyErr {
    exception_with_attrs::<ScConfigError>(&message, code)
}

pub(crate) fn config_error_with_recovery_hints(
    message: String,
    code: Option<&str>,
    recovery_hints: Vec<String>,
) -> PyErr {
    exception_with_details::<ScConfigError>(
        &message,
        code,
        ExceptionDetails {
            recovery_hints,
            ..ExceptionDetails::default()
        },
    )
}

/// Map the pure extraction contract onto the adapter's existing configuration
/// error family. Extraction is an in-memory input operation, and its stable
/// Rust diagnostic code remains available through the Python exception.
#[allow(
    clippy::needless_pass_by_value,
    reason = "This helper accepts the owned error produced by map_err."
)]
pub(crate) fn extract_error_to_pyerr(error: ExtractError) -> PyErr {
    let diagnostic = error.diagnostic();
    let details = ExceptionDetails {
        recovery_hints: error
            .recovery_hints()
            .iter()
            .map(recovery_hint_description)
            .collect(),
        diagnostic_kind: diagnostic.map(|diagnostic| extraction_diagnostic_kind(diagnostic.kind)),
        diagnostic_message: diagnostic.map(|diagnostic| diagnostic.message.clone()),
        diagnostic_occurrence: diagnostic
            .and_then(|diagnostic| diagnostic.occurrence.map(|index| index.0)),
    };
    exception_with_details::<ScConfigError>(
        &extract_error_message(&error),
        Some(error.code().as_str()),
        details,
    )
}

fn exception_with_attrs<T>(message: &str, code: Option<&str>) -> PyErr
where
    T: PyTypeInfo,
{
    exception_with_details::<T>(message, code, ExceptionDetails::default())
}

fn exception_with_details<T>(message: &str, code: Option<&str>, details: ExceptionDetails) -> PyErr
where
    T: PyTypeInfo,
{
    Python::attach(|py| {
        let instance = py.get_type::<T>().call1((
            message,
            code.map(str::to_owned),
            details.recovery_hints,
            details.diagnostic_kind,
            details.diagnostic_message,
            details.diagnostic_occurrence,
        ));
        match instance {
            Ok(instance) => PyErr::from_value(instance.into_any()),
            Err(construction_error) => PyException::new_err(format!(
                "{message} (typed exception construction failed: {construction_error})"
            )),
        }
    })
}

fn extract_error_message(error: &ExtractError) -> String {
    match error {
        ExtractError::InvalidRequest { message, .. } => message.clone(),
        ExtractError::MalformedXml { diagnostic, .. }
        | ExtractError::UnsupportedSyntax { diagnostic, .. }
        | ExtractError::AmbiguousStructure { diagnostic, .. }
        | ExtractError::FormatError { diagnostic, .. } => diagnostic.message.clone(),
    }
}

fn extraction_diagnostic_kind(kind: sc_composer::ExtractionDiagnosticKind) -> String {
    match kind {
        sc_composer::ExtractionDiagnosticKind::Unsupported => "unsupported",
        sc_composer::ExtractionDiagnosticKind::Ambiguous => "ambiguous",
        sc_composer::ExtractionDiagnosticKind::NotObserved => "not_observed",
        sc_composer::ExtractionDiagnosticKind::Malformed => "malformed",
    }
    .to_owned()
}

fn recovery_hint_description(hint: &sc_composer::RecoveryHint) -> String {
    match &hint.kind {
        RecoveryHintKind::RunCommand { command } => format!("run command: {command}"),
        RecoveryHintKind::InspectPath { path } => format!("inspect path: {}", path.display()),
        RecoveryHintKind::ProvideVariable { variable } => {
            format!("provide variable: {variable}")
        }
        RecoveryHintKind::ReviewConfiguration { key } => {
            format!("review configuration: {key}")
        }
        RecoveryHintKind::InspectInput { description }
        | RecoveryHintKind::DisambiguateOccurrences { description }
        | RecoveryHintKind::UnsupportedConstruct { description } => description.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use sc_composer::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot};

    use super::*;

    fn unique_missing_template() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("sc-compose-missing-{nanos}.md.j2"))
    }

    #[test]
    fn helper_errors_expose_message_and_code_attributes() {
        Python::initialize();
        Python::attach(|py| {
            let err = validation_error("bad input".to_owned(), Some("ERR_VAL_EMPTY"));
            let exc = err.value(py);
            assert_eq!(exc.get_type().name().unwrap(), "ScValidationError");
            assert_eq!(
                exc.getattr("message").unwrap().extract::<String>().unwrap(),
                "bad input"
            );
            assert_eq!(
                exc.getattr("code")
                    .unwrap()
                    .extract::<Option<String>>()
                    .unwrap(),
                Some("ERR_VAL_EMPTY".to_owned())
            );

            let err = config_error("bad config".to_owned(), Some("ERR_CONFIG_PARSE"));
            let exc = err.value(py);
            assert_eq!(exc.get_type().name().unwrap(), "ScConfigError");
            assert_eq!(
                exc.getattr("code")
                    .unwrap()
                    .extract::<Option<String>>()
                    .unwrap(),
                Some("ERR_CONFIG_PARSE".to_owned())
            );
        });
    }

    #[test]
    fn compose_error_to_pyerr_maps_resolve_errors_to_scresolveerror() {
        let root_path = std::env::temp_dir();
        let request = ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: unique_missing_template(),
            },
            root: ConfiningRoot::new(&root_path).unwrap(),
            vars_input: BTreeMap::new(),
            vars_env: BTreeMap::new(),
            vars_defaults: BTreeMap::new(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        };

        let err = sc_composer::resolve_template_path(&request).unwrap_err();
        Python::initialize();
        Python::attach(|py| {
            let pyerr = compose_error_to_pyerr(err);
            let exc = pyerr.value(py);
            assert_eq!(exc.get_type().name().unwrap(), "ScResolveError");
            assert_eq!(
                exc.getattr("code")
                    .unwrap()
                    .extract::<Option<String>>()
                    .unwrap(),
                Some("ERR_RESOLVE_NOT_FOUND".to_owned())
            );
        });
    }
}
