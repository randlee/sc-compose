//! Typed YAML frontmatter parsing and normalization.

mod model;
mod normalizer;
mod parser;

pub use model::{Frontmatter, ParsedTemplate};
pub use parser::parse_template_document;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::parse_template_document;

    #[test]
    fn parses_document_without_frontmatter() {
        let parsed = parse_template_document("hello world").unwrap();

        assert!(parsed.passes().is_empty());
        assert_eq!(parsed.body(), "hello world");
        assert!(parsed.frontmatter().is_none());
    }

    #[test]
    fn parses_single_header_with_explicit_pass() {
        let parsed = parse_template_document("---\npass: 2\n---\nbody").unwrap();

        assert_eq!(parsed.passes().len(), 1);
        assert_eq!(parsed.passes()[0].pass_number(), 2);
        assert_eq!(parsed.frontmatter().unwrap().pass_number(), 2);
        assert_eq!(parsed.body(), "body");
    }

    #[test]
    fn strips_utf8_bom_before_parsing_frontmatter() {
        let parsed = parse_template_document(
            "\u{feff}---\nrequired_variables:\n  - name\n---\nHello {{name}}\n",
        )
        .unwrap();

        assert_eq!(parsed.passes().len(), 1);
        assert_eq!(parsed.body(), "Hello {{name}}\n");
    }

    #[test]
    fn recognizes_declared_required_variables_map() {
        let parsed = parse_template_document(
            "---\nvariables:\n  name:\n    required: true\n---\nHello {{name}}\n",
        )
        .unwrap();

        assert_eq!(parsed.frontmatter().unwrap().required_variables().len(), 1);
        assert_eq!(
            parsed.frontmatter().unwrap().required_variables()[0].as_str(),
            "name"
        );
    }

    #[test]
    fn parses_stacked_empty_headers_with_default_pass_numbers() {
        let parsed = parse_template_document("---\n---\n---\n---\nbody").unwrap();

        assert_eq!(parsed.passes().len(), 2);
        assert_eq!(parsed.passes()[0].pass_number(), 1);
        assert_eq!(parsed.passes()[1].pass_number(), 1);
        assert_eq!(parsed.body(), "body");
    }

    #[test]
    fn supports_dot_delimiter_in_stacked_headers() {
        let parsed = parse_template_document("---\n...\n---\n...\nbody").unwrap();

        assert_eq!(parsed.passes().len(), 2);
        assert_eq!(parsed.body(), "body");
    }

    #[test]
    fn preserves_later_body_delimiters_after_leading_header_stack() {
        let parsed =
            parse_template_document("---\ndefaults: {name: world}\n---\nhello\n---\nrule").unwrap();

        assert_eq!(parsed.passes().len(), 1);
        assert_eq!(parsed.body(), "hello\n---\nrule");
    }

    #[test]
    fn preserves_jinja_body_after_single_frontmatter_block() {
        let parsed = parse_template_document(
            "---\nrequired_variables:\n  - id\n---\n{% if id %}{{ id }}{% endif %}\n",
        )
        .unwrap();

        assert_eq!(parsed.passes().len(), 1);
        assert_eq!(parsed.body(), "{% if id %}{{ id }}{% endif %}\n");
    }

    #[test]
    fn preserves_adjacent_plain_yaml_header_stack() {
        let parsed = parse_template_document(
            "---\nname: config\n---\n---\nmetadata:\n  id: rendered\n---\nbody\n",
        )
        .unwrap();

        assert_eq!(parsed.passes().len(), 2);
        assert_eq!(parsed.body(), "body\n");
    }

    #[test]
    fn treats_adjacent_jinja_document_header_as_body() {
        let parsed = parse_template_document(
            "---\nname: config\n---\n---\nid: {{ id }}\n{% if worktree %}worktree: {{ worktree }}\n{% endif %}target: x\n---\nbody\n",
        )
        .unwrap();

        assert_eq!(parsed.passes().len(), 1);
        assert_eq!(
            parsed.body(),
            "---\nid: {{ id }}\n{% if worktree %}worktree: {{ worktree }}\n{% endif %}target: x\n---\nbody\n"
        );
    }

    #[test]
    fn preserves_recognized_stacked_frontmatter_headers() {
        let parsed = parse_template_document(
            "---\npass: 2\ndefaults:\n  outer: value\n---\n---\npass: 1\ndefaults:\n  inner: value\n---\nbody\n",
        )
        .unwrap();

        assert_eq!(parsed.passes().len(), 2);
        assert_eq!(parsed.body(), "body\n");
    }

    #[test]
    fn treats_adjacent_unrecognized_yaml_block_as_body() {
        let parsed = parse_template_document("---\n{}\n---\n---\na: b\n---\nBODY\n").unwrap();

        assert_eq!(parsed.passes().len(), 1);
        assert_eq!(parsed.body(), "---\na: b\n---\nBODY\n");
    }

    #[test]
    fn malformed_yaml_fails_closed() {
        let error = parse_template_document("---\ndefaults: [\n---\nbody").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("failed to parse YAML frontmatter")
        );
    }

    #[test]
    fn duplicate_explicit_pass_numbers_fail_closed() {
        let error =
            parse_template_document("---\npass: 2\n---\n---\npass: 2\n---\nbody").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("duplicate explicit pass number in stacked frontmatter")
        );
    }

    #[test]
    fn from_parts_validated_allows_omitted_default_pass_duplicates() {
        let parsed = parse_template_document("---\n---\n---\n---\nbody").unwrap();

        let reparsed = super::ParsedTemplate::from_parts_validated(
            parsed.passes().to_vec(),
            "body".to_owned(),
        )
        .unwrap();

        assert_eq!(reparsed.passes().len(), 2);
        assert_eq!(reparsed.passes()[0].pass_number(), 1);
        assert_eq!(reparsed.passes()[1].pass_number(), 1);
    }

    #[test]
    fn from_parts_validated_rejects_duplicate_explicit_pass_numbers() {
        let explicit = super::Frontmatter {
            pass_number: 2,
            has_explicit_pass_number: true,
            required_variables: Vec::new(),
            defaults: BTreeMap::new(),
            metadata: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        let error = super::ParsedTemplate::from_parts_validated(
            vec![explicit.clone(), explicit],
            "body".to_owned(),
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("duplicate explicit pass number in stacked frontmatter")
        );
    }

    #[test]
    fn default_section_precedence_prefers_input_defaults() {
        let parsed = parse_template_document(
            "---\ndefaults:\n  name: from-default\ninput_defaults:\n  name: from-input-default\n---\nbody",
        )
        .unwrap();

        assert_eq!(
            parsed.frontmatter().unwrap().defaults()[&crate::VariableName::new("name").unwrap()],
            serde_json::json!("from-input-default")
        );
        assert_eq!(parsed.frontmatter().unwrap().diagnostics().len(), 1);
    }
}
