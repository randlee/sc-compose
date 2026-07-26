use std::collections::BTreeMap;

use pyo3::prelude::*;
use sc_composer::{InputValue, NamedTemplateAsset, ProfileName, RuntimeKind, VariableName};

use crate::convert::maps::extract_var_map;
use crate::enums::parse_runtime_kind;
use crate::errors::{config_error, validation_error};
use crate::types::{PyNamedTemplateAsset, PyPassConfig, PyProfileName, PyVariableName};

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
