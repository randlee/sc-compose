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
    if let Ok(value) = parse_json_value_rejecting_duplicate_keys(contents) {
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
