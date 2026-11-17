//! Public composition regressions for the frontmatter discovery boundary.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use sc_composer::{
    ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, DiagnosticCode,
    UnknownVariablePolicy, VariableName, compose, validate,
};
use serde_json::json;

static NEXT: AtomicU64 = AtomicU64::new(0);
const HEADER: &str = "---\nrequired_variables: [x]\n---\n";
const CASES: [(&str, &str, &[&str]); 3] = [
    (
        "{% autoescape false %}hi {{ x }}{% endautoescape %}",
        "hi a<b",
        &["autoescape", "endautoescape", "x"],
    ),
    (
        "{% raw %}{{ hidden }}{% endraw %}hi {{ x }}",
        "{{ hidden }}hi a<b",
        &["endraw", "hidden", "raw", "x"],
    ),
    (
        "{% filter upper %}hi {{ x }}{% endfilter %}",
        "HI A<B",
        &["upper", "x"],
    ),
];

struct Fixture(PathBuf);

impl Fixture {
    fn new(text: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "sc-compose-1043-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("template.md.j2"), text).unwrap();
        Self(root)
    }

    fn request(&self) -> ComposeRequest {
        ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            root: ConfiningRoot::new(&self.0).unwrap(),
            vars_input: [(VariableName::new("x").unwrap(), json!("a<b"))].into(),
            vars_env: BTreeMap::new(),
            vars_defaults: BTreeMap::new(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy {
                strict_undeclared_variables: false,
                unknown_variable_policy: UnknownVariablePolicy::Error,
                unbound_variable_policy: Some(UnknownVariablePolicy::Error),
                ..ComposePolicy::default()
            },
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn assert_frontmatter_compose(case: usize) {
    let (body, expected, _) = CASES[case];
    let fixture = Fixture::new(&format!("{HEADER}{body}"));
    let request = fixture.request();
    assert!(!request.policy.strict_undeclared_variables);
    let result = compose(&request).unwrap_or_else(|error| panic!("{body}: {error}"));
    assert_eq!(result.rendered_text, expected);
    assert!(result.warnings.is_empty(), "{body}: {:?}", result.warnings);
}

#[test]
fn frontmatter_autoescape_composes_with_error_policies_and_strict_off() {
    assert_frontmatter_compose(0);
}

#[test]
fn frontmatter_raw_composes_with_error_policies_and_strict_off() {
    assert_frontmatter_compose(1);
}

#[test]
fn frontmatter_filter_composes_with_error_policies_and_strict_off() {
    assert_frontmatter_compose(2);
}

#[test]
fn schemaless_templates_keep_exact_legacy_tag_token_warnings() {
    for (body, expected, tokens) in CASES {
        let fixture = Fixture::new(body);
        let mut request = fixture.request();
        request.policy.unknown_variable_policy = UnknownVariablePolicy::Ignore;
        request.policy.unbound_variable_policy = Some(UnknownVariablePolicy::Ignore);
        let result = compose(&request).unwrap();
        assert_eq!(result.rendered_text, expected);
        let messages: Vec<_> = result
            .warnings
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::ErrValUndeclaredToken)
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        let expected: Vec<_> = tokens
            .iter()
            .map(|name| format!("undeclared referenced token: {name}"))
            .collect();
        assert_eq!(messages, expected, "{body}");
    }
}

#[test]
fn frontmatter_argument_scan_does_not_make_real_undeclared_tokens_fatal() {
    let fixture = Fixture::new(&format!("---\n---\n{}", CASES[0].0));
    let result = compose(&fixture.request()).unwrap();
    assert_eq!(result.rendered_text, "hi a<b");
    assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
    assert_eq!(
        result.warnings[0].code,
        DiagnosticCode::ErrValUndeclaredToken
    );
    assert_eq!(result.warnings[0].message, "undeclared referenced token: x");
}

#[test]
fn undeclared_tag_named_input_is_unknown_but_declared_input_is_accepted() {
    let fixture = Fixture::new(&format!("{HEADER}{}", CASES[0].0));
    let mut request = fixture.request();
    request
        .vars_input
        .insert(VariableName::new("autoescape").unwrap(), json!(false));
    let report = validate(&request).unwrap();
    assert!(!report.ok, "{report:?}");
    assert_eq!(report.errors.len(), 1, "{report:?}");
    assert_eq!(report.errors[0].code, DiagnosticCode::ErrValExtraInput);
    assert_eq!(
        report.errors[0].message,
        "extra provided variable: autoescape"
    );
    fs::write(
        fixture.0.join("template.md.j2"),
        format!(
            "---\nrequired_variables: [x, autoescape]\n---\n{}",
            CASES[0].0
        ),
    )
    .unwrap();
    assert_eq!(compose(&request).unwrap().rendered_text, "hi a<b");
}

#[test]
fn custom_delimiters_use_the_frontmatter_scan_with_strict_off() {
    let fixture = Fixture::new(&format!(
        "{HEADER}{{% autoescape false %}}[[ x ]]{{% endautoescape %}}"
    ));
    let mut observer = sc_composer::observer::NoopObserver;
    let report = sc_composer::validate_with_observer_and_delimiters(
        &fixture.request(),
        &mut observer,
        Some(("[[", "]]")),
    )
    .unwrap();
    assert!(report.ok, "{report:?}");
    assert!(report.warnings.is_empty(), "{report:?}");
}

const FRAGMENT: &str = "{% autoescape false %}literal{% endautoescape %}{% raw %}{{ hidden_raw }}{% endraw %}{# {{ hidden_comment }} #}";

fn included_fragment(frontmatter: bool, real_variable: bool) -> Fixture {
    let header = if frontmatter { HEADER } else { "" };
    let fixture = Fixture::new(&format!("{header}{{{{ x }}}}\n@<fragment.md.j2>\n"));
    let variable = if real_variable { "{{ x }}" } else { "" };
    fs::write(
        fixture.0.join("fragment.md.j2"),
        format!("{FRAGMENT}{variable}"),
    )
    .unwrap();
    fixture
}

#[test]
fn frontmatter_root_applies_argument_scan_to_schemaless_fragment_and_its_warning() {
    let fixture = included_fragment(true, false);
    let result = compose(&fixture.request()).unwrap();
    assert!(result.rendered_text.starts_with("a<b\n"), "{result:?}");
    assert!(
        result.rendered_text.contains("literal{{ hidden_raw }}"),
        "{result:?}"
    );
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

#[test]
fn fragment_with_a_real_reference_still_warns_and_renders_the_root_input() {
    let fixture = included_fragment(true, true);
    let result = compose(&fixture.request()).unwrap();
    assert_eq!(result.rendered_text.matches("a<b").count(), 2, "{result:?}");
    assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
    let warning = &result.warnings[0];
    assert_eq!(warning.code, DiagnosticCode::ErrValMissingFrontmatter);
    assert!(
        warning
            .message
            .starts_with("included file has no frontmatter;"),
        "{warning:?}"
    );
    assert_eq!(
        warning.path.as_ref().unwrap().file_name().unwrap(),
        "fragment.md.j2"
    );
}

#[test]
fn schemaless_root_keeps_legacy_fragment_tokens_and_both_missing_header_warnings() {
    let fixture = included_fragment(false, false);
    let mut request = fixture.request();
    request.policy.unknown_variable_policy = UnknownVariablePolicy::Ignore;
    request.policy.unbound_variable_policy = Some(UnknownVariablePolicy::Ignore);
    let result = compose(&request).unwrap();
    let tokens: Vec<_> = result
        .warnings
        .iter()
        .filter(|warning| warning.code == DiagnosticCode::ErrValUndeclaredToken)
        .map(|warning| warning.message.as_str())
        .collect();
    assert_eq!(
        tokens,
        [
            "undeclared referenced token: autoescape",
            "undeclared referenced token: endautoescape",
            "undeclared referenced token: endraw",
            "undeclared referenced token: hidden_comment",
            "undeclared referenced token: hidden_raw",
            "undeclared referenced token: raw",
            "undeclared referenced token: x",
        ]
    );
    let missing: Vec<_> = result
        .warnings
        .iter()
        .filter(|warning| warning.code == DiagnosticCode::ErrValMissingFrontmatter)
        .collect();
    assert_eq!(missing.len(), 2, "{missing:?}");
    assert!(
        missing[0]
            .message
            .starts_with("root template has no frontmatter;")
    );
    assert!(
        missing[1]
            .message
            .starts_with("included file has no frontmatter;")
    );
    assert_eq!(
        missing[1].path.as_ref().unwrap().file_name().unwrap(),
        "fragment.md.j2"
    );
}

#[test]
fn strict_schemaless_root_keeps_legacy_missing_fragment_frontmatter_warning() {
    let fixture = included_fragment(false, false);
    fs::write(fixture.0.join("template.md.j2"), "@<fragment.md.j2>\n").unwrap();
    let mut request = fixture.request();
    request.vars_input.clear();
    request.policy.strict_undeclared_variables = true;
    let result = compose(&request).unwrap();
    assert_eq!(result.warnings.len(), 1, "{:?}", result.warnings);
    assert_eq!(
        result.warnings[0].code,
        DiagnosticCode::ErrValMissingFrontmatter
    );
    assert_eq!(
        result.warnings[0]
            .path
            .as_ref()
            .unwrap()
            .file_name()
            .unwrap(),
        "fragment.md.j2"
    );
}
