//! Differential snapshots generated with `serde_yaml` 0.9.34 before migration.
//!
//! The corpus contains tracked YAML documents and frontmatter plus string
//! literals extracted from frontmatter, var-file, and YAML extraction tests.
//! Both parsed values and complete error messages are compared.

use serde_json::{Value, json};

#[test]
fn yaml_fixture_values_and_errors_match_previous_parser() {
    let corpus = include_str!("fixtures/yaml-compat/serde-yaml-0.9.jsonl");
    for line in corpus.lines() {
        let case: Value = serde_json::from_str(line).expect("valid snapshot record");
        let input = case["input"].as_str().expect("snapshot input");
        let actual = match serde_yaml::from_str::<serde_yaml::Value>(input) {
            Ok(value) => match serde_json::to_value(value) {
                Ok(value) => json!({"value": value}),
                Err(error) => json!({"conversion_error": error.to_string()}),
            },
            Err(error) => json!({"error": error.to_string()}),
        };
        assert_eq!(actual, case["expected"], "{}", case["path"]);
    }
}
