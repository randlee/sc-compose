use std::ffi::CString;

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

/// The six exception classes, resolved together so installation is
/// all-or-nothing: a failure while loading any class caches nothing and the
/// next call retries from scratch.
struct ExceptionTypes {
    compose: Py<PyType>,
    render: Py<PyType>,
    validation: Py<PyType>,
    resolve: Py<PyType>,
    include: Py<PyType>,
    config: Py<PyType>,
}

static EXCEPTION_TYPES: PyOnceLock<ExceptionTypes> = PyOnceLock::new();

/// Python source of the classes, kept as a real file so it is reviewed and
/// linted as Python.
const EXCEPTION_HELPERS: &str = include_str!("exceptions.py");

impl ExceptionTypes {
    fn get(py: Python<'_>) -> PyResult<&Self> {
        EXCEPTION_TYPES.get_or_try_init(py, || Self::load(py))
    }

    fn load(py: Python<'_>) -> PyResult<Self> {
        let source = CString::new(EXCEPTION_HELPERS)
            .map_err(|error| PyException::new_err(format!("exception helper source: {error}")))?;
        let helpers = PyModule::from_code(
            py,
            &source,
            c"<sc-compose-exceptions>",
            c"sc_compose._native_exception_helpers",
        )?;
        let class = |name: &str| -> PyResult<Py<PyType>> {
            Ok(helpers.getattr(name)?.cast_into::<PyType>()?.unbind())
        };
        Ok(Self {
            compose: class("ScComposeError")?,
            render: class("ScRenderError")?,
            validation: class("ScValidationError")?,
            resolve: class("ScResolveError")?,
            include: class("ScIncludeError")?,
            config: class("ScConfigError")?,
        })
    }

    /// Public module names paired with their classes, in registration order.
    fn named(&self) -> [(&'static str, &Py<PyType>); 6] {
        [
            ("ScComposeError", &self.compose),
            ("ScRenderError", &self.render),
            ("ScValidationError", &self.validation),
            ("ScResolveError", &self.resolve),
            ("ScIncludeError", &self.include),
            ("ScConfigError", &self.config),
        ]
    }
}

/// Selects one class from the resolved table. Each marker type names a field,
/// so the compiler checks the mapping instead of a string match.
trait ExceptionType {
    fn select(types: &ExceptionTypes) -> &Py<PyType>;
}

macro_rules! exception_type {
    ($ty:ident, $field:ident) => {
        struct $ty;
        impl ExceptionType for $ty {
            fn select(types: &ExceptionTypes) -> &Py<PyType> {
                &types.$field
            }
        }
    };
}
exception_type!(ScRenderError, render);
exception_type!(ScValidationError, validation);
exception_type!(ScResolveError, resolve);
exception_type!(ScIncludeError, include);
exception_type!(ScConfigError, config);

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = module.py();
    for (name, class) in ExceptionTypes::get(py)?.named() {
        module.add(name, class.bind(py))?;
    }
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
        ExceptionTypes::get(py)
            .and_then(|types| {
                T::select(types).bind(py).call1((
                    message,
                    code.map(str::to_owned),
                    details.recovery_hints,
                    details.diagnostic_kind,
                    details.diagnostic_message,
                    details.diagnostic_occurrence,
                ))
            })
            .map_or_else(
                |construction_error| fallback_exception(py, message, code, &construction_error),
                |instance| PyErr::from_value(instance.into_any()),
            )
    })
}

/// The adapter's one fallback when a typed class cannot be loaded or
/// constructed: a plain `Exception` that still carries the stable `code` and
/// `message` as attributes and in its text, with the construction failure.
fn fallback_exception(
    py: Python<'_>,
    message: &str,
    code: Option<&str>,
    construction_error: &PyErr,
) -> PyErr {
    let label = code.map_or_else(String::new, |code| format!(" [{code}]"));
    let error = PyException::new_err(format!(
        "{message}{label} (typed exception construction failed: {construction_error})"
    ));
    let value = error.value(py);
    match value
        .setattr("message", message)
        .and_then(|()| value.setattr("code", code))
    {
        Ok(()) => error,
        Err(attribute_error) => attribute_error,
    }
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

    /// A class that cannot be loaded is simulated with a controlled
    /// construction error; the fallback keeps code and message.
    #[test]
    fn fallback_exception_keeps_code_and_message() {
        Python::initialize();
        Python::attach(|py| {
            let cause = pyo3::exceptions::PyImportError::new_err("helper import failed");
            let error = fallback_exception(py, "bad input", Some("ERR_VAL_EMPTY"), &cause);
            let value = error.value(py);
            assert_eq!(value.get_type().name().unwrap(), "Exception");
            assert_eq!(
                value
                    .getattr("message")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "bad input"
            );
            assert_eq!(
                value
                    .getattr("code")
                    .unwrap()
                    .extract::<Option<String>>()
                    .unwrap(),
                Some("ERR_VAL_EMPTY".to_owned())
            );
            let text = value.str().unwrap().to_string();
            assert!(text.starts_with("bad input [ERR_VAL_EMPTY]"), "{text}");
            assert!(text.contains("helper import failed"), "{text}");

            let error = fallback_exception(py, "no code", None, &cause);
            let value = error.value(py);
            assert!(value.getattr("code").unwrap().is_none());
            assert!(
                value
                    .str()
                    .unwrap()
                    .to_string()
                    .starts_with("no code (typed")
            );
        });
    }

    /// The table loads all six classes from `exceptions.py` at once; every
    /// specific class derives from `ScComposeError` and is registered under
    /// its own name.
    #[test]
    fn exception_table_registers_six_classes_from_the_python_file() {
        Python::initialize();
        Python::attach(|py| {
            let module = PyModule::new(py, "_native").unwrap();
            register(&module).unwrap();
            let types = ExceptionTypes::get(py).unwrap();
            let base = types.compose.bind(py);
            for (name, class) in types.named() {
                let class = class.bind(py);
                assert_eq!(class.name().unwrap(), name);
                assert_eq!(class.module().unwrap(), "builtins");
                assert!(class.is_subclass(base).unwrap(), "{name}");
                assert!(module.getattr(name).unwrap().is(class), "{name}");
            }
            assert!(EXCEPTION_HELPERS.lines().count() > 1);
        });
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
