//! Typed request preflight; protocol classification never parses serializer prose.

use std::collections::BTreeMap;

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::{BeadComposeError, BeadComposeRequest, BeadRelation, PourAuthorization};

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
    validate_endpoint_prefixes(&preflight.relations)?;
    serde_json::from_str(input).map_err(|error| request_error(&error))
}

fn validate_endpoint_prefixes(relations: &Value) -> Result<(), BeadComposeError> {
    if let Some(relations) = relations.as_array() {
        for relation in relations {
            for field in ["from", "to"] {
                if let Some(value) = relation.get(field).and_then(Value::as_str)
                    && !value.starts_with("step:")
                    && !value.starts_with("bead:")
                {
                    return Err(BeadComposeError::RelationEndpointInvalid {
                        value: value.to_owned(),
                    });
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
