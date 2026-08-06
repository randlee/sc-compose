use std::collections::BTreeMap;
use std::path::Path;

use anyhow::anyhow;
use sc_composer::{
    DiagnosticCode, InputValue, RecoveryHint, RecoveryHintKind, VariableName,
    input_value_from_yaml, validate_input_value,
};
use serde::Deserializer;
use serde::de::{DeserializeSeed, Error as DeError, MapAccess, SeqAccess, Visitor};

use crate::CommandError;

pub(crate) fn load_var_file(
    path: &Path,
) -> Result<BTreeMap<VariableName, InputValue>, CommandError> {
    let contents = std::fs::read_to_string(path).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!(error).context(format!("failed to read var-file {}", path.display())),
            DiagnosticCode::ErrConfigRead,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: path.to_owned(),
            })],
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
    OutOfRangeInteger { value: String },
    UnsupportedYamlMergeKey { line: usize, column: usize },
    NotAnObject,
}

impl VarFileDecodeError {
    fn into_command_error(self) -> CommandError {
        match self {
            Self::InvalidFormat(error) => CommandError::usage_with_code(
                error.context("var-file must be valid JSON or YAML"),
                DiagnosticCode::ErrConfigParse,
            ),
            Self::OutOfRangeInteger { value } => CommandError::usage_with_code(
                anyhow!("JSON integer {value} is outside the representable range"),
                DiagnosticCode::ErrConfigVarfile,
            ),
            Self::UnsupportedYamlMergeKey { line, column } => CommandError::usage_with_code(
                anyhow!(
                    "unsupported YAML merge key `<<` at line {line}, column {column}; expand the mapping explicitly to preserve inherited fields"
                ),
                DiagnosticCode::ErrConfigVarfile,
            ),
            Self::NotAnObject => CommandError::usage_with_code(
                anyhow!("var-file top-level value must be an object (JSON) or mapping (YAML)"),
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
        if let Some(value) = find_out_of_range_json_integer(contents) {
            return Err(VarFileDecodeError::OutOfRangeInteger { value });
        }
        return decode_json_object(value);
    }

    let value = serde_yaml::from_str::<serde_yaml::Value>(contents)
        .map_err(|error| VarFileDecodeError::InvalidFormat(anyhow!(error)))?;
    if let Some((line, column)) = find_yaml_merge_key(contents) {
        return Err(VarFileDecodeError::UnsupportedYamlMergeKey { line, column });
    }
    decode_yaml_object(value)
}

/// Find integer literals that `serde_json`'s default number representation cannot
/// preserve without narrowing them to a lossy floating-point value.
fn find_out_of_range_json_integer(contents: &str) -> Option<String> {
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

/// Find YAML merge-key syntax while the source still retains quoting and
/// comment boundaries. Inspecting only `serde_yaml::Value` would conflate a
/// quoted ordinary key named `<<` with the YAML merge-key construct.
fn find_yaml_merge_key(contents: &str) -> Option<(usize, usize)> {
    let mut block_scalar_indent = None;

    for (line_index, line) in contents.lines().enumerate() {
        let indentation = line.bytes().take_while(|byte| *byte == b' ').count();
        let has_content = line[indentation..]
            .chars()
            .next()
            .is_some_and(|character| character != '#');

        if let Some(base_indent) = block_scalar_indent {
            if has_content && indentation > base_indent {
                continue;
            }
            block_scalar_indent = None;
        }

        if let Some(byte_index) = scan_yaml_line_for_merge_key(line) {
            let column = line[..byte_index].chars().count() + 1;
            return Some((line_index + 1, column));
        }

        if has_yaml_block_scalar_indicator(line) {
            block_scalar_indent = Some(indentation);
        }
    }

    None
}

fn scan_yaml_line_for_merge_key(line: &str) -> Option<usize> {
    for (byte_index, _) in unquoted_uncommented(line) {
        if !line[byte_index..].starts_with("<<") {
            continue;
        }
        let suffix = &line[byte_index + 2..];
        if suffix
            .chars()
            .find(|character| !character.is_ascii_whitespace())
            .is_none_or(|character| character != ':')
        {
            continue;
        }

        let prefix = line[..byte_index].trim_end();
        if prefix.is_empty()
            || prefix == "-"
            || prefix.ends_with('{')
            || prefix.ends_with(',')
            || prefix.ends_with('?')
        {
            return Some(byte_index);
        }
    }

    None
}

fn has_yaml_block_scalar_indicator(line: &str) -> bool {
    let outside_quote: String = unquoted_uncommented(line)
        .into_iter()
        .map(|(_, character)| character)
        .collect();

    outside_quote.split_whitespace().any(|token| {
        matches!(
            token.trim_end_matches(','),
            "|" | ">" | "|-" | "|+" | ">-" | ">+"
        )
    })
}

/// Return source characters outside YAML quotes and before an unquoted
/// comment, preserving each character's original byte offset.
fn unquoted_uncommented(line: &str) -> Vec<(usize, char)> {
    let mut outside_quote = Vec::new();
    let mut quote = None;
    let mut escaped = false;

    for (byte_index, character) in line.char_indices() {
        match quote {
            Some('"') => {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    quote = None;
                }
            }
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                }
            }
            Some(_) => unreachable!("only YAML single and double quotes are tracked"),
            None if character == '#' => break,
            None if character == '"' || character == '\'' => quote = Some(character),
            None => outside_quote.push((byte_index, character)),
        }
    }

    outside_quote
}

fn decode_json_object(value: serde_json::Value) -> Result<DecodedVarObject, VarFileDecodeError> {
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

fn decode_yaml_object(value: serde_yaml::Value) -> Result<DecodedVarObject, VarFileDecodeError> {
    let serde_yaml::Value::Mapping(object) = value else {
        return Err(VarFileDecodeError::NotAnObject);
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

    fn temp_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("sc-compose-fix-252-{label}-{}", std::process::id()))
    }

    #[test]
    fn missing_var_file_reports_config_read() {
        let path = temp_path("missing");
        let _ = std::fs::remove_file(&path);

        let error = load_var_file(&path).unwrap_err();

        assert_eq!(error.diagnostic_code, Some(DiagnosticCode::ErrConfigRead));
    }

    #[test]
    fn directory_var_file_reports_config_read_with_inspect_hint() {
        let path = temp_path("directory");
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(&path).unwrap();

        let error = load_var_file(&path).unwrap_err();
        std::fs::remove_dir(&path).unwrap();

        assert_eq!(error.diagnostic_code, Some(DiagnosticCode::ErrConfigRead));
        assert_eq!(
            error.recovery_hints,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath { path })]
        );
    }

    #[test]
    fn out_of_range_json_integers_fail_closed() {
        for contents in [
            r#"{"n": -9223372036854775809}"#,
            r#"{"n": 18446744073709551616}"#,
        ] {
            let error = parse_var_file_contents(contents).unwrap_err();
            assert_eq!(
                error.diagnostic_code,
                Some(DiagnosticCode::ErrConfigVarfile),
                "contents: {contents}"
            );
        }
    }

    #[test]
    fn out_of_range_json_integer_scanner_ignores_quoted_digit_runs() {
        assert_eq!(
            find_out_of_range_json_integer(
                r#"{"text":"-9223372036854775809 and 18446744073709551616"}"#
            ),
            None
        );
        assert_eq!(
            find_out_of_range_json_integer(r#"{"n":18446744073709551616}"#),
            Some("18446744073709551616".to_owned())
        );
    }

    #[test]
    fn json_integer_scanner_enforces_exact_boundaries() {
        let cases = [
            (
                r#"{"n":-9223372036854775809}"#,
                Some("-9223372036854775809"),
            ),
            (r#"{"n":-9223372036854775808}"#, None),
            (r#"{"n":-42}"#, None),
            (r#"{"n":0}"#, None),
            (r#"{"n":9223372036854775807}"#, None),
            (r#"{"n":9223372036854775808}"#, None),
            (r#"{"n":18446744073709551615}"#, None),
            (
                r#"{"n":18446744073709551616}"#,
                Some("18446744073709551616"),
            ),
        ];

        for (contents, expected) in cases {
            assert_eq!(
                find_out_of_range_json_integer(contents).as_deref(),
                expected,
                "contents: {contents}"
            );
        }
    }

    #[test]
    fn in_range_json_integer_boundaries_remain_exact() {
        let cases = [
            (
                r#"{"n": -9223372036854775808}"#,
                serde_json::json!(i64::MIN),
            ),
            (r#"{"n": -42}"#, serde_json::json!(-42)),
            (r#"{"n": 0}"#, serde_json::json!(0)),
            (r#"{"n": 9223372036854775807}"#, serde_json::json!(i64::MAX)),
            (
                r#"{"n": 18446744073709551615}"#,
                serde_json::json!(u64::MAX),
            ),
        ];

        for (contents, expected) in cases {
            let vars = parse_var_file_contents(contents).unwrap();
            assert_eq!(vars[&VariableName::new("n").unwrap()], expected);
        }
    }

    #[test]
    fn yaml_out_of_range_integer_still_fails_to_parse() {
        for contents in ["n: -9223372036854775809\n", "n: 18446744073709551616\n"] {
            let error = parse_var_file_contents(contents).unwrap_err();
            assert_eq!(
                error.diagnostic_code,
                Some(DiagnosticCode::ErrConfigParse),
                "contents: {contents}"
            );
        }
    }

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
    fn top_level_non_object_messages_are_format_neutral() {
        let json = parse_var_file_contents("42").unwrap_err();
        let yaml = parse_var_file_contents("hello").unwrap_err();

        assert_eq!(json.error.to_string(), yaml.error.to_string());
    }

    #[test]
    fn all_top_level_non_object_shapes_share_the_same_message() {
        let messages = ["42", "[1, 2, 3]", "hello", "- one\n- two\n"]
            .into_iter()
            .map(|contents| {
                parse_var_file_contents(contents)
                    .unwrap_err()
                    .error
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert!(messages.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn valid_object_shapes_remain_supported() {
        parse_var_file_contents(r#"{"name":"world"}"#).unwrap();
        parse_var_file_contents("name: world\n").unwrap();
    }

    #[test]
    fn unrelated_varfile_messages_remain_unchanged() {
        let merge_error =
            parse_var_file_contents("defaults: &defaults\n  name: base\nitem:\n  <<: *defaults\n")
                .unwrap_err();
        assert!(
            merge_error
                .error
                .to_string()
                .contains("unsupported YAML merge key `<<`")
        );

        let invalid_error = parse_var_file_contents("{\n").unwrap_err();
        assert_eq!(
            invalid_error.error.to_string(),
            "var-file must be valid JSON or YAML"
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

    #[test]
    fn exact_issue_166_reproduction_fails_before_tagged_value_unwrapping() {
        let contents = "defaults: &defaults\n  base: /tmp\n  name: base\nitem:\n  <<: *defaults\n  name: override\n";

        let error = parse_var_file_contents(contents).unwrap_err();

        assert_eq!(
            error.diagnostic_code,
            Some(DiagnosticCode::ErrConfigVarfile)
        );
        let message = error.error.to_string();
        assert!(message.contains("unsupported YAML merge key `<<`"));
        assert!(message.contains("line 5, column 3"));
        assert!(message.contains("expand the mapping explicitly"));
    }

    #[test]
    fn nested_multiple_and_precedence_merges_are_all_rejected() {
        for contents in [
            "defaults: &defaults\n  name: inherited\nouter:\n  inner:\n    <<: *defaults\n",
            "first: &first\n  a: one\nsecond: &second\n  b: two\nmerged:\n  <<: [*first, *second]\n",
            "defaults: &defaults\n  name: inherited\nitem:\n  name: explicit\n  <<: *defaults\n",
        ] {
            let error = parse_var_file_contents(contents).unwrap_err();
            assert_eq!(
                error.diagnostic_code,
                Some(DiagnosticCode::ErrConfigVarfile),
                "contents: {contents}"
            );
            assert!(
                error
                    .error
                    .to_string()
                    .contains("unsupported YAML merge key"),
                "contents: {contents}"
            );
        }
    }

    #[test]
    fn aliases_without_merge_keys_remain_supported() {
        let vars = parse_var_file_contents(
            "defaults: &defaults\n  base: /tmp\n  name: base\nitem: *defaults\n",
        )
        .expect("plain aliases are not merge keys");

        assert_eq!(
            vars[&VariableName::new("item").unwrap()],
            serde_json::json!({"base": "/tmp", "name": "base"})
        );
    }

    #[test]
    fn quoted_merge_key_is_an_ordinary_yaml_key() {
        let vars = parse_var_file_contents("config:\n  '<<': literal\n").expect("quoted key");

        assert_eq!(
            vars[&VariableName::new("config").unwrap()],
            serde_json::json!({"<<": "literal"})
        );
    }

    #[test]
    fn doubled_single_quote_preserves_option_b_scanner_behavior() {
        let merge_line = "map: {item: 'it''s', <<: *defaults}";
        let merge_index = scan_yaml_line_for_merge_key(merge_line)
            .expect("merge key after doubled quote should remain visible");
        assert_eq!(&merge_line[merge_index..merge_index + 2], "<<");

        let block_line = "item: 'it''s' |";
        assert!(has_yaml_block_scalar_indicator(block_line));

        let quoted_merge_line = "map: {'it''s <<: *defaults'}";
        assert_eq!(scan_yaml_line_for_merge_key(quoted_merge_line), None);
        assert!(!has_yaml_block_scalar_indicator("item: 'it''s |'"));
    }

    #[test]
    fn json_merge_shaped_keys_are_unaffected() {
        let vars = parse_var_file_contents(r#"{"config":{"<<":"literal"}}"#)
            .expect("JSON keys are not YAML merge syntax");

        assert_eq!(
            vars[&VariableName::new("config").unwrap()],
            serde_json::json!({"<<": "literal"})
        );
    }

    #[test]
    fn malformed_yaml_still_reports_a_parse_error() {
        let error = parse_var_file_contents("item:\n  <<: [unterminated\n").unwrap_err();

        assert_eq!(error.diagnostic_code, Some(DiagnosticCode::ErrConfigParse));
    }

    #[test]
    fn cyclic_and_deep_merge_inputs_never_succeed() {
        let cyclic = "defaults: &defaults\n  <<: *defaults\n";
        let error = parse_var_file_contents(cyclic).unwrap_err();
        assert!(matches!(
            error.diagnostic_code,
            Some(DiagnosticCode::ErrConfigParse | DiagnosticCode::ErrConfigVarfile)
        ));

        let mut deep = String::from("defaults: &defaults\n  leaf: value\nroot:\n");
        for depth in 0..32 {
            deep.push_str(&" ".repeat(2 + depth * 2));
            deep.push_str("level:\n");
        }
        deep.push_str(&" ".repeat(66));
        deep.push_str("<<: *defaults\n");

        let error = parse_var_file_contents(&deep).unwrap_err();
        assert_eq!(
            error.diagnostic_code,
            Some(DiagnosticCode::ErrConfigVarfile)
        );
    }

    #[test]
    fn merge_syntax_inside_a_block_scalar_is_not_a_merge_key() {
        let vars = parse_var_file_contents("text: |\n  <<: literal\n").expect("literal text");

        assert_eq!(
            vars[&VariableName::new("text").unwrap()],
            serde_json::json!("<<: literal\n")
        );
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
