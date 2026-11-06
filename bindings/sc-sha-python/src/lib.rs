//! Thin Python adapter for the two published `sc-sha` operations.

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyBytes, PyDict, PyList, PyType};
use sc_sha::{
    CanonicalSource, CanonicalSourceUrl, CanonicalTemplatePath, CompositionError, HashInput,
    ManifestSchemaError, ManifestSchemaVersion, ResolvedIncludeEdge, ResolvedTemplateManifest,
    ResolvedTemplateNode, TemplateSha256, calculate_composition_hash, calculate_hash,
};

const SC_SHA_ERROR_HELPERS: &str = r"def _sc_sha_error_init(self, *args, **kwargs):
    if kwargs:
        raise TypeError('builtins.ScShaError() takes no keyword arguments')
    if len(args) != 2:
        raise TypeError(f'ScShaError() takes exactly 2 positional arguments ({len(args)} given)')
    code, message = args
    if not isinstance(code, str) or not isinstance(message, str):
        raise TypeError('code and message must be strings')
    Exception.__init__(self, *args)
    self.code = code
    self.message = message

def _sc_sha_error_str(self):
    return self.message

import inspect as _inspect
_sc_sha_error_init.__signature__ = _inspect.Signature([
    _inspect.Parameter('self', _inspect.Parameter.POSITIONAL_OR_KEYWORD),
    _inspect.Parameter('code', _inspect.Parameter.POSITIONAL_OR_KEYWORD),
    _inspect.Parameter('message', _inspect.Parameter.POSITIONAL_OR_KEYWORD),
])
ScShaError = type(
    'ScShaError',
    (Exception,),
    {
        '__module__': 'builtins',
        '__init__': _sc_sha_error_init,
        '__str__': _sc_sha_error_str,
    },
)
";

static SC_SHA_ERROR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();

fn sc_sha_error_type(py: Python<'_>) -> PyResult<Bound<'_, PyType>> {
    if let Some(error_type) = SC_SHA_ERROR_TYPE.get(py) {
        return Ok(error_type.bind(py).clone());
    }

    let source = std::ffi::CString::new(SC_SHA_ERROR_HELPERS)
        .expect("static sc-sha exception helper source contains no null bytes");
    let helpers = PyModule::from_code(
        py,
        source.as_c_str(),
        c"<sc-sha-exceptions>",
        c"sc_sha._native_exception_helpers",
    )?;
    let error_type = helpers.getattr("ScShaError")?.cast_into::<PyType>()?;
    let _ = SC_SHA_ERROR_TYPE.set(py, error_type.clone().unbind());
    Ok(SC_SHA_ERROR_TYPE
        .get(py)
        .expect("ScShaError initialized")
        .bind(py)
        .clone())
}

fn register_error_type(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("ScShaError", sc_sha_error_type(py)?)
}

fn error(py: Python<'_>, code: impl Into<String>, message: impl Into<String>) -> PyErr {
    let code = code.into();
    let message = message.into();
    match sc_sha_error_type(py) {
        Ok(error_type) => PyErr::from_type(error_type, (code, message)),
        Err(error) => PyException::new_err(format!(
            "{message} (typed exception construction failed: {error})"
        )),
    }
}

fn input_dict<'py>(py: Python<'py>, input: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyDict>> {
    input
        .cast::<PyDict>()
        .map_err(|_error| error(py, "SC_SHA_INVALID_INPUT", "hash input must be a mapping"))
        .cloned()
}

fn input_bytes<'py>(py: Python<'py>, input: &Bound<'py, PyAny>) -> PyResult<Vec<u8>> {
    let dict = input_dict(py, input)?;
    let Some(value) = dict.get_item("utf8_file_bytes")? else {
        return Err(error(
            py,
            "SC_SHA_INVALID_INPUT",
            "hash input requires utf8_file_bytes",
        ));
    };
    let bytes = value.cast::<PyBytes>().map_err(|_error| {
        error(
            py,
            "SC_SHA_INVALID_INPUT",
            "utf8_file_bytes must be bytes; encode text as UTF-8 explicitly",
        )
    })?;
    Ok(bytes.as_bytes().to_vec())
}

fn string_field<'py>(py: Python<'py>, dict: &Bound<'py, PyDict>, name: &str) -> PyResult<String> {
    let Some(value) = dict.get_item(name)? else {
        return Err(error(
            py,
            "SC_SHA_INVALID_MANIFEST",
            format!("manifest object requires {name}"),
        ));
    };
    value.extract::<String>().map_err(|_error| {
        error(
            py,
            "SC_SHA_INVALID_MANIFEST",
            format!("manifest field {name} must be a string"),
        )
    })
}

fn parse_source<'py>(py: Python<'py>, value: &Bound<'py, PyAny>) -> PyResult<CanonicalSource> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_error| error(py, "SC_SHA_INVALID_MANIFEST", "source must be a mapping"))?;
    let kind = string_field(py, dict, "kind")?;
    let value = string_field(py, dict, "value")?;
    match kind.as_str() {
        "local_path" => CanonicalTemplatePath::try_from(value)
            .map(CanonicalSource::LocalPath)
            .map_err(|e| error(py, e.code(), e.to_string())),
        "url" => CanonicalSourceUrl::try_from(value)
            .map(CanonicalSource::Url)
            .map_err(|e| error(py, e.code(), e.to_string())),
        _ => Err(error(
            py,
            "SC_SHA_INVALID_MANIFEST",
            "source kind must be local_path or url",
        )),
    }
}

fn parse_digest(value: &str) -> Result<TemplateSha256, sc_sha::ShaError> {
    TemplateSha256::from_hex(value)
}

fn parse_nodes(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Vec<ResolvedTemplateNode>> {
    let nodes = value.cast::<PyList>().map_err(|_error| {
        error(
            py,
            "SC_SHA_INVALID_MANIFEST",
            "manifest nodes must be a list",
        )
    })?;
    let mut parsed_nodes = Vec::with_capacity(nodes.len());
    for value in nodes.iter() {
        let node = value.cast::<PyDict>().map_err(|_error| {
            error(
                py,
                "SC_SHA_INVALID_MANIFEST",
                "manifest node must be a mapping",
            )
        })?;
        let source_value = node.get_item("source")?.ok_or_else(|| {
            error(
                py,
                "SC_SHA_INVALID_MANIFEST",
                "manifest node requires source",
            )
        })?;
        let source = parse_source(py, &source_value)?;
        let digest = parse_digest(&string_field(py, node, "sha256")?)
            .map_err(|parse_error| error(py, parse_error.code(), parse_error.to_string()))?;
        parsed_nodes.push(ResolvedTemplateNode {
            source,
            content_hash: digest,
        });
    }
    Ok(parsed_nodes)
}

fn parse_edges(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Vec<ResolvedIncludeEdge>> {
    let edges = value.cast::<PyList>().map_err(|_error| {
        error(
            py,
            "SC_SHA_INVALID_MANIFEST",
            "manifest edges must be a list",
        )
    })?;
    let mut parsed_edges = Vec::with_capacity(edges.len());
    for value in edges.iter() {
        let edge = value.cast::<PyDict>().map_err(|_error| {
            error(
                py,
                "SC_SHA_INVALID_MANIFEST",
                "manifest edge must be a mapping",
            )
        })?;
        let parent = edge.get_item("parent")?.ok_or_else(|| {
            error(
                py,
                "SC_SHA_INVALID_MANIFEST",
                "manifest edge requires parent",
            )
        })?;
        let child = edge.get_item("child")?.ok_or_else(|| {
            error(
                py,
                "SC_SHA_INVALID_MANIFEST",
                "manifest edge requires child",
            )
        })?;
        let occurrence = edge
            .get_item("occurrence")?
            .ok_or_else(|| {
                error(
                    py,
                    "SC_SHA_INVALID_MANIFEST",
                    "manifest edge requires occurrence",
                )
            })?
            .extract::<u32>()
            .map_err(|_error| {
                error(
                    py,
                    "SC_SHA_INVALID_MANIFEST",
                    "occurrence must be a non-negative integer",
                )
            })?;
        parsed_edges.push(ResolvedIncludeEdge {
            parent: parse_source(py, &parent)?,
            child: parse_source(py, &child)?,
            occurrence,
        });
    }
    Ok(parsed_edges)
}

fn parse_manifest(py: Python<'_>, dict: &Bound<'_, PyDict>) -> PyResult<ResolvedTemplateManifest> {
    let schema_name = string_field(py, dict, "schema")?;
    let schema =
        ManifestSchemaVersion::try_from(schema_name.as_str()).map_err(
            |error_kind| match error_kind {
                ManifestSchemaError::Empty => error(
                    py,
                    "SC_SHA_INVALID_MANIFEST",
                    "manifest schema must not be empty",
                ),
                ManifestSchemaError::Unsupported => error(
                    py,
                    "SC_SHA_UNSUPPORTED_MANIFEST_SCHEMA",
                    "unsupported manifest schema; use sc-sha/manifest/v1",
                ),
            },
        )?;
    let nodes_value = dict.get_item("nodes")?.ok_or_else(|| {
        error(
            py,
            "SC_SHA_INVALID_MANIFEST",
            "manifest object requires nodes",
        )
    })?;
    let edges_value = dict.get_item("edges")?.ok_or_else(|| {
        error(
            py,
            "SC_SHA_INVALID_MANIFEST",
            "manifest object requires edges",
        )
    })?;
    Ok(ResolvedTemplateManifest {
        schema,
        nodes: parse_nodes(py, &nodes_value)?,
        edges: parse_edges(py, &edges_value)?,
    })
}

#[pyfunction]
#[pyo3(name = "calculate_hash")]
fn calculate_hash_py<'py>(
    py: Python<'py>,
    input: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyDict>> {
    let bytes = input_bytes(py, input)?;
    let result = calculate_hash(HashInput::TextFileBytes {
        utf8_file_bytes: &bytes,
    })
    .map_err(|e| error(py, e.code(), e.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("kind", "template")?;
    output.set_item("sha256", result.template().to_hex())?;
    Ok(output)
}

#[pyfunction]
#[pyo3(name = "calculate_composition_hash")]
fn calculate_composition_hash_py<'py>(
    py: Python<'py>,
    input: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = input_dict(py, input)?;
    let manifest = parse_manifest(py, &dict)?;
    let result = calculate_composition_hash(&manifest)
        .map_err(|e: CompositionError| error(py, e.code(), e.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("kind", "composition")?;
    output.set_item("sha256", result.to_hex())?;
    Ok(output)
}

#[pymodule]
#[pyo3(name = "_native")]
fn native(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_error_type(py, module)?;
    module.add_function(wrap_pyfunction!(calculate_hash_py, module)?)?;
    module.add_function(wrap_pyfunction!(calculate_composition_hash_py, module)?)?;
    Ok(())
}
