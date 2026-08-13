use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use sc_composer::observer::{
    CompositionObserver, IncludeOutcomeEvent, PassEndEvent, PassStartEvent, RenderOutcomeEvent,
    ResolveAttemptEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
};
use sc_composer::{
    ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, DiagnosticCode, OutputFormat,
    VariableName, check_rendered_output, compose, compose_with_observer,
    compose_with_observer_and_expanded, expand_includes, parse_template_document,
    protect_higher_braces, render_all, resolve_template_path,
};

#[derive(Default)]
struct CapturingObserver {
    pass_start: Vec<PassStartEvent>,
    pass_end: Vec<PassEndEvent>,
    resolve: Vec<ResolveOutcomeEvent>,
    include: Vec<IncludeOutcomeEvent>,
    validation: Vec<ValidationOutcomeEvent>,
    render: Vec<RenderOutcomeEvent>,
}

impl CompositionObserver for CapturingObserver {
    fn on_pass_start(&mut self, event: &PassStartEvent) {
        self.pass_start.push(event.clone());
    }

    fn on_pass_end(&mut self, event: &PassEndEvent) {
        self.pass_end.push(event.clone());
    }

    fn on_resolve_attempt(&mut self, _event: &ResolveAttemptEvent) {}

    fn on_resolve_outcome(&mut self, event: &ResolveOutcomeEvent) {
        self.resolve.push(event.clone());
    }

    fn on_include_outcome(&mut self, event: &IncludeOutcomeEvent) {
        self.include.push(event.clone());
    }

    fn on_validation_outcome(&mut self, event: &ValidationOutcomeEvent) {
        self.validation.push(event.clone());
    }

    fn on_render_outcome(&mut self, event: &RenderOutcomeEvent) {
        self.render.push(event.clone());
    }
}

#[test]
fn render_all_renders_two_pass_template() {
    let parsed = parse_template_document(
        "---\npass: 2\ndefaults:\n  install_dir: /opt/tools\n---\n---\ndefaults:\n  script_name: run.sh\n---\nPath={{{ install_dir }}}\nScript={{ script_name }}\n",
    )
    .unwrap();

    let contexts = vec![
        (
            2,
            BTreeMap::from([(VariableName::new("install_dir").unwrap(), json!("/srv/app"))]),
        ),
        (
            1,
            BTreeMap::from([(
                VariableName::new("script_name").unwrap(),
                json!("deploy.sh"),
            )]),
        ),
    ];

    let rendered = render_all(&parsed, &contexts).unwrap();

    assert_eq!(rendered, "Path=/srv/app\nScript=deploy.sh");
}

#[test]
fn render_all_merges_frontmatter_defaults_for_direct_callers() {
    let parsed = parse_template_document(
        "---\npass: 2\ndefaults:\n  install_dir: /opt/tools\n---\n---\npass: 1\ndefaults:\n  script_name: run.sh\n---\nPath={{{ install_dir }}}\nScript={{ script_name }}\n",
    )
    .unwrap();

    let rendered = render_all(&parsed, &[(2, BTreeMap::new()), (1, BTreeMap::new())]).unwrap();

    assert_eq!(rendered, "Path=/opt/tools\nScript=run.sh");
}

#[test]
fn render_all_renders_three_pass_template() {
    let parsed = parse_template_document(
        "---\npass: 3\n---\n---\npass: 2\n---\n---\npass: 1\n---\nOrg={{{{ org }}}}\nTeam={{{ team }}}\nUser={{ user }}\n",
    )
    .unwrap();

    let rendered = render_all(
        &parsed,
        &[
            (
                3,
                BTreeMap::from([(VariableName::new("org").unwrap(), json!("acme"))]),
            ),
            (
                2,
                BTreeMap::from([(VariableName::new("team").unwrap(), json!("platform"))]),
            ),
            (
                1,
                BTreeMap::from([(VariableName::new("user").unwrap(), json!("randlee"))]),
            ),
        ],
    )
    .unwrap();

    assert_eq!(rendered, "Org=acme\nTeam=platform\nUser=randlee");
}

#[test]
fn render_all_public_api_keeps_inline_template_unescaped() {
    let parsed =
        parse_template_document("---\nname: inline\n---\n<root>{{ note }}</root>\n").unwrap();
    let rendered = render_all(
        &parsed,
        &[(
            1,
            BTreeMap::from([(VariableName::new("note").unwrap(), json!("<tag> &"))]),
        )],
    )
    .unwrap();

    assert_eq!(rendered, "<root><tag> &</root>");
}

#[test]
fn render_all_errors_on_context_count_mismatch() {
    let parsed = parse_template_document("---\npass: 2\n---\n---\n---\nbody").unwrap();

    let error = render_all(&parsed, &[(2, BTreeMap::new())]).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("expected 2 render contexts for 2 passes, got 1")
    );
}

#[test]
fn render_all_errors_on_pass_number_mismatch() {
    let parsed = parse_template_document("---\npass: 2\n---\nbody").unwrap();

    let error = render_all(&parsed, &[(1, BTreeMap::new())]).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("render context pass 1 does not match header pass 2")
    );
}

#[test]
fn render_all_with_empty_passes_returns_body_unchanged() {
    let parsed = parse_template_document("body").unwrap();

    let rendered = render_all(&parsed, &[]).unwrap();

    assert_eq!(rendered, "body");
}

#[test]
fn protect_higher_braces_wraps_only_next_higher_brace_count() {
    assert_eq!(
        protect_higher_braces("{{{ x }}}", 2),
        "{% raw %}{{{ x }}}{% endraw %}"
    );
    assert_eq!(protect_higher_braces("{{ x }}", 2), "{{ x }}");
}

#[test]
fn compose_single_pass_backward_compat_output_is_unchanged() {
    let root = temp_root("single-pass-backcompat");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let rendered = compose(&ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: PathBuf::from("template.md.j2"),
        },
        root: ConfiningRoot::new(&root).unwrap(),
        vars_input: BTreeMap::default(),
        vars_env: BTreeMap::default(),
        vars_defaults: BTreeMap::default(),
        guidance_block: None,
        user_prompt: None,
        policy: ComposePolicy::default(),
    })
    .unwrap();

    assert_eq!(rendered.rendered_text, "hello world");
}

#[test]
fn compose_multi_pass_output_is_unchanged_when_render_all_reapplies_defaults() {
    let root = temp_root("multi-pass-compose-default-idempotent");
    write_file(
        &root.join("template.md.j2"),
        "---\npass: 2\ndefaults:\n  install_dir: /opt/tools\n---\n---\npass: 1\ndefaults:\n  script_name: run.sh\n---\nPath={{{ install_dir }}}\nScript={{ script_name }}\n",
    );

    let rendered = compose(&ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: PathBuf::from("template.md.j2"),
        },
        root: ConfiningRoot::new(&root).unwrap(),
        vars_input: BTreeMap::default(),
        vars_env: BTreeMap::default(),
        vars_defaults: BTreeMap::default(),
        guidance_block: None,
        user_prompt: None,
        policy: ComposePolicy::default(),
    })
    .unwrap();

    assert_eq!(rendered.rendered_text, "Path=/opt/tools\nScript=run.sh");
}

#[test]
fn parse_template_document_allows_omitted_duplicate_default_pass_numbers() {
    let parsed = parse_template_document("---\n---\n---\n---\nbody").unwrap();

    assert_eq!(parsed.passes().len(), 2);
    assert_eq!(parsed.passes()[0].pass_number(), 1);
    assert_eq!(parsed.passes()[1].pass_number(), 1);
    assert_eq!(parsed.body(), "body");
}

#[test]
fn parse_template_document_rejects_duplicate_explicit_pass_numbers() {
    let error = parse_template_document("---\npass: 2\n---\n---\npass: 2\n---\nbody").unwrap_err();

    assert!(
        error
            .to_string()
            .contains("duplicate explicit pass number in stacked frontmatter")
    );
}

#[test]
fn compose_with_observer_emits_pass_events_for_multi_pass_template() {
    let root = temp_root("multi-pass-observer");
    write_file(
        &root.join("template.md.j2"),
        "---\npass: 2\ndefaults:\n  install_dir: /opt/tools\n---\n---\ndefaults:\n  script_name: run.sh\n---\nPath={{{ install_dir }}}\nScript={{ script_name }}\n",
    );

    let mut observer = CapturingObserver::default();
    let result = compose_with_observer(
        &ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input: BTreeMap::from([
                (VariableName::new("install_dir").unwrap(), json!("/srv/app")),
                (
                    VariableName::new("script_name").unwrap(),
                    json!("deploy.sh"),
                ),
            ]),
            vars_env: BTreeMap::default(),
            vars_defaults: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        },
        &mut observer,
    )
    .unwrap();

    assert_eq!(result.rendered_text, "Path=/srv/app\nScript=deploy.sh");
    assert_eq!(
        observer
            .pass_start
            .iter()
            .map(|event| event.pass_number)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert_eq!(
        observer
            .pass_end
            .iter()
            .map(|event| event.pass_number)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
}

#[test]
fn compose_with_expanded_reuses_preflight_source_after_include_mutation() {
    let root = temp_root("compose_reuse_preflight_expansion");
    let template_path = root.join("template.md.j2");
    let included_path = root.join("partials/body.md.j2");
    write_file(&template_path, "@<partials/body.md.j2>\n");
    write_file(&included_path, "hello {{ name }}\n");

    let request = ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: PathBuf::from("template.md.j2"),
        },
        root: ConfiningRoot::new(&root).unwrap(),
        vars_input: BTreeMap::from([(VariableName::new("name").unwrap(), json!("world"))]),
        vars_env: BTreeMap::default(),
        vars_defaults: BTreeMap::default(),
        guidance_block: None,
        user_prompt: None,
        policy: ComposePolicy::default(),
    };
    let resolved = resolve_template_path(&request).unwrap();
    let expanded =
        expand_includes(&resolved.resolved_path, &request.root, &request.policy).unwrap();

    write_file(&included_path, "mutated after preflight {{ name }}\n");

    let mut observer = CapturingObserver::default();
    let result =
        compose_with_observer_and_expanded(&request, &mut observer, resolved, expanded).unwrap();

    assert_eq!(result.rendered_text, "hello world");
    assert_eq!(observer.include.len(), 1);
}

#[test]
fn compose_multi_pass_preserves_still_higher_brace_literals_across_lower_passes() {
    let root = temp_root("higher-brace-literal");
    write_file(
        &root.join("template.md.j2"),
        "---\npass: 3\n---\n---\npass: 2\n---\n---\npass: 1\n---\nLiteral={{{ literal_block }}}\nTeam={{{ team }}}\nRendered={{ script_name }}\n",
    );

    let result = compose(&ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: PathBuf::from("template.md.j2"),
        },
        root: ConfiningRoot::new(&root).unwrap(),
        vars_input: BTreeMap::from([
            (
                VariableName::new("literal_block").unwrap(),
                json!("{{{{ org }}}}"),
            ),
            (VariableName::new("team").unwrap(), json!("platform")),
            (
                VariableName::new("script_name").unwrap(),
                json!("deploy.sh"),
            ),
        ]),
        vars_env: BTreeMap::default(),
        vars_defaults: BTreeMap::default(),
        guidance_block: None,
        user_prompt: None,
        policy: ComposePolicy::default(),
    })
    .unwrap();

    assert_eq!(
        result.rendered_text,
        "Literal={{{{ org }}}}\nTeam=platform\nRendered=deploy.sh"
    );
}

#[test]
fn compose_with_observer_emits_single_render_failure_event_for_multi_pass_errors() {
    let root = temp_root("multi-pass-render-failure-event");
    write_file(
        &root.join("template.md.j2"),
        "---\npass: 2\n---\n---\npass: 1\n---\nBroken={{{ invalid + }}}\nRendered={{ script_name }}\n",
    );

    let mut observer = CapturingObserver::default();
    let error = compose_with_observer(
        &ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input: BTreeMap::from([(
                VariableName::new("script_name").unwrap(),
                json!("deploy.sh"),
            )]),
            vars_env: BTreeMap::default(),
            vars_defaults: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        },
        &mut observer,
    )
    .unwrap_err();

    let source = std::error::Error::source(&error)
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(source.contains("syntax error"));
    assert_eq!(observer.render.len(), 1);
    assert_eq!(observer.render[0].rendered_bytes, None);
}

#[test]
fn compose_then_checked_output_rejects_malformed_json_without_echoing_rendered_value() {
    let root = temp_root("checked-json-malformed");
    let template_path = root.join("payload.json.j2");
    let secret = "TOP_SECRET_CHECKED_OUTPUT";
    write_file(&template_path, r#"{"value": "{{ value }}"}"#);

    let result = compose(&ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: PathBuf::from("payload.json.j2"),
        },
        root: ConfiningRoot::new(&root).unwrap(),
        vars_input: BTreeMap::from([(VariableName::new("value").unwrap(), json!(secret))]),
        vars_env: BTreeMap::default(),
        vars_defaults: BTreeMap::default(),
        guidance_block: None,
        user_prompt: None,
        policy: ComposePolicy::default(),
    })
    .unwrap();

    let error = check_rendered_output(
        OutputFormat::Json,
        &result.resolve_result.resolved_path,
        &result.rendered_text,
    )
    .expect_err("manually quoted placeholder must fail the auto JSON contract");

    assert_eq!(
        error.diagnostics[0].code,
        DiagnosticCode::ErrRenderJsonMalformed
    );
    assert_eq!(
        error.diagnostics[0].path,
        Some(result.resolve_result.resolved_path.clone())
    );
    assert!(error.diagnostics[0].line.is_some());
    assert!(error.diagnostics[0].column.is_some());
    assert!(!format!("{error}").contains(secret));
    assert!(!format!("{error:?}").contains(secret));
}

#[test]
fn compose_then_checked_output_emits_valid_json_only_through_checked_output() {
    let root = temp_root("checked-json-valid");
    write_file(&root.join("payload.json.j2"), r#"{"value": {{ value }}}"#);

    let result = compose(&ComposeRequest {
        runtime: None,
        mode: ComposeMode::File {
            template_path: PathBuf::from("payload.json.j2"),
        },
        root: ConfiningRoot::new(&root).unwrap(),
        vars_input: BTreeMap::from([(VariableName::new("value").unwrap(), json!("checked-value"))]),
        vars_env: BTreeMap::default(),
        vars_defaults: BTreeMap::default(),
        guidance_block: None,
        user_prompt: None,
        policy: ComposePolicy::default(),
    })
    .unwrap();

    let checked = check_rendered_output(
        OutputFormat::Json,
        &result.resolve_result.resolved_path,
        &result.rendered_text,
    )
    .expect("valid JSON must produce CheckedOutput");
    let mut emitted = Vec::new();
    checked.emit(&mut emitted).unwrap();

    assert_eq!(checked.body(), result.rendered_text);
    assert_eq!(String::from_utf8(emitted).unwrap(), result.rendered_text);
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sc-composer-d2-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}
