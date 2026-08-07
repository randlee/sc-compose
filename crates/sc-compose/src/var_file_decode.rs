use anyhow::anyhow;

use super::{VarFileDecodeError, json, yaml};

pub(super) enum DecodedVarKey {
    String(String),
    Yaml(serde_yaml::Value),
}

pub(super) enum DecodedVarValue {
    Json(serde_json::Value),
    Yaml(serde_yaml::Value),
}

pub(super) struct DecodedVarEntry {
    pub(super) key: DecodedVarKey,
    pub(super) value: DecodedVarValue,
}

pub(super) struct DecodedVarObject {
    pub(super) entries: Vec<DecodedVarEntry>,
}

pub(super) fn decode_var_file(contents: &str) -> Result<DecodedVarObject, VarFileDecodeError> {
    if let Ok(value) = json::parse_json_value_rejecting_duplicate_keys(contents) {
        if let Some(value) = json::find_out_of_range_json_integer(contents) {
            return Err(VarFileDecodeError::OutOfRangeInteger { value });
        }
        return json::decode_json_object(value);
    }

    let value = serde_yaml::from_str::<serde_yaml::Value>(contents)
        .map_err(|error| VarFileDecodeError::InvalidFormat(anyhow!(error)))?;
    if let Some((line, column)) = yaml::find_yaml_merge_key(contents) {
        return Err(VarFileDecodeError::UnsupportedYamlMergeKey { line, column });
    }
    yaml::decode_yaml_object(value)
}
