use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyType;
use sc_composer::{
    ComposeError, ConfigError, DiagnosticCode, ExtractError, IncludeError, OutputCheckError,
    RecoveryHintKind, RenderError, ResolveError, ValidationError,
};

#[derive(Default)]
struct ExceptionDetails {
    recovery_hints: Vec<String>,
    diagnostic_kind: Option<String>,
    diagnostic_message: Option<String>,
    diagnostic_occurrence: Option<usize>,
}

trait ExceptionType {
    fn class(py: Python<'_>) -> PyResult<Bound<'_, PyType>>;
    fn install(py: Python<'_>, class: Bound<'_, PyType>) -> PyResult<()>;
}

static SC_COMPOSE_ERROR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
static SC_RENDER_ERROR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
static SC_VALIDATION_ERROR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
static SC_RESOLVE_ERROR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
static SC_INCLUDE_ERROR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
static SC_CONFIG_ERROR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
static EXCEPTION_TYPES_INITIALIZED: PyOnceLock<()> = PyOnceLock::new();

struct ScComposeError;
struct ScRenderError;
struct ScValidationError;
struct ScResolveError;
struct ScIncludeError;
struct ScConfigError;

impl ExceptionType for ScComposeError {
    fn class(py: Python<'_>) -> PyResult<Bound<'_, PyType>> {
        ensure_exception_types(py)?;
        Ok(SC_COMPOSE_ERROR_TYPE
            .get(py)
            .ok_or_else(|| PyException::new_err("ScComposeError is not registered"))?
            .bind(py)
            .clone())
    }
    fn install(py: Python<'_>, class: Bound<'_, PyType>) -> PyResult<()> {
        SC_COMPOSE_ERROR_TYPE
            .set(py, class.unbind())
            .map_err(|_error| PyException::new_err("ScComposeError registered more than once"))
    }
}
macro_rules! register_exception_type {
    ($ty:ty, $cell:ident, $label:literal) => {
        impl ExceptionType for $ty {
            fn class(py: Python<'_>) -> PyResult<Bound<'_, PyType>> {
                ensure_exception_types(py)?;
                Ok($cell
                    .get(py)
                    .ok_or_else(|| PyException::new_err(concat!($label, " is not registered")))?
                    .bind(py)
                    .clone())
            }
            fn install(py: Python<'_>, class: Bound<'_, PyType>) -> PyResult<()> {
                $cell.set(py, class.unbind()).map_err(|_error| {
                    PyException::new_err(concat!($label, " registered more than once"))
                })
            }
        }
    };
}
register_exception_type!(ScRenderError, SC_RENDER_ERROR_TYPE, "ScRenderError");
register_exception_type!(
    ScValidationError,
    SC_VALIDATION_ERROR_TYPE,
    "ScValidationError"
);
register_exception_type!(ScResolveError, SC_RESOLVE_ERROR_TYPE, "ScResolveError");
register_exception_type!(ScIncludeError, SC_INCLUDE_ERROR_TYPE, "ScIncludeError");
register_exception_type!(ScConfigError, SC_CONFIG_ERROR_TYPE, "ScConfigError");

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    Python::attach(|py| -> PyResult<()> {
        ensure_exception_types(py)?;
        for (name, class) in [
            ("ScComposeError", ScComposeError::class(py)?),
            ("ScRenderError", ScRenderError::class(py)?),
            ("ScValidationError", ScValidationError::class(py)?),
            ("ScResolveError", ScResolveError::class(py)?),
            ("ScIncludeError", ScIncludeError::class(py)?),
            ("ScConfigError", ScConfigError::class(py)?),
        ] {
            module.add(name, class)?;
        }
        Ok(())
    })
}

fn ensure_exception_types(py: Python<'_>) -> PyResult<()> {
    EXCEPTION_TYPES_INITIALIZED.get_or_try_init(py, || -> PyResult<()> {
        let helpers = PyModule::from_code(
            py,
            c"def _sc_compose_error_init(self, *args, **kwargs):\n    if kwargs:\n        raise TypeError(f'{type(self).__name__}() takes no keyword arguments')\n    if not 1 <= len(args) <= 6:\n        raise TypeError(f'{type(self).__name__}() takes from 1 to 6 positional arguments')\n    message = args[0]\n    code = args[1] if len(args) > 1 else None\n    recovery_hints = args[2] if len(args) > 2 else None\n    diagnostic_kind = args[3] if len(args) > 3 else None\n    diagnostic_message = args[4] if len(args) > 4 else None\n    diagnostic_occurrence = args[5] if len(args) > 5 else None\n    if not isinstance(message, str) or (code is not None and not isinstance(code, str)):\n        raise TypeError('message and code must be strings')\n    if recovery_hints is not None and (not isinstance(recovery_hints, list) or not all(isinstance(hint, str) for hint in recovery_hints)):\n        raise TypeError('recovery_hints must be a list of strings or None')\n    if any(value is not None and not isinstance(value, str) for value in (diagnostic_kind, diagnostic_message)):\n        raise TypeError('diagnostic fields must be strings or None')\n    if diagnostic_occurrence is not None and (not isinstance(diagnostic_occurrence, int) or diagnostic_occurrence < 0):\n        raise TypeError('diagnostic_occurrence must be a non-negative integer or None')\n    Exception.__init__(self, *args)\n    self.message = message\n    self.code = code\n    self.recovery_hints = recovery_hints or []\n    self.diagnostic_kind = diagnostic_kind\n    self.diagnostic_message = diagnostic_message\n    self.diagnostic_occurrence = diagnostic_occurrence\nimport inspect as _inspect\n_sc_compose_error_init.__signature__ = _inspect.Signature([_inspect.Parameter('self', _inspect.Parameter.POSITIONAL_OR_KEYWORD), _inspect.Parameter('message', _inspect.Parameter.POSITIONAL_OR_KEYWORD), *[_inspect.Parameter(name, _inspect.Parameter.POSITIONAL_OR_KEYWORD, default=None) for name in ('code', 'recovery_hints', 'diagnostic_kind', 'diagnostic_message', 'diagnostic_occurrence')]])\nScComposeError = type('ScComposeError', (Exception,), {'__module__': 'builtins', '__init__': _sc_compose_error_init})\nScRenderError = type('ScRenderError', (ScComposeError,), {'__module__': 'builtins'})\nScValidationError = type('ScValidationError', (ScComposeError,), {'__module__': 'builtins'})\nScResolveError = type('ScResolveError', (ScComposeError,), {'__module__': 'builtins'})\nScIncludeError = type('ScIncludeError', (ScComposeError,), {'__module__': 'builtins'})\nScConfigError = type('ScConfigError', (ScComposeError,), {'__module__': 'builtins'})\n",
            c"<sc-compose-exceptions>",
            c"sc_compose._native_exception_helpers",
        )?;
        for name in [
            "ScComposeError",
            "ScRenderError",
            "ScValidationError",
            "ScResolveError",
            "ScIncludeError",
            "ScConfigError",
        ] {
            let class = helpers.getattr(name)?.cast_into::<PyType>()?;
            match name {
                "ScComposeError" => ScComposeError::install(py, class.clone())?,
                "ScRenderError" => ScRenderError::install(py, class.clone())?,
                "ScValidationError" => ScValidationError::install(py, class.clone())?,
                "ScResolveError" => ScResolveError::install(py, class.clone())?,
                "ScIncludeError" => ScIncludeError::install(py, class.clone())?,
                "ScConfigError" => ScConfigError::install(py, class.clone())?,
                _ => unreachable!(),
            }
            let _ = class;
        }
        Ok(())
    })?;
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

/// Raise a checked-render rejection with the CLI's diagnostic code
/// (`ERR_RENDER_JSON_MALFORMED`, ...) and its redacted location message.
#[allow(
    clippy::needless_pass_by_value,
    reason = "These helpers accept owned error values from map_err and enum matching."
)]
pub(crate) fn output_check_error_to_pyerr(error: OutputCheckError) -> PyErr {
    let diagnostic = error.diagnostics.first();
    let message = diagnostic.map_or_else(
        || error.to_string(),
        |diagnostic| format!("{error}: {}", diagnostic.message),
    );
    exception_with_attrs::<ScRenderError>(
        &message,
        diagnostic.map(|diagnostic| diagnostic.code.as_str()),
    )
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

#[allow(
    clippy::needless_pass_by_value,
    reason = "Wrapper helpers often build dynamic String messages before constructing Python exceptions."
)]
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

fn exception_with_attrs<T: ExceptionType>(message: &str, code: Option<&str>) -> PyErr {
    exception_with_details::<T>(message, code, ExceptionDetails::default())
}

fn exception_with_details<T: ExceptionType>(
    message: &str,
    code: Option<&str>,
    details: ExceptionDetails,
) -> PyErr {
    Python::attach(|py| {
        let instance = T::class(py)?.call1((
            message,
            code.map(str::to_owned),
            details.recovery_hints,
            details.diagnostic_kind,
            details.diagnostic_message,
            details.diagnostic_occurrence,
        ));
        instance
            .map(|instance| PyErr::from_value(instance.into_any()))
            .map_err(|construction_error| {
                PyException::new_err(format!(
                    "{message} (typed exception construction failed: {construction_error})"
                ))
            })
    })
    .unwrap_or_else(|error| error)
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

    use sc_composer::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot};

    use super::*;

    fn unique_missing_template() -> PathBuf {
        std::env::temp_dir().join(format!("sc-compose-missing-{}.md.j2", std::process::id()))
    }

    #[test]
    fn helper_errors_expose_message_and_code_attributes() {
        Python::initialize();
        Python::attach(|py| {
            let module = PyModule::new(py, "_native").unwrap();
            register(&module).unwrap();
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
