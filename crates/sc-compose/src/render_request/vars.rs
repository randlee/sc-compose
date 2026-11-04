use std::collections::BTreeMap;
use std::io::Read;

use anyhow::anyhow;
use sc_composer::{
    BUILTIN_VARIABLE_NAMES, DiagnosticCode, InputValue, PassConfig, RecoveryHint, RecoveryHintKind,
    UnknownVariablePolicy, VariableName,
};

use crate::CommandError;
use crate::cli::{InputArgs, PassInputArgs, UnknownVarMode};
use crate::var_file::{load_var_file, parse_var_file_contents};

pub(super) fn compose_policy(input: &InputArgs) -> sc_composer::ComposePolicy {
    sc_composer::ComposePolicy {
        strict_undeclared_variables: input.strict,
        unknown_variable_policy: match input.unknown_var_mode {
            UnknownVarMode::Error => UnknownVariablePolicy::Error,
            UnknownVarMode::Warn => UnknownVariablePolicy::Warn,
            UnknownVarMode::Ignore => UnknownVariablePolicy::Ignore,
        },
        unbound_variable_policy: Some(match input.unknown_var_mode {
            UnknownVarMode::Error => UnknownVariablePolicy::Error,
            UnknownVarMode::Warn => UnknownVariablePolicy::Warn,
            UnknownVarMode::Ignore => UnknownVariablePolicy::Ignore,
        }),
        ..sc_composer::ComposePolicy::default()
    }
}

pub(super) fn load_cli_and_file_vars(
    args: &InputArgs,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::default();
    load_cli_vars(&mut vars, &args.vars)?;
    load_var_file_vars(&mut vars, &args.var_file)?;
    Ok(vars)
}

pub(super) fn load_prefixed_env_vars(
    args: &InputArgs,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::default();
    if let Some(prefix) = &args.env_prefix {
        for (key, value) in std::env::vars() {
            if let Some(trimmed) = key.strip_prefix(prefix) {
                let name = if BUILTIN_VARIABLE_NAMES.contains(&trimmed) {
                    trimmed.to_owned()
                } else {
                    trimmed.to_ascii_lowercase()
                };
                vars.insert(
                    VariableName::new(name).map_err(|error| {
                        CommandError::usage_with_code(
                            anyhow!("invalid environment-derived variable `{trimmed}`: {error}"),
                            DiagnosticCode::ErrConfigParse,
                        )
                    })?,
                    serde_json::Value::String(value),
                );
            }
        }
    }
    Ok(vars)
}

fn load_pass_input_vars(
    pass: &PassInputArgs,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::new();
    load_cli_vars(&mut vars, &pass.vars)?;
    load_var_file_vars(&mut vars, &pass.var_files)?;
    Ok(vars)
}

pub(super) fn build_pass_configs(
    pass_inputs: &[PassInputArgs],
) -> Result<Vec<PassConfig>, CommandError> {
    pass_inputs
        .iter()
        .map(|pass| {
            Ok(PassConfig {
                pass_number: if pass.pass_number == 0 {
                    sc_composer::types::default_pass_number()
                } else {
                    pass.pass_number
                },
                defaults: load_pass_input_vars(pass)?,
                ..PassConfig::default()
            })
        })
        .collect()
}

pub(super) fn collect_pass_union(
    pass_inputs: &[PassInputArgs],
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let mut vars = BTreeMap::new();
    for pass in pass_inputs {
        vars.extend(load_pass_input_vars(pass)?);
    }
    Ok(vars)
}

fn load_var_source(path: &str) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    if path == "-" {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| {
                CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
            })?;
        parse_var_file_contents(&input)
    } else {
        load_var_file(std::path::Path::new(path))
    }
}

fn invalid_var_name_error(
    key: &str,
    error: &sc_composer::InvalidVariableNameError,
) -> CommandError {
    CommandError::usage_with_code_and_hints(
        anyhow!("invalid `--var` name `{key}`: {error}"),
        DiagnosticCode::ErrConfigParse,
        vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "use variable names containing only ASCII letters, digits, ., _, or -".to_owned(),
        })],
    )
}

fn load_cli_vars(
    vars: &mut BTreeMap<VariableName, InputValue>,
    entries: &[(String, String)],
) -> Result<(), CommandError> {
    for (key, value) in entries {
        vars.insert(
            VariableName::new(key.clone()).map_err(|error| invalid_var_name_error(key, &error))?,
            serde_json::Value::String(value.clone()),
        );
    }
    Ok(())
}

fn load_var_file_vars(
    vars: &mut BTreeMap<VariableName, InputValue>,
    paths: &[String],
) -> Result<(), CommandError> {
    for path in paths {
        vars.extend(load_var_source(path)?);
    }
    Ok(())
}
