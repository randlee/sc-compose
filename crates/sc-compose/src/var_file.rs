use std::collections::BTreeMap;
use std::path::Path;

use anyhow::anyhow;
use sc_composer::{DiagnosticCode, InputValue, RecoveryHint, RecoveryHintKind, VariableName};

use crate::CommandError;

#[path = "var_file_decode.rs"]
mod decode;
#[path = "var_file_json.rs"]
mod json;
#[path = "var_file_validate.rs"]
mod validate;
#[path = "var_file_yaml.rs"]
mod yaml;

use decode::decode_var_file;
use validate::validate_var_object;

#[cfg(test)]
use json::find_out_of_range_json_integer;
#[cfg(test)]
use yaml::scan_yaml_line;

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
    fn comments_and_block_scalar_text_are_not_merge_keys() {
        let contents = "comment: value # <<: *defaults\nitem: |\n  <<: *defaults\n";
        let vars = parse_var_file_contents(contents).expect("comment and block text");

        assert_eq!(
            vars[&VariableName::new("item").unwrap()],
            serde_json::json!("<<: *defaults\n")
        );
    }

    #[test]
    fn doubled_single_quote_preserves_option_b_scanner_behavior() {
        let merge_line = "map: {item: 'it''s', <<: *defaults}";
        let merge_index = scan_yaml_line(merge_line)
            .merge_key
            .expect("merge key after doubled quote should remain visible");
        assert_eq!(&merge_line[merge_index..merge_index + 2], "<<");

        let block_line = "item: 'it''s' |";
        assert!(scan_yaml_line(block_line).block_scalar);

        let quoted_merge_line = "map: {'it''s <<: *defaults'}";
        assert_eq!(scan_yaml_line(quoted_merge_line).merge_key, None);
        assert!(!scan_yaml_line("item: 'it''s |'").block_scalar);
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
