use std::collections::BTreeMap;

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList};
use sc_composer::{
    ConfiningRoot, InputValue, MetadataValue, NamedTemplateAsset, ProfileName, RuntimeKind,
    VariableName,
};

use crate::enums::parse_runtime_kind;
use crate::errors::{config_error, validation_error};
use crate::types::{
    PyConfiningRoot, PyNamedTemplateAsset, PyPassConfig, PyProfileName, PyVariableName,
};

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
        roots.push(extract_allowed_root(&item?)?);
    }
    Ok(roots)
}

pub(crate) fn extract_string_map(
    value: &Bound<'_, PyAny>,
) -> PyResult<BTreeMap<String, InputValue>> {
    let dict = expect_dict(value, "context must be a Python dict")?;
    let mut vars = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key = extract_string_key(&key, "context keys must be strings")?;
        let value = validated_input_value(&value)?;
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
    let dict = expect_dict(value, "variable maps must be Python dict instances")?;
    let mut vars = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key = extract_string_key(&key, "variable names must be strings")?;
        let variable = VariableName::new(key.clone()).map_err(|error| {
            validation_error(format!("invalid variable name `{key}`: {error}"), None)
        })?;
        let input = validated_input_value(&value)?;
        vars.insert(variable, input);
    }
    Ok(vars)
}

pub(crate) fn extract_variable_names(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<VariableName>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut variables = Vec::new();
    for item in value.try_iter()? {
        let item = item?;
        if let Ok(variable) = item.extract::<PyRef<'_, PyVariableName>>() {
            variables.push(variable.inner.clone());
            continue;
        }

        let raw = item.extract::<String>().map_err(|_error| {
            validation_error(
                "required_variables entries must be strings or VariableName instances".to_owned(),
                None,
            )
        })?;
        let variable = VariableName::new(raw.clone()).map_err(|error| {
            validation_error(format!("invalid variable name `{raw}`: {error}"), None)
        })?;
        variables.push(variable);
    }
    Ok(variables)
}

pub(crate) fn extract_metadata_map(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<BTreeMap<String, MetadataValue>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let dict = expect_dict(value, "metadata must be a Python dict instance")?;
    let mut metadata = BTreeMap::new();
    for (key, value) in dict.iter() {
        let key = extract_string_key(&key, "metadata keys must be strings")?;
        let json = py_to_json_value(&value)?;
        let metadata_value = serde_json::from_value::<MetadataValue>(json)
            .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))?;
        metadata.insert(key, metadata_value);
    }
    Ok(metadata)
}

pub(crate) fn extract_pass_configs(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<sc_composer::PassConfig>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut passes = Vec::new();
    for item in value.try_iter()? {
        let item = item?;
        let pass = item
            .extract::<PyRef<'_, PyPassConfig>>()
            .map_err(|_error| {
                validation_error(
                    "passes entries must be PassConfig instances".to_owned(),
                    None,
                )
            })?;
        passes.push(pass.inner.clone());
    }
    Ok(passes)
}

pub(crate) fn extract_pass_contexts(
    value: &Bound<'_, PyAny>,
) -> PyResult<Vec<(u8, BTreeMap<VariableName, InputValue>)>> {
    let mut contexts = Vec::new();
    for item in value.try_iter()? {
        let item = item?;
        let pair = item.cast::<pyo3::types::PyTuple>().map_err(|_error| {
            validation_error(
                "contexts entries must be (pass_number, variables) tuples".to_owned(),
                None,
            )
        })?;
        if pair.len() != 2 {
            return Err(validation_error(
                "contexts entries must contain exactly two items".to_owned(),
                None,
            ));
        }
        let pass_number = pair.get_item(0)?.extract::<u8>().map_err(|_error| {
            validation_error("context pass numbers must be integers".to_owned(), None)
        })?;
        let variables = extract_var_map(Some(&pair.get_item(1)?))?;
        contexts.push((pass_number, variables));
    }
    Ok(contexts)
}

pub(crate) fn extract_json_context(value: &Bound<'_, PyAny>) -> PyResult<InputValue> {
    validated_input_value(value)
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

fn extract_allowed_root(value: &Bound<'_, PyAny>) -> PyResult<ConfiningRoot> {
    if let Ok(root) = value.extract::<PyRef<'_, PyConfiningRoot>>() {
        return Ok(root.inner.clone());
    }
    let path = coerce_path_like(value)?;
    ConfiningRoot::new(path)
        .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))
}

fn expect_dict<'py>(
    value: &'py Bound<'py, PyAny>,
    message: &str,
) -> PyResult<&'py Bound<'py, PyDict>> {
    value
        .cast::<PyDict>()
        .map_err(|_error| validation_error(message.to_owned(), None))
}

fn extract_string_key(value: &Bound<'_, PyAny>, message: &str) -> PyResult<String> {
    value
        .extract::<String>()
        .map_err(|_error| validation_error(message.to_owned(), None))
}

fn validated_input_value(value: &Bound<'_, PyAny>) -> PyResult<InputValue> {
    let json = py_to_json_value(value)?;
    sc_composer::validate_input_value(&json).map_err(|error| {
        validation_error(error.message().to_owned(), Some(error.code().as_str()))
    })?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sc-compose-py-convert-{label}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn py_error_details(py: Python<'_>, err: PyErr) -> (String, String, Option<String>) {
        let exc = err.value(py);
        (
            exc.get_type().name().unwrap().to_string(),
            exc.getattr("message").unwrap().extract::<String>().unwrap(),
            exc.getattr("code")
                .unwrap()
                .extract::<Option<String>>()
                .unwrap(),
        )
    }

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
    fn extract_allowed_roots_accepts_pathlike_and_wrapper_instances() {
        Python::initialize();
        Python::attach(|py| {
            let first_root = temp_root("allowed-roots-a");
            let second_root = temp_root("allowed-roots-b");
            let pathlib = py.import("pathlib").unwrap();
            let second_path = pathlib
                .getattr("Path")
                .unwrap()
                .call1((second_root.display().to_string(),))
                .unwrap();
            let wrapped = Py::new(
                py,
                PyConfiningRoot {
                    inner: ConfiningRoot::new(&first_root).unwrap(),
                },
            )
            .unwrap();
            let roots = PyList::empty(py);
            roots.append(wrapped.bind(py)).unwrap();
            roots.append(second_path).unwrap();

            let extracted = extract_allowed_roots(Some(roots.as_any())).unwrap();

            assert_eq!(extracted.len(), 2);
            assert_eq!(extracted[0].as_path(), first_root.canonicalize().unwrap());
            assert_eq!(extracted[1].as_path(), second_root.canonicalize().unwrap());
        });
    }

    #[test]
    fn extract_allowed_roots_rejects_unconfinable_paths() {
        Python::initialize();
        Python::attach(|py| {
            let missing = std::env::temp_dir().join(format!(
                "sc-compose-py-missing-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let roots = PyList::empty(py);
            roots.append(missing.display().to_string()).unwrap();

            let (ty, message, code) =
                py_error_details(py, extract_allowed_roots(Some(roots.as_any())).unwrap_err());

            assert_eq!(ty, "ScConfigError");
            assert!(message.contains("No such file or directory"));
            assert_eq!(code.as_deref(), Some("ERR_CONFIG_PARSE"));
        });
    }

    #[test]
    fn extract_string_map_rejects_non_dict_and_invalid_values() {
        Python::initialize();
        Python::attach(|py| {
            let list = PyList::empty(py);
            let (ty, message, code) =
                py_error_details(py, extract_string_map(list.as_any()).unwrap_err());
            assert_eq!(ty, "ScValidationError");
            assert_eq!(message, "context must be a Python dict");
            assert_eq!(code, None);

            let dict = PyDict::new(py);
            dict.set_item("nested", vec![vec!["nope"]]).unwrap();
            let (ty, message, code) =
                py_error_details(py, extract_string_map(dict.as_any()).unwrap_err());
            assert_eq!(ty, "ScValidationError");
            assert!(message.contains("nested arrays"));
            assert_eq!(code.as_deref(), Some("ERR_VAL_NESTED_ARRAY_UNSUPPORTED"));
        });
    }

    #[test]
    fn extract_json_context_rejects_invalid_nested_arrays() {
        Python::initialize();
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("items", vec![vec!["nope"]]).unwrap();

            let (ty, message, code) =
                py_error_details(py, extract_json_context(dict.as_any()).unwrap_err());

            assert_eq!(ty, "ScValidationError");
            assert!(message.contains("nested arrays"));
            assert_eq!(code.as_deref(), Some("ERR_VAL_NESTED_ARRAY_UNSUPPORTED"));
        });
    }

    #[test]
    fn py_to_json_value_rejects_non_string_object_keys_non_finite_and_unsupported_types() {
        Python::initialize();
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item(7, "value").unwrap();
            let (ty, message, code) =
                py_error_details(py, py_to_json_value(dict.as_any()).unwrap_err());
            assert_eq!(ty, "ScValidationError");
            assert_eq!(message, "object keys must be strings");
            assert_eq!(code, None);

            let inf = py
                .eval(pyo3::ffi::c_str!("float('inf')"), None, None)
                .unwrap();
            let (ty, message, code) = py_error_details(py, py_to_json_value(&inf).unwrap_err());
            assert_eq!(ty, "ScValidationError");
            assert_eq!(message, "floating-point values must be finite");
            assert_eq!(code, None);

            let complex = py
                .eval(pyo3::ffi::c_str!("complex(1, 2)"), None, None)
                .unwrap();
            let (ty, message, code) = py_error_details(py, py_to_json_value(&complex).unwrap_err());
            assert_eq!(ty, "ScValidationError");
            assert!(message.contains("unsupported Python value type"));
            assert_eq!(code, None);
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
