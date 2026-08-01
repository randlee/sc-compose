use pyo3::prelude::*;
use sc_composer::ConfiningRoot;

use crate::convert::helpers::coerce_path_like;
use crate::errors::config_error;
use crate::types::PyConfiningRoot;

pub(crate) fn extract_allowed_roots(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<ConfiningRoot>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut roots = Vec::new();
    for item in value.try_iter()? {
        roots.push(extract_allowed_root(&item?)?);
    }
    Ok(roots)
}

fn extract_allowed_root(value: &Bound<'_, PyAny>) -> PyResult<ConfiningRoot> {
    if let Ok(root) = value.extract::<PyRef<'_, PyConfiningRoot>>() {
        return Ok(root.inner.clone());
    }
    let path = coerce_path_like(value)?;
    ConfiningRoot::new(path)
        .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))
}
