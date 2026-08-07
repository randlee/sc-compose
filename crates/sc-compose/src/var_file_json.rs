use serde::Deserializer;
use serde::de::{DeserializeSeed, Error as DeError, MapAccess, SeqAccess, Visitor};

use super::VarFileDecodeError;
use super::decode::{DecodedVarEntry, DecodedVarKey, DecodedVarObject, DecodedVarValue};

pub(super) fn find_out_of_range_json_integer(contents: &str) -> Option<String> {
    let bytes = contents.as_bytes();
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            index += 1;
            continue;
        }
        if byte != b'-' && !byte.is_ascii_digit() {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < bytes.len()
            && !matches!(
                bytes[index],
                b' ' | b'\t' | b'\r' | b'\n' | b',' | b']' | b'}'
            )
        {
            index += 1;
        }
        let token = &contents[start..index];
        if token.contains('.') || token.contains('e') || token.contains('E') {
            continue;
        }

        if token.starts_with('-') {
            match token.parse::<i128>() {
                Ok(value) if value >= i128::from(i64::MIN) => {}
                _ => return Some(token.to_owned()),
            }
        } else {
            match token.parse::<u128>() {
                Ok(value) if value <= u128::from(u64::MAX) => {}
                _ => return Some(token.to_owned()),
            }
        }
    }
    None
}

pub(super) fn decode_json_object(
    value: serde_json::Value,
) -> Result<DecodedVarObject, VarFileDecodeError> {
    let serde_json::Value::Object(object) = value else {
        return Err(VarFileDecodeError::NotAnObject);
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

pub(super) fn parse_json_value_rejecting_duplicate_keys(
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
