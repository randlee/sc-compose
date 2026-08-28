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
/// Parser-backed inspection of Jinja template-loading directives.
pub mod directive_inspection;
mod discovery;
/// Canonical crate-owned error types.
pub mod error;
/// Known-template extraction contract and report types.
pub mod extract;
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
mod path_containment;
mod path_utils;
/// Format-aware output validation before emission.
pub mod render_check;
/// Template renderer wrapper.
pub mod renderer;
/// Runtime-aware profile resolution and search tracing.
pub mod resolver;
/// Template filename extension helpers.
pub mod template_ext;
/// Shared lexical scanning for Jinja variable expressions.
pub mod template_scanner;
/// Foundational request, result, and value-model types.
pub mod types;
/// Public validation entrypoint.
pub mod validate;
/// Variable discovery and validation semantics.
pub mod validation;
/// Drift-verification entrypoints.
pub mod verify;

#[doc(inline)]
pub use composer::{
    assemble_output, compose, compose_with_observer, compose_with_observer_and_expanded,
    protect_higher_braces, render_all,
};
#[doc(inline)]
pub use diagnostics::{
    DIAGNOSTIC_SCHEMA_VERSION, Diagnostic, DiagnosticCode, DiagnosticEnvelope, DiagnosticSeverity,
};
#[doc(inline)]
pub use directive_inspection::{
    SourceSpan, TemplateDirective, TemplateDirectiveKind, inspect_template_directives,
};
pub use discovery::{
    discover_all_pass_tokens, discover_tokens, discover_tokens_with_brace_count,
    discover_tokens_with_delimiters,
};
#[doc(inline)]
pub use error::{
    ComposeError, ConfigError, IncludeError, RecoveryHint, RecoveryHintKind, RenderError,
    ResolveError, ValidationError,
};
#[doc(inline)]
pub use extract::{
    ExtractError, ExtractFormat, ExtractRequest, ExtractionDiagnostic, ExtractionDiagnosticKind,
    ExtractionOccurrence, ExtractionPathSegment, ExtractionReport, ExtractionSource,
    JsonExtractionReport, JsonExtractionSource, JsonPathSegment, OccurrenceIndex,
    OccurrencePathSegment, OccurrenceSource, RawExtractionReport, RawExtractionSource,
    RawPathSegment, TomlExtractionReport, TomlExtractionSource, TomlPathSegment,
    XmlExtractionOccurrence, XmlExtractionReport, XmlExtractionSource, XmlPathSegment,
    YamlExtractionReport, YamlExtractionSource, YamlPathSegment, extract,
};
#[doc(inline)]
pub use frontmatter::{Frontmatter, ParsedTemplate, parse_template_document};
#[doc(inline)]
pub use frontmatter_init::frontmatter_init;
#[doc(inline)]
pub use include::{CompositionFingerprint, ExpandedTemplate, expand_includes};
#[doc(inline)]
pub use init_workspace::{init_workspace, read_optional_text_file};
#[doc(inline)]
pub use observer::{
    CompositionObserver, IncludeOutcomeEvent, NoopObserver, ObservationEvent, ObservationSink,
    PassEndEvent, PassStartEvent, RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent,
    ValidationOutcomeEvent, VerifyEndEvent, VerifyStartEvent,
};
#[doc(inline)]
pub use path_utils::to_forward_slash;
#[doc(inline)]
pub use render_check::{
    CheckedOutput, ContextSummary, OutputCheckError, OutputCheckReason, OutputFormat,
    RenderCheckMeta, RenderCheckReport, check_rendered_output, check_rendered_output_with_meta,
};
#[doc(inline)]
pub use renderer::{
    JSON_LEGACY_WARNING, JsonEscapeMode, LoadedTemplateRequest, NamedTemplateAsset,
    RenderedArtifact, Renderer, TemplateEscapeMode, render_loaded_template,
    render_loaded_template_with_json_escape_mode, render_template, resolve_json_escape_mode,
};
#[doc(inline)]
pub use resolver::{resolve_profile, resolve_profile_with_observer, resolve_template_path};
#[doc(inline)]
pub use sc_sha::{
    CanonicalSource, CanonicalSourceError, CanonicalSourceUrl, CanonicalTemplatePath,
    CompositionError, CompositionSha256, HashInput, HashResult, ManifestSchemaVersion,
    ResolvedIncludeEdge, ResolvedTemplateManifest, ResolvedTemplateNode, ShaError, TemplateSha256,
    calculate_composition_hash, calculate_hash,
};
#[doc(inline)]
pub use template_ext::{
    is_json_template_path, strip_all_template_suffixes, template_content_extension,
};
#[doc(inline)]
pub use types::{
    ComposeMode, ComposePolicy, ComposeRequest, ComposeResult, ConfiningRoot,
    FrontmatterInitResult, IncludeDepth, InitResult, InputValue, InvalidInputValueError,
    InvalidProfileNameError, InvalidVariableNameError, MetadataValue, PassConfig, ProfileKind,
    ProfileName, ResolveResult, ResolverPolicy, RuntimeKind, UnknownVariablePolicy,
    ValidationReport, VariableName, VariableSource, VerifyResult, input_value_from_yaml,
    validate_input_value,
};
#[doc(inline)]
pub use validate::{
    validate, validate_with_observer, validate_with_observer_and_delimiters,
    validate_with_observer_and_delimiters_with_expansion,
};
pub use validation::BUILTIN_VARIABLE_NAMES;
#[doc(inline)]
pub use verify::{verify, verify_with_observer};

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use serde_json::json;

    use super::{RenderError, parse_template_document, render_template};

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
    fn frontmatter_defaults_accept_object_value() {
        let parsed = parse_template_document(
            "---\ndefaults:\n  pr:\n    number: 43\n    url: https://example.test/pr/43\n---\nhello {{ pr.number }}\n",
        )
        .unwrap();
        let frontmatter = parsed.frontmatter().unwrap();

        assert_eq!(
            frontmatter
                .defaults()
                .get(&super::VariableName::new("pr").unwrap()),
            Some(&json!({
                "number": 43,
                "url": "https://example.test/pr/43"
            }))
        );
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
    fn frontmatter_defaults_accept_array_of_objects() {
        let parsed = parse_template_document(
            "---\ndefaults:\n  sprints:\n    - id: H1\n      stage: merged\n    - id: H2\n      stage: in-review\n---\n{{ sprints | length }}\n",
        )
        .unwrap();
        let frontmatter = parsed.frontmatter().unwrap();

        assert_eq!(
            frontmatter
                .defaults()
                .get(&super::VariableName::new("sprints").unwrap()),
            Some(&json!([
                { "id": "H1", "stage": "merged" },
                { "id": "H2", "stage": "in-review" }
            ]))
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
        assert!(frontmatter.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == super::DiagnosticCode::WarnValConflictingDefaultSections
                && diagnostic.message.contains("input_defaults")
        }));
    }

    #[test]
    fn render_error_constructor_is_documented_and_usable() {
        let error = RenderError::render(std::io::Error::other("boom"));
        assert!(error.source().is_some());
    }
}
