use std::collections::BTreeMap;

use anyhow::anyhow;
use sc_composer::{
    DiagnosticCode, InputValue, VariableName, input_value_from_yaml, validate_input_value,
};

use crate::CommandError;

use super::decode::{DecodedVarKey, DecodedVarObject, DecodedVarValue};

pub(super) fn validate_var_object(
    object: DecodedVarObject,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::new();
    for entry in object.entries {
        let key = match entry.key {
            DecodedVarKey::String(key) => key,
            DecodedVarKey::Yaml(key) => key
                .as_str()
                .ok_or_else(|| {
                    CommandError::usage_with_code(
                        anyhow!("var-file keys must be strings"),
                        DiagnosticCode::ErrConfigVarfile,
                    )
                })?
                .to_owned(),
        };
        let variable_name = VariableName::new(key.clone()).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!("invalid var-file key `{key}`: {error}"),
                DiagnosticCode::ErrConfigVarfile,
            )
        })?;
        let value = match entry.value {
            DecodedVarValue::Json(value) => {
                validate_input_value(&value).map_err(|error| {
                    CommandError::usage_with_code(
                        anyhow!("invalid var-file value for `{key}`: {error}"),
                        error.code(),
                    )
                })?;
                value
            }
            DecodedVarValue::Yaml(value) => input_value_from_yaml(value).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file value for `{key}`: {error}"),
                    error.code(),
                )
            })?,
        };
        vars.insert(variable_name, value);
    }
    Ok(vars)
}
