use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::errors::validation_error;

pub(crate) fn coerce_path_like(value: &Bound<'_, PyAny>) -> PyResult<String> {
    let os = value.py().import("os")?;
    os.call_method1("fspath", (value,))?.extract::<String>()
}

pub(super) fn expect_dict<'py>(
    value: &'py Bound<'py, PyAny>,
    message: &str,
) -> PyResult<&'py Bound<'py, PyDict>> {
    value
        .cast::<PyDict>()
        .map_err(|_error| validation_error(message.to_owned(), None))
}

pub(super) fn extract_string_key(value: &Bound<'_, PyAny>, message: &str) -> PyResult<String> {
    value
        .extract::<String>()
        .map_err(|_error| validation_error(message.to_owned(), None))
}
