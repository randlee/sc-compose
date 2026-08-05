use std::collections::BTreeMap;

use pyo3::prelude::*;
use sc_composer::{InputValue, MetadataValue, VariableName};

use crate::convert::helpers::{expect_dict, extract_string_key};
use crate::convert::values::{py_to_json_value, validated_input_value};
use crate::errors::{config_error, validation_error};

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
