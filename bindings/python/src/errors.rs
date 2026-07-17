use pyo3::PyTypeInfo;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use sc_composer::{
    ComposeError, ConfigError, DiagnosticCode, IncludeError, RenderError, ResolveError,
    ValidationError,
};

#[pyclass(extends=PyException, subclass, name = "ScComposeError")]
#[derive(Debug)]
pub(crate) struct ScComposeError {
    #[pyo3(get)]
    message: String,
    #[pyo3(get)]
    code: Option<String>,
}

impl ScComposeError {
    fn new_inner(message: String, code: Option<String>) -> Self {
        Self { message, code }
    }
}

#[pymethods]
impl ScComposeError {
    #[new]
    #[pyo3(signature = (message, code=None))]
    fn new(message: String, code: Option<String>) -> Self {
        Self::new_inner(message, code)
    }
}

#[pyclass(extends=ScComposeError, name = "ScRenderError")]
#[derive(Debug)]
pub(crate) struct ScRenderError;

#[pymethods]
impl ScRenderError {
    #[new]
    #[pyo3(signature = (message, code=None))]
    fn new(message: String, code: Option<String>) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(message, code)).add_subclass(Self)
    }
}

#[pyclass(extends=ScComposeError, name = "ScValidationError")]
#[derive(Debug)]
pub(crate) struct ScValidationError;

#[pymethods]
impl ScValidationError {
    #[new]
    #[pyo3(signature = (message, code=None))]
    fn new(message: String, code: Option<String>) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(message, code)).add_subclass(Self)
    }
}

#[pyclass(extends=ScComposeError, name = "ScResolveError")]
#[derive(Debug)]
pub(crate) struct ScResolveError;

#[pymethods]
impl ScResolveError {
    #[new]
    #[pyo3(signature = (message, code=None))]
    fn new(message: String, code: Option<String>) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(message, code)).add_subclass(Self)
    }
}

#[pyclass(extends=ScComposeError, name = "ScIncludeError")]
#[derive(Debug)]
pub(crate) struct ScIncludeError;

#[pymethods]
impl ScIncludeError {
    #[new]
    #[pyo3(signature = (message, code=None))]
    fn new(message: String, code: Option<String>) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(message, code)).add_subclass(Self)
    }
}

#[pyclass(extends=ScComposeError, name = "ScConfigError")]
#[derive(Debug)]
pub(crate) struct ScConfigError;

#[pymethods]
impl ScConfigError {
    #[new]
    #[pyo3(signature = (message, code=None))]
    fn new(message: String, code: Option<String>) -> PyClassInitializer<Self> {
        PyClassInitializer::from(ScComposeError::new_inner(message, code)).add_subclass(Self)
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

fn exception_with_attrs<T>(message: &str, code: Option<&str>) -> PyErr
where
    T: PyTypeInfo,
{
    Python::attach(|py| {
        let instance = py
            .get_type::<T>()
            .call1((message, code.map(str::to_owned)))
            .expect("exception construction");
        PyErr::from_value(instance.into_any())
    })
}
