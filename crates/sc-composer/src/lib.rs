#![deny(missing_docs)]
//! Core rendering and composition primitives for the `sc-compose` workspace.
//!
//! Sprint 2 establishes the foundational public types, canonical error
//! families, diagnostics envelope, and typed frontmatter parsing surface used
//! by later sprints.

/// End-to-end composition orchestration.
pub mod composer;
/// Structured diagnostics and the stable `ERR_*` code registry.
pub mod diagnostics;
/// Canonical crate-owned error types.
pub mod error;
/// Typed frontmatter parsing and normalization.
pub mod frontmatter;
/// Frontmatter initialization helper.
pub mod frontmatter_init;
/// Recursive include expansion and confinement enforcement.
pub mod include;
/// Workspace bootstrap helper.
pub mod init_workspace;
/// Observer traits and event payloads.
pub mod observer;
/// Template renderer wrapper.
pub mod renderer;
/// Runtime-aware profile resolution and search tracing.
pub mod resolver;
/// Foundational request, result, and value-model types.
pub mod types;
/// Public validation entrypoint.
pub mod validate;
/// Variable discovery and validation semantics.
pub mod validation;

#[doc(inline)]
pub use composer::{compose, compose_with_observer};
#[doc(inline)]
pub use diagnostics::{
    DIAGNOSTIC_SCHEMA_VERSION, Diagnostic, DiagnosticCode, DiagnosticEnvelope, DiagnosticSeverity,
};
#[doc(inline)]
pub use error::{
    ComposeError, ConfigError, IncludeError, RecoveryHint, RecoveryHintKind, RenderError,
    ResolveError, ValidationError,
};
#[doc(inline)]
pub use frontmatter::{Frontmatter, ParsedTemplate, parse_template_document};
#[doc(inline)]
pub use frontmatter_init::frontmatter_init;
#[doc(inline)]
pub use include::{ExpandedTemplate, expand_includes};
#[doc(inline)]
pub use init_workspace::init_workspace;
#[doc(inline)]
pub use observer::{
    CompositionObserver, IncludeOutcomeEvent, NoopObserver, ObservationEvent, ObservationSink,
    RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
};
#[doc(inline)]
pub use renderer::{Renderer, render_template};
#[doc(inline)]
pub use resolver::{resolve_profile, resolve_profile_with_observer, resolve_template_path};
#[doc(inline)]
pub use types::{
    ComposeMode, ComposePolicy, ComposeRequest, ComposeResult, ConfiningRoot,
    FrontmatterInitResult, IncludeDepth, InitResult, InputValue, MetadataValue, ProfileKind,
    ProfileName, ResolveResult, ResolverPolicy, RuntimeKind, ScalarValue, UnknownVariablePolicy,
    ValidationReport, VariableName, VariableSource, input_value_from_yaml, validate_input_value,
};
#[doc(inline)]
pub use validate::{validate, validate_with_observer};

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use serde_json::json;

    use super::{
        ComposeError, DiagnosticCode, RenderError, ScalarValue, parse_template_document,
        render_template,
    };

    #[test]
    fn renders_inline_template() {
        let rendered = render_template("hello {{ name }}", json!({ "name": "world" })).unwrap();
        assert_eq!(rendered, "hello world");
    }

    #[test]
    fn render_error_preserves_source_and_backtrace() {
        let error = render_template("{{ broken", json!({})).unwrap_err();
        assert!(error.source().is_some());
        assert!(!format!("{}", error.backtrace()).is_empty());
    }

    #[test]
    fn frontmatter_defaults_to_empty_maps_when_omitted() {
        let parsed =
            parse_template_document("---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n")
                .unwrap();
        let frontmatter = parsed.frontmatter().unwrap();

        assert_eq!(frontmatter.required_variables().len(), 1);
        assert!(frontmatter.defaults().is_empty());
        assert!(frontmatter.metadata().is_empty());
    }

    #[test]
    fn frontmatter_rejects_nested_defaults() {
        let error = parse_template_document(
            "---\ndefaults:\n  name:\n    nested: nope\n---\nhello {{ name }}\n",
        )
        .unwrap_err();

        match error {
            ComposeError::Validation(validation) => {
                assert_eq!(validation.code(), Some(DiagnosticCode::ErrValType));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn scalar_value_rejects_object_json_values() {
        let value = serde_json::json!({ "nested": true });
        let error = ScalarValue::try_from(value).unwrap_err();
        assert!(error.to_string().contains("scalar"));
    }

    #[test]
    fn frontmatter_accepts_array_defaults() {
        let parsed = parse_template_document(
            "---\ndefaults:\n  test_names:\n    - login\n    - logout\n---\n{{ test_names | length }}\n",
        )
        .unwrap();
        let frontmatter = parsed.frontmatter().unwrap();

        assert_eq!(
            frontmatter
                .defaults()
                .get(&super::VariableName::new("test_names").unwrap()),
            Some(&json!(["login", "logout"]))
        );
    }

    #[test]
    fn frontmatter_accepts_input_defaults_alias() {
        let parsed = parse_template_document(
            "---\ninput_defaults:\n  assignee: teammate\n  branch: \"\"\n---\n{{ assignee }}\n",
        )
        .unwrap();
        let frontmatter = parsed.frontmatter().unwrap();

        assert_eq!(
            frontmatter
                .defaults()
                .get(&super::VariableName::new("assignee").unwrap()),
            Some(&json!("teammate"))
        );
        assert_eq!(
            frontmatter
                .defaults()
                .get(&super::VariableName::new("branch").unwrap()),
            Some(&json!(""))
        );
    }

    #[test]
    fn frontmatter_warns_when_defaults_and_input_defaults_both_exist() {
        let parsed = parse_template_document(
            "---\ndefaults:\n  name: old\ninput_defaults:\n  name: new\n---\n{{ name }}\n",
        )
        .unwrap();
        let frontmatter = parsed.frontmatter().unwrap();

        assert_eq!(
            frontmatter
                .defaults()
                .get(&super::VariableName::new("name").unwrap()),
            Some(&json!("new"))
        );
        assert!(
            frontmatter
                .diagnostics()
                .iter()
                .any(|diagnostic| {
                    diagnostic.code == super::DiagnosticCode::WarnValConflictingDefaultSections
                        && diagnostic.message.contains("input_defaults")
                })
        );
    }

    #[test]
    fn render_error_constructor_is_documented_and_usable() {
        let error = RenderError::render(std::io::Error::other("boom"));
        assert!(error.source().is_some());
    }
}
