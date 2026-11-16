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
