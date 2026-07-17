use std::collections::BTreeMap;

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList};
use sc_composer::{
    ConfiningRoot, InputValue, NamedTemplateAsset, ProfileName, RuntimeKind, VariableName,
};

use crate::enums::parse_runtime_kind;
use crate::errors::{config_error, validation_error};
use crate::types::{PyConfiningRoot, PyNamedTemplateAsset, PyProfileName};

pub(crate) fn extract_supporting_templates(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<NamedTemplateAsset>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut assets = Vec::new();
    for item in value.try_iter()? {
        let item = item?;
        let asset = item.extract::<PyRef<'_, PyNamedTemplateAsset>>()?;
        assets.push(asset.inner.clone());
    }
    Ok(assets)
}

pub(crate) fn extract_profile_name(value: &Bound<'_, PyAny>) -> PyResult<ProfileName> {
    if let Ok(profile_name) = value.extract::<PyRef<'_, PyProfileName>>() {
        return Ok(profile_name.inner.clone());
    }
    let value = value.extract::<String>()?;
    ProfileName::new(value).map_err(|error| config_error(error.to_string(), None))
}

pub(crate) fn extract_runtime_kind(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<RuntimeKind>> {
    value.map(parse_runtime_kind).transpose()
}

pub(crate) fn extract_allowed_roots(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<ConfiningRoot>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut roots = Vec::new();
    for item in value.try_iter()? {
        let item = item?;
        if let Ok(root) = item.extract::<PyRef<'_, PyConfiningRoot>>() {
            roots.push(root.inner.clone());
        } else {
            let path = coerce_path_like(&item)?;
            roots.push(
                ConfiningRoot::new(path)
                    .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))?,
            );
        }
    }
    Ok(roots)
}

pub(crate) fn extract_string_map(
    value: &Bound<'_, PyAny>,
) -> PyResult<BTreeMap<String, InputValue>> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_error| validation_error("context must be a Python dict".to_owned(), None))?;
    let mut vars = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key = key
            .extract::<String>()
            .map_err(|_error| validation_error("context keys must be strings".to_owned(), None))?;
        let value = py_to_json_value(&value)?;
        sc_composer::validate_input_value(&value).map_err(|error| {
            validation_error(error.message().to_owned(), Some(error.code().as_str()))
        })?;
        vars.insert(key, value);
    }
    Ok(vars)
}

pub(crate) fn extract_var_map(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<BTreeMap<VariableName, InputValue>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let dict = value.cast::<PyDict>().map_err(|_error| {
        validation_error(
            "variable maps must be Python dict instances".to_owned(),
            None,
        )
    })?;
    let mut vars = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key = key.extract::<String>().map_err(|_error| {
            validation_error("variable names must be strings".to_owned(), None)
        })?;
        let variable = VariableName::new(key.clone()).map_err(|error| {
            validation_error(format!("invalid variable name `{key}`: {error}"), None)
        })?;
        let input = py_to_json_value(&value)?;
        sc_composer::validate_input_value(&input).map_err(|error| {
            validation_error(error.message().to_owned(), Some(error.code().as_str()))
        })?;
        vars.insert(variable, input);
    }
    Ok(vars)
}

pub(crate) fn extract_json_context(value: &Bound<'_, PyAny>) -> PyResult<InputValue> {
    let json = py_to_json_value(value)?;
    sc_composer::validate_input_value(&json).map_err(|error| {
        validation_error(error.message().to_owned(), Some(error.code().as_str()))
    })?;
    Ok(json)
}

pub(crate) fn coerce_path_like(value: &Bound<'_, PyAny>) -> PyResult<String> {
    let os = value.py().import("os")?;
    os.call_method1("fspath", (value,))?.extract::<String>()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn py_value_round_trips_nested_json_shapes() {
        Python::initialize();
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("flag", true).unwrap();
            dict.set_item("items", vec!["a", "b"]).unwrap();

            let nested = PyDict::new(py);
            nested.set_item("count", 3).unwrap();
            dict.set_item("nested", nested).unwrap();

            let json = py_to_json_value(dict.as_any()).unwrap();
            assert_eq!(
                json,
                serde_json::json!({
                    "flag": true,
                    "items": ["a", "b"],
                    "nested": {"count": 3},
                })
            );

            let roundtrip = py_to_json_value(json_to_py(py, &json).unwrap().bind(py)).unwrap();
            assert_eq!(roundtrip, json);
        });
    }

    #[test]
    fn extract_var_map_rejects_invalid_variable_names() {
        Python::initialize();
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("", "value").unwrap();

            let err = extract_var_map(Some(dict.as_any())).unwrap_err();
            let exc = err.value(py);
            let message = exc.getattr("message").unwrap().extract::<String>().unwrap();
            let code = exc
                .getattr("code")
                .unwrap()
                .extract::<Option<String>>()
                .unwrap();

            assert_eq!(exc.get_type().name().unwrap(), "ScValidationError");
            assert!(message.contains("invalid variable name ``"));
            assert_eq!(code, None);
        });
    }
}
