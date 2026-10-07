//! Typed request preflight; protocol classification never parses serializer prose.

use std::collections::BTreeMap;

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::{
    BeadComposeError, BeadComposeRequest, BeadId, BeadOperation, BeadRelation, GraphRef,
    PourAuthorization, StepId,
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
    operation: Value,
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

pub(crate) fn parse_request(input: &str) -> Result<BeadComposeRequest, BeadComposeError> {
    // Finish parsing the JSON before classifying semantic errors. The captured
    // duplicate is an exact decoded key, independent of serde's display format.
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
    // Check the complete serde contract without allowing identifier grammar
    // errors to mask operation, field-type, or required-field diagnostics.
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
    let shape = deserialize_request(&shape.to_string())?;
    let identifiers = (|| {
        if let Some(parent) = preflight.parent.as_str() {
            BeadId::new(parent)?;
        }
        if matches!(
            preflight.operation.as_str(),
            Some("attach" | "preview_attach")
        ) && let Some(reference) = preflight.reference.as_str()
        {
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
    deserialize_request(input)
}

fn deserialize_request(input: &str) -> Result<BeadComposeRequest, BeadComposeError> {
    match serde_json::from_str(input) {
        Ok(request) => Ok(request),
        Err(error) => {
            let mut value: Value =
                serde_json::from_str(input).map_err(|_reparse| request_error(&error))?;
            if !matches!(
                value.get("operation").and_then(Value::as_str),
                Some("attach" | "preview_attach")
            ) {
                let legacy_name = value
                    .get("formula_name")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                value
                    .as_object_mut()
                    .expect("JSON object")
                    .remove("formula_name");
                // Report the original typed parse error, not the retry's.
                let mut request: BeadComposeRequest =
                    serde_json::from_value(value).map_err(|_retry| request_error(&error))?;
                if let Some(name) = legacy_name.filter(|name| !name.is_empty()) {
                    request.formula_name = Some(crate::FormulaName::legacy(name));
                }
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
