//! Frontmatter section normalization, diagnostics, and pass validation.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::error::{ComposeError, ConfigError, RecoveryHint, RecoveryHintKind, ValidationError};
use crate::types::{
    InputValue, MetadataValue, VariableName, default_pass_number, input_value_from_yaml,
};

use super::model::{Frontmatter, RawFrontmatter};

pub(super) fn normalize_frontmatter(raw: RawFrontmatter) -> Result<Frontmatter, ComposeError> {
    let parse_default_entry = |section_name: &str,
                               name: String,
                               value: serde_yaml::Value|
     -> Result<(VariableName, InputValue), ComposeError> {
        let variable = VariableName::new(name).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!("invalid frontmatter {section_name} variable name: {error}"),
            )
            .with_recovery_hint(RecoveryHint::new(
                RecoveryHintKind::ReviewConfiguration {
                    key: section_name.to_owned(),
                },
            ))
        })?;
        let input_value = input_value_from_yaml(value).map_err(|error| {
            ValidationError::invalid_input_value(error.code(), error.to_string())
        })?;
        Ok((variable, input_value))
    };

    let RawFrontmatter {
        pass,
        required_variables: raw_required_variables,
        variables: raw_variables,
        defaults: raw_defaults,
        input_defaults: raw_input_defaults,
        metadata: raw_metadata,
    } = raw;

    let mut required_variables =
        Vec::with_capacity(raw_required_variables.len() + raw_variables.len());
    let mut seen = BTreeSet::new();
    let mut add_required_variable = |variable: String, section_name: &str| {
        let variable = VariableName::new(variable).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!("invalid frontmatter variable name: {error}"),
            )
            .with_recovery_hint(RecoveryHint::new(
                RecoveryHintKind::ReviewConfiguration {
                    key: section_name.to_owned(),
                },
            ))
        })?;
        if !seen.insert(variable.clone()) {
            return Err(ValidationError::duplicate_variable(&variable).into());
        }
        required_variables.push(variable);
        Ok::<(), ComposeError>(())
    };

    for variable in raw_required_variables {
        add_required_variable(variable, "required_variables")?;
    }
    for (variable, declaration) in raw_variables {
        if declaration.required {
            add_required_variable(variable, "variables")?;
        }
    }

    let mut diagnostics = Vec::new();
    let mut defaults = BTreeMap::new();
    for (name, value) in raw_defaults {
        let (variable, input_value) = parse_default_entry("default", name, value)?;
        defaults.insert(variable, input_value);
    }

    if !defaults.is_empty() && !raw_input_defaults.is_empty() {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticCode::WarnValConflictingDefaultSections,
            "frontmatter contains both `defaults` and `input_defaults`; `input_defaults` overrides overlapping keys",
        ));
    }

    for (name, value) in raw_input_defaults {
        let (variable, input_value) = parse_default_entry("input_defaults", name, value)?;
        defaults.insert(variable, input_value);
    }

    let metadata = raw_metadata
        .into_iter()
        .map(|(key, value)| (key, MetadataValue::new(value)))
        .collect();

    Ok(Frontmatter {
        pass_number: match pass {
            Some(0) | None => default_pass_number(),
            Some(pass_number) => pass_number,
        },
        has_explicit_pass_number: pass.is_some(),
        required_variables,
        defaults,
        metadata,
        diagnostics,
    })
}

pub(super) fn validate_explicit_pass_numbers(passes: &[Frontmatter]) -> Result<(), ComposeError> {
    let mut seen_explicit_pass_numbers = BTreeSet::new();
    for frontmatter in passes {
        if frontmatter.has_explicit_pass_number()
            && !seen_explicit_pass_numbers.insert(frontmatter.pass_number())
        {
            return Err(ValidationError::invalid_input_value(
                DiagnosticCode::ErrConfigParse,
                format!(
                    "duplicate explicit pass number in stacked frontmatter: {}",
                    frontmatter.pass_number()
                ),
            )
            .into());
        }
    }
    Ok(())
}
