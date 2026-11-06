//! Decode YAML mappings while retaining duplicate key identity.
use std::fmt;

use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_yaml::{Mapping, Value};

pub(super) struct CheckedValue(Value);

impl<'de> Deserialize<'de> for CheckedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(CheckedValue(Value::Null))
    }
}

impl<'de> Visitor<'de> for CheckedValue {
    type Value = CheckedValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML value")
    }
    fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(CheckedValue(Value::Bool(v)))
    }
    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(CheckedValue(Value::Number(v.into())))
    }
    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(CheckedValue(Value::Number(v.into())))
    }
    fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(CheckedValue(Value::Number(v.into())))
    }
    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(CheckedValue(Value::String(v.into())))
    }
    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(CheckedValue(Value::String(v)))
    }
    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(CheckedValue(Value::Null))
    }
    fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
        Ok(CheckedValue(Value::Null))
    }
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        CheckedValue::deserialize(deserializer)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(CheckedValue(value)) = seq.next_element()? {
            values.push(value);
        }
        Ok(CheckedValue(Value::Sequence(values)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = Mapping::new();
        while let Some(CheckedValue(key)) = map.next_key()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom(format!(
                    "duplicate entry with key {key:?}"
                )));
            }
            let CheckedValue(value) = map.next_value()?;
            values.insert(key, value);
        }
        Ok(CheckedValue(Value::Mapping(values)))
    }
    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
        let (tag, variant) = data.variant::<String>()?;
        let CheckedValue(value) = variant.newtype_variant()?;
        Ok(CheckedValue(Value::Tagged(Box::new(
            serde_yaml::value::TaggedValue {
                tag: serde_yaml::value::Tag::new(tag),
                value,
            },
        ))))
    }
}
