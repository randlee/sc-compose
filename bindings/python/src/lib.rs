mod convert;
mod enums;
mod errors;
mod functions;
mod types;

use pyo3::prelude::*;
use pyo3::types::PyList;
use sc_composer::BUILTIN_VARIABLE_NAMES;

#[pymodule]
#[pyo3(name = "_native")]
fn native(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add(
        "BUILTIN_VARIABLE_NAMES",
        PyList::new(py, BUILTIN_VARIABLE_NAMES)?,
    )?;

    errors::register(module)?;
    enums::register(module)?;
    types::register(module)?;
    functions::register(module)?;
    Ok(())
}
