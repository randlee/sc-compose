//! Typed request preflight; protocol classification never parses serializer prose.

use std::collections::{BTreeMap, BTreeSet};

use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::{
    BeadComposeError, BeadComposeRequest, BeadDiagnostic, BeadId, BeadOperation, BeadOutcome,
    BeadRelation, BeadStage, BeadStageOutcome, BeadStageReceipt, GraphRef, PourAuthorization,
    RefusedBeadComposeReceipt, RequestParseOutcome, StepId,
};

#[derive(Default)]
pub(crate) struct BeadVariables {
    pub(crate) values: BTreeMap<String, String>,
    pub(crate) duplicate: Option<String>,
}

impl<'de> Deserialize<'de> for BeadVariables {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VariablesVisitor;
        impl<'de> Visitor<'de> for VariablesVisitor {
            type Value = BeadVariables;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON object of string Beads variables")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut parsed = BeadVariables::default();
                while let Some((key, value)) = map.next_entry::<String, String>()? {
                    if parsed.values.insert(key.clone(), value).is_some()
                        && parsed.duplicate.is_none()
                    {
                        parsed.duplicate = Some(key);
                    }
                }
                Ok(parsed)
            }
        }
        deserializer.deserialize_map(VariablesVisitor)
    }
}

#[derive(Deserialize)]
struct RequestPreflight {
    #[serde(default)]
    parent: Value,
    #[serde(default, rename = "ref")]
    reference: Value,
    #[serde(default)]
    bead_variables: BeadVariables,
    #[serde(default)]
    pour_authorization: Value,
    #[serde(default)]
    relations: Value,
}

fn request_error(error: &serde_json::Error) -> BeadComposeError {
    BeadComposeError::RequestDeserializationFailed {
        message: error.to_string(),
    }
}

fn duplicate_top_level_key(input: &str) -> Result<Option<String>, BeadComposeError> {
    struct Keys;
    impl<'de> Visitor<'de> for Keys {
        type Value = Option<String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON object")
        }
        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut seen = BTreeSet::new();
            let mut duplicate = None;
            while let Some((key, _)) = map.next_entry::<String, IgnoredAny>()? {
                if !seen.insert(key.clone()) && duplicate.is_none() {
                    duplicate = Some(key);
                }
            }
            Ok(duplicate)
        }
    }
    let mut deserializer = serde_json::Deserializer::from_str(input);
    deserializer
        .deserialize_map(Keys)
        .map_err(|error| request_error(&error))
}

pub(crate) fn parse_request(input: &str) -> Result<BeadComposeRequest, BeadComposeError> {
    parse_request_as(input, None)
}

/// Parse with `operation` (the CLI subcommand) overriding the request file's
/// own `operation` for every operation-dependent check.
pub(crate) fn parse_request_as(
    input: &str,
    operation: Option<BeadOperation>,
) -> Result<BeadComposeRequest, BeadComposeError> {
    // Finish parsing the JSON before classifying semantic errors. The captured
    // duplicate is an exact decoded key, independent of serde's display format.
    if let Some(key) = duplicate_top_level_key(input)? {
        return Err(BeadComposeError::RequestDeserializationFailed {
            message: format!("duplicate field `{key}`"),
        });
    }
    let preflight: RequestPreflight =
        serde_json::from_str(input).map_err(|error| request_error(&error))?;
    if let Some(key) = preflight.bead_variables.duplicate {
        return Err(BeadComposeError::BeadVariableKeyDuplicate { key });
    }
    if !preflight.pour_authorization.is_null() {
        let token = preflight
            .pour_authorization
            .as_str()
            .ok_or(BeadComposeError::PourAuthorizationInvalid)?;
        PourAuthorization::try_from(token)?;
    }
    // Request shape and authorization take precedence over identifier grammar.
    let shape = request_shape_as(input, operation)?;
    let attach_family = matches!(
        shape.operation,
        BeadOperation::Attach | BeadOperation::PreviewAttach
    );
    let identifiers = (|| {
        if let Some(parent) = preflight.parent.as_str() {
            BeadId::new(parent)?;
        }
        if attach_family && let Some(reference) = preflight.reference.as_str() {
            GraphRef::new(reference)?;
        }
        validate_endpoint_prefixes(&preflight.relations)
    })();
    if let Err(error) = identifiers {
        if matches!(&error, BeadComposeError::GraphIdInvalid { .. })
            && matches!(shape.operation, BeadOperation::Attach | BeadOperation::Pour)
            && shape.pour_authorization.is_none()
        {
            return Err(BeadComposeError::PourAuthorizationRequired);
        }
        return Err(error);
    }
    let mut request = deserialize_request(input, operation)?;
    if let Some(operation) = operation {
        request.operation = operation;
    }
    Ok(request)
}

fn request_shape_as(
    input: &str,
    operation: Option<BeadOperation>,
) -> Result<BeadComposeRequest, BeadComposeError> {
    let mut shape = request_shape(input, operation)?;
    if let Some(operation) = operation {
        shape.operation = operation;
    }
    Ok(shape)
}

fn request_shape(
    input: &str,
    operation: Option<BeadOperation>,
) -> Result<BeadComposeRequest, BeadComposeError> {
    let mut shape: Value = serde_json::from_str(input).map_err(|error| request_error(&error))?;
    for field in ["parent", "ref"] {
        if let Some(value) = shape.get_mut(field).filter(|value| value.is_string()) {
            *value = Value::String("valid".into());
        }
    }
    if let Some(relations) = shape.get_mut("relations").and_then(Value::as_array_mut) {
        for relation in relations {
            for field in ["from", "to"] {
                if let Some(value) = relation.get_mut(field).filter(|value| value.is_string()) {
                    *value = Value::String("bead:valid".into());
                }
            }
        }
    }
    deserialize_request(&shape.to_string(), operation)
}

pub(crate) fn parse_request_with_outcome(
    input: &str,
    operation: Option<BeadOperation>,
) -> Result<RequestParseOutcome, BeadComposeError> {
    match parse_request_as(input, operation) {
        Ok(request) => Ok(RequestParseOutcome::Ready(request)),
        Err(error) => {
            let Some(diagnostic) = BeadDiagnostic::graph_id_invalid(&error) else {
                return Err(error);
            };
            let shape = request_shape_as(input, operation)?;
            if !matches!(
                shape.operation,
                BeadOperation::Attach | BeadOperation::PreviewAttach
            ) {
                return Err(error);
            }
            let rendered_formula = refused_formula_path(&shape)?;
            let receipt = crate::execute::receipt(
                &shape,
                rendered_formula,
                vec![BeadStageReceipt {
                    stage: BeadStage::Validate,
                    argv: Vec::new(),
                    exit_status: None,
                    elapsed_ms: 0,
                    stdout_excerpt: String::new(),
                    stderr_excerpt: crate::execute::excerpt(&error.to_string()),
                    outcome: BeadStageOutcome::Failed {
                        code: error.code().into(),
                    },
                }],
                BeadOutcome::Refused {
                    code: error.code().into(),
                },
            );
            Ok(RequestParseOutcome::Refused(RefusedBeadComposeReceipt {
                receipt,
                error: diagnostic,
            }))
        }
    }
}

fn refused_formula_path(
    request: &BeadComposeRequest,
) -> Result<std::path::PathBuf, BeadComposeError> {
    let path = if request.rendered_formula.is_absolute() {
        request.rendered_formula.clone()
    } else if request.working_directory.is_absolute() {
        request.working_directory.join(&request.rendered_formula)
    } else {
        std::env::current_dir()
            .map_err(|source| BeadComposeError::RequestReadFailed {
                path: request.working_directory.clone(),
                source,
            })?
            .join(&request.working_directory)
            .join(&request.rendered_formula)
    };
    // Refusal requires no filesystem validation: normalize lexically so missing
    // templates or output directories cannot mask the native identifier error.
    let mut normalized = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

/// Deserialize a request under its effective operation: the caller's
/// `operation` (the CLI subcommand) when given, else the request file's own.
///
/// Operation-dependent contract checks (attach `parent`/`ref`, relation
/// placement) and the legacy `formula_name` tolerance all key on the effective
/// operation. A file whose own `operation` is missing or unknown is left
/// untouched so it still fails as a malformed request. When the effective
/// operation equals the file's, the original text is parsed so serde keeps its
/// line and column locations.
fn deserialize_request(
    input: &str,
    operation: Option<BeadOperation>,
) -> Result<BeadComposeRequest, BeadComposeError> {
    let overridden = operation.and_then(|operation| {
        let mut value: Value = serde_json::from_str(input).ok()?;
        let file_operation = value.get("operation").and_then(Value::as_str)?;
        serde_json::from_value::<BeadOperation>(Value::String(file_operation.to_owned())).ok()?;
        if file_operation == operation.as_str() {
            return None;
        }
        *value.get_mut("operation")? = Value::String(operation.as_str().to_owned());
        Some(value)
    });
    let first = match &overridden {
        Some(value) => serde_json::from_value(value.clone()),
        None => serde_json::from_str(input),
    };
    match first {
        Ok(request) => Ok(request),
        Err(error) => {
            let mut value = match overridden {
                Some(value) => value,
                None => serde_json::from_str(input).map_err(|_reparse| request_error(&error))?,
            };
            let formula_name = value.get("formula_name");
            if formula_name.is_some_and(|name| !name.is_string()) {
                return Err(BeadComposeError::RequestDeserializationFailed {
                    message: format!("formula_name: {error}"),
                });
            }
            let legacy_name = formula_name.and_then(Value::as_str).map(str::to_owned);
            let effective = operation.or_else(|| {
                value
                    .get("operation")
                    .and_then(|name| serde_json::from_value(name.clone()).ok())
            });
            if let (Some(name), Some(effective)) = (legacy_name, effective)
                && !matches!(
                    effective,
                    BeadOperation::Attach | BeadOperation::PreviewAttach
                )
            {
                value
                    .as_object_mut()
                    .expect("JSON object")
                    .remove("formula_name");
                // Report the original typed parse error, not the retry's.
                let mut request: BeadComposeRequest =
                    serde_json::from_value(value).map_err(|_retry| request_error(&error))?;
                request.formula_name = crate::FormulaName::for_operation(effective, name)?;
                return Ok(request);
            }
            Err(request_error(&error))
        }
    }
}

fn validate_endpoint_prefixes(relations: &Value) -> Result<(), BeadComposeError> {
    if let Some(relations) = relations.as_array() {
        for relation in relations {
            for field in ["from", "to"] {
                if let Some(value) = relation.get(field).and_then(Value::as_str) {
                    if let Some(step) = value.strip_prefix("step:") {
                        StepId::new(step)?;
                    } else if let Some(bead) = value.strip_prefix("bead:") {
                        BeadId::new(bead)?;
                    } else {
                        return Err(BeadComposeError::RelationEndpointInvalid {
                            value: value.to_owned(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn parse_relations(value: Value) -> Result<Vec<BeadRelation>, BeadComposeError> {
    validate_endpoint_prefixes(&value)?;
    serde_json::from_value(value).map_err(|error| request_error(&error))
}
