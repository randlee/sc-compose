use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use sc_composer::ConfiningRoot;

use super::{
    extract_allowed_roots, extract_json_context, extract_string_map, extract_var_map, json_to_py,
    py_to_json_value,
};
use crate::types::PyConfiningRoot;

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

fn py_error_details(py: Python<'_>, err: &PyErr) -> (String, String, Option<String>) {
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

        let (ty, _message, code) = py_error_details(
            py,
            &extract_allowed_roots(Some(roots.as_any())).unwrap_err(),
        );

        assert_eq!(ty, "ScConfigError");
        assert_eq!(code.as_deref(), Some("ERR_CONFIG_PARSE"));
    });
}

#[test]
fn extract_string_map_rejects_non_dict_and_accepts_recursive_values() {
    Python::initialize();
    Python::attach(|py| {
        let list = PyList::empty(py);
        let (ty, message, code) =
            py_error_details(py, &extract_string_map(list.as_any()).unwrap_err());
        assert_eq!(ty, "ScValidationError");
        assert_eq!(message, "context must be a Python dict");
        assert_eq!(code, None);

        let dict = PyDict::new(py);
        dict.set_item("nested", vec![vec!["nope"]]).unwrap();
        let extracted = extract_string_map(dict.as_any()).unwrap();
        assert_eq!(
            extracted.get("nested"),
            Some(&serde_json::json!([["nope"]]))
        );
    });
}

#[test]
fn extract_json_context_accepts_recursive_arrays() {
    Python::initialize();
    Python::attach(|py| {
        let dict = PyDict::new(py);
        dict.set_item("items", vec![vec!["nope"]]).unwrap();

        let extracted = extract_json_context(dict.as_any()).unwrap();
        assert_eq!(extracted, serde_json::json!({"items": [["nope"]]}));
    });
}

#[test]
fn py_to_json_value_rejects_non_string_object_keys_non_finite_and_unsupported_types() {
    Python::initialize();
    Python::attach(|py| {
        let dict = PyDict::new(py);
        dict.set_item(7, "value").unwrap();
        let (ty, message, code) =
            py_error_details(py, &py_to_json_value(dict.as_any()).unwrap_err());
        assert_eq!(ty, "ScValidationError");
        assert_eq!(message, "object keys must be strings");
        assert_eq!(code, None);

        let inf = py
            .eval(pyo3::ffi::c_str!("float('inf')"), None, None)
            .unwrap();
        let (ty, message, code) = py_error_details(py, &py_to_json_value(&inf).unwrap_err());
        assert_eq!(ty, "ScValidationError");
        assert_eq!(message, "floating-point values must be finite");
        assert_eq!(code, None);

        let complex = py
            .eval(pyo3::ffi::c_str!("complex(1, 2)"), None, None)
            .unwrap();
        let (ty, message, code) = py_error_details(py, &py_to_json_value(&complex).unwrap_err());
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
