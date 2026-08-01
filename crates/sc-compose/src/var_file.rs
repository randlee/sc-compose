use std::collections::BTreeMap;
use std::path::Path;

use anyhow::anyhow;
use sc_composer::{
    DiagnosticCode, InputValue, VariableName, input_value_from_yaml, validate_input_value,
};
use serde::Deserializer;
use serde::de::{DeserializeSeed, Error as DeError, MapAccess, SeqAccess, Visitor};

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
    let object = decode_var_file(contents).map_err(VarFileDecodeError::into_command_error)?;
    validate_var_object(object)
}

#[derive(Debug)]
enum VarFileDecodeError {
    InvalidFormat(anyhow::Error),
    NotAnObject { format: VarFileFormat },
}

#[derive(Debug)]
enum VarFileFormat {
    Json,
    Yaml,
}

impl VarFileDecodeError {
    fn into_command_error(self) -> CommandError {
        match self {
            Self::InvalidFormat(error) => CommandError::usage_with_code(
                error.context("var-file must be valid JSON or YAML"),
                DiagnosticCode::ErrConfigParse,
            ),
            Self::NotAnObject {
                format: VarFileFormat::Json,
            } => CommandError::usage_with_code(
                anyhow!("var-file must be a JSON object"),
                DiagnosticCode::ErrConfigVarfile,
            ),
            Self::NotAnObject {
                format: VarFileFormat::Yaml,
            } => CommandError::usage_with_code(
                anyhow!("var-file must be a JSON or YAML object"),
                DiagnosticCode::ErrConfigVarfile,
            ),
        }
    }
}

enum DecodedVarKey {
    String(String),
    Yaml(serde_yaml::Value),
}

enum DecodedVarValue {
    Json(serde_json::Value),
    Yaml(serde_yaml::Value),
}

struct DecodedVarEntry {
    key: DecodedVarKey,
    value: DecodedVarValue,
}

struct DecodedVarObject {
    entries: Vec<DecodedVarEntry>,
}

fn decode_var_file(contents: &str) -> Result<DecodedVarObject, VarFileDecodeError> {
    if let Ok(value) = parse_json_value_rejecting_duplicate_keys(contents) {
        return decode_json_object(value);
    }

    let value = serde_yaml::from_str::<serde_yaml::Value>(contents)
        .map_err(|error| VarFileDecodeError::InvalidFormat(anyhow!(error)))?;
    decode_yaml_object(value)
}

fn decode_json_object(value: serde_json::Value) -> Result<DecodedVarObject, VarFileDecodeError> {
    let serde_json::Value::Object(object) = value else {
        return Err(VarFileDecodeError::NotAnObject {
            format: VarFileFormat::Json,
        });
    };
    Ok(DecodedVarObject {
        entries: object
            .into_iter()
            .map(|(key, value)| DecodedVarEntry {
                key: DecodedVarKey::String(key),
                value: DecodedVarValue::Json(value),
            })
            .collect(),
    })
}

fn decode_yaml_object(value: serde_yaml::Value) -> Result<DecodedVarObject, VarFileDecodeError> {
    let serde_yaml::Value::Mapping(object) = value else {
        return Err(VarFileDecodeError::NotAnObject {
            format: VarFileFormat::Yaml,
        });
    };
    Ok(DecodedVarObject {
        entries: object
            .into_iter()
            .map(|(key, value)| DecodedVarEntry {
                key: DecodedVarKey::Yaml(key),
                value: DecodedVarValue::Yaml(value),
            })
            .collect(),
    })
}

fn validate_var_object(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_json_and_yaml_objects_share_validated_conversion() {
        let json = decode_var_file(r#"{"name":"world","items":[{"id":1}],"enabled":true}"#)
            .expect("JSON object");
        let yaml = decode_var_file("name: world\nitems:\n  - id: 1\nenabled: true\n")
            .expect("YAML object");

        assert_eq!(
            validate_var_object(json).unwrap(),
            validate_var_object(yaml).unwrap()
        );
    }

    #[test]
    fn decode_and_validation_preserve_source_specific_boundaries() {
        assert_eq!(
            parse_var_file_contents("[1, 2, 3]")
                .unwrap_err()
                .diagnostic_code,
            Some(DiagnosticCode::ErrConfigVarfile)
        );
        assert_eq!(
            parse_var_file_contents("- one\n- two\n")
                .unwrap_err()
                .diagnostic_code,
            Some(DiagnosticCode::ErrConfigVarfile)
        );
        assert_eq!(
            parse_var_file_contents("value:\n  1: invalid-key\n")
                .unwrap_err()
                .diagnostic_code,
            Some(DiagnosticCode::ErrValObjectShape)
        );
        assert_eq!(
            parse_var_file_contents("{\n").unwrap_err().diagnostic_code,
            Some(DiagnosticCode::ErrConfigParse)
        );
    }

    #[test]
    fn duplicate_keys_remain_rejected_at_decode_boundary() {
        for contents in [
            r#"{"outer":{"name":"one","name":"two"}}"#,
            "outer:\n  name: one\n  name: two\n",
        ] {
            assert_eq!(
                parse_var_file_contents(contents)
                    .unwrap_err()
                    .diagnostic_code,
                Some(DiagnosticCode::ErrConfigParse),
                "contents: {contents}"
            );
        }
    }

    #[test]
    fn invalid_variable_name_remains_a_var_file_error() {
        let error = parse_var_file_contents("bad key: value").unwrap_err();

        assert_eq!(
            error.diagnostic_code,
            Some(DiagnosticCode::ErrConfigVarfile)
        );
        assert!(error.error.to_string().contains("invalid var-file key"));
    }
}

fn parse_json_value_rejecting_duplicate_keys(
    contents: &str,
) -> Result<serde_json::Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_str(contents);
    let value = deserializer.deserialize_any(DuplicateAwareValueVisitor)?;
    deserializer.end()?;
    Ok(value)
}

#[derive(Clone, Copy)]
struct DuplicateAwareValueVisitor;

impl<'de> DeserializeSeed<'de> for DuplicateAwareValueVisitor {
    type Value = serde_json::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for DuplicateAwareValueVisitor {
    type Value = serde_json::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Number(v.into()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Number(v.into()))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        serde_json::Number::from_f64(v)
            .map(serde_json::Value::Number)
            .ok_or_else(|| E::custom("JSON number is not finite"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(v.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(v))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(Self)? {
            values.push(value);
        }
        Ok(serde_json::Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(A::Error::custom(format!(
                    "duplicate entry with key \"{key}\""
                )));
            }
            object.insert(key, map.next_value_seed(Self)?);
        }
        Ok(serde_json::Value::Object(object))
    }
}
