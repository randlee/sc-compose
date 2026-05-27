use std::collections::BTreeMap;
use std::path::Path;

use anyhow::anyhow;
use sc_composer::{
    DiagnosticCode, InputValue, VariableName, input_value_from_yaml, validate_input_value,
};

use crate::CommandError;

pub(crate) fn load_var_file(
    path: &Path,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let contents = std::fs::read_to_string(path).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!("failed to read var-file {}", path.display())),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    parse_var_file_contents(&contents)
}

pub(crate) fn parse_var_file_contents(
    contents: &str,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) {
        return parse_json_object_value(&value);
    }
    let value = serde_yaml::from_str::<serde_yaml::Value>(contents).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context("var-file must be valid JSON or YAML"),
            DiagnosticCode::ErrConfigParse,
        )
    })?;
    let serde_yaml::Value::Mapping(object) = value else {
        return Err(CommandError::usage_with_code(
            anyhow!("var-file must be a JSON or YAML object"),
            DiagnosticCode::ErrConfigVarfile,
        ));
    };

    let mut vars = BTreeMap::new();
    for (key, value) in object {
        let key = key.as_str().ok_or_else(|| {
            CommandError::usage_with_code(
                anyhow!("var-file keys must be strings"),
                DiagnosticCode::ErrConfigVarfile,
            )
        })?;
        vars.insert(
            VariableName::new(key.to_owned()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file key `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
            input_value_from_yaml(value).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file value for `{key}`: {error}"),
                    error.code(),
                )
            })?,
        );
    }
    Ok(vars)
}

fn parse_json_object_value(
    value: &serde_json::Value,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let object = value.as_object().ok_or_else(|| {
        CommandError::usage_with_code(
            anyhow!("var-file must be a JSON object"),
            DiagnosticCode::ErrConfigVarfile,
        )
    })?;
    let mut vars = BTreeMap::new();
    for (key, value) in object {
        vars.insert(
            VariableName::new(key.clone()).map_err(|error| {
                CommandError::usage_with_code(
                    anyhow!("invalid var-file key `{key}`: {error}"),
                    DiagnosticCode::ErrConfigVarfile,
                )
            })?,
            {
                validate_input_value(value).map_err(|error| {
                    CommandError::usage_with_code(
                        anyhow!("invalid var-file value for `{key}`: {error}"),
                        error.code(),
                    )
                })?;
                value.clone()
            },
        );
    }
    Ok(vars)
}
