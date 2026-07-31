use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList};
use sc_composer::InputValue;

use crate::errors::validation_error;

pub(crate) fn extract_json_context(value: &Bound<'_, PyAny>) -> PyResult<InputValue> {
    validated_input_value(value)
}

pub(crate) fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(value) => {
            Ok(PyBool::new(py, *value).to_owned().into_any().unbind())
        }
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(value.into_pyobject(py)?.unbind().into_any())
            } else if let Some(value) = value.as_u64() {
                Ok(value.into_pyobject(py)?.unbind().into_any())
            } else if let Some(value) = value.as_f64() {
                Ok(value.into_pyobject(py)?.unbind().into_any())
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(value) => Ok(value.into_pyobject(py)?.unbind().into_any()),
        serde_json::Value::Array(values) => {
            let list = PyList::empty(py);
            for value in values {
                list.append(json_to_py(py, value)?)?;
            }
            Ok(list.into_any().unbind())
        }
        serde_json::Value::Object(values) => {
            let dict = PyDict::new(py);
            for (key, value) in values {
                dict.set_item(key, json_to_py(py, value)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

pub(crate) fn py_to_json_value(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
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
        let number = serde_json::Number::from_f64(value).ok_or_else(|| {
            validation_error("floating-point values must be finite".to_owned(), None)
        })?;
        return Ok(serde_json::Value::Number(number));
    }
    if let Ok(value) = value.extract::<String>() {
        return Ok(serde_json::Value::String(value));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let mut object = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key = key.extract::<String>().map_err(|_error| {
                validation_error("object keys must be strings".to_owned(), None)
            })?;
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

    Err(validation_error(
        format!(
            "unsupported Python value type for compose input: {}",
            value.get_type().name()?
        ),
        None,
    ))
}

pub(super) fn validated_input_value(value: &Bound<'_, PyAny>) -> PyResult<InputValue> {
    let json = py_to_json_value(value)?;
    sc_composer::validate_input_value(&json).map_err(|error| {
        validation_error(error.message().to_owned(), Some(error.code().as_str()))
    })?;
    Ok(json)
}
