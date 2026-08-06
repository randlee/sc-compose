//! Template renderer wrapper.

use std::collections::BTreeMap;

use minijinja::value::ValueKind;
use minijinja::{
    AutoEscape, Environment, Error, Output, State, Value as JinjaValue, escape_formatter,
};
use serde::Serialize;
use serde_json::Value;

use crate::RenderError;

const XML_REPLACEMENT_NCR: &str = "&#xfffd;";

/// Additional named templates that the main template may extend or include.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NamedTemplateAsset {
    /// Stable template identifier used for loader lookups.
    pub template_name: String,
    /// Template body associated with the identifier.
    pub template_text: String,
}

/// Request for rendering template text that the caller already loaded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoadedTemplateRequest {
    /// Stable template identifier used for diagnostics and template naming.
    pub template_name: String,
    /// Pre-loaded template text to render.
    pub template_text: String,
    /// Render context supplied by the caller.
    pub context: BTreeMap<String, Value>,
    /// Additional named templates the main template may extend or include.
    #[serde(default)]
    pub supporting_templates: Vec<NamedTemplateAsset>,
}

/// Artifact returned by the pre-loaded template render path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RenderedArtifact {
    /// Final rendered output text.
    pub rendered: String,
    /// Stable template identifier used during rendering.
    pub template_name: String,
}

/// Pure template-engine wrapper used by composition entry points.
#[derive(Debug)]
pub struct Renderer {
    env: Environment<'static>,
}

fn legacy_auto_escape_callback(name: &str) -> AutoEscape {
    let mut name = name;
    for extension in [".j2", ".jinja2", ".jinja"] {
        if let Some(stripped) = name.strip_suffix(extension) {
            name = stripped;
            break;
        }
    }

    match name.rsplit('.').next() {
        Some("html" | "htm" | "xml" | "xhtml") => AutoEscape::Custom("sc-compose-html"),
        Some("json") => AutoEscape::Json,
        _ => AutoEscape::None,
    }
}

fn cdata_escape_filter(value: &str) -> JinjaValue {
    // Split CDATA before the terminating `]]>`; mark the result safe so the
    // XML formatter does not entity-escape the reopened `<![CDATA[>` tag.
    JinjaValue::from_safe_string(value.replace("]]>", "]]]]><![CDATA[>"))
}

fn turtle_escape_filter(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn escape_markup(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn encode_xml_illegal_controls(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for character in value.chars() {
        let code = character as u32;
        if (0..=0x08).contains(&code) || matches!(code, 0x0b | 0x0c | 0x0e..=0x1f) {
            encoded.push_str(XML_REPLACEMENT_NCR);
        } else {
            encoded.push(character);
        }
    }
    encoded
}

fn xml_char_safe_string(value: &str) -> String {
    encode_xml_illegal_controls(&escape_markup(value))
}

fn xml_char_safe_filter(value: &str) -> JinjaValue {
    JinjaValue::from_safe_string(xml_char_safe_string(value))
}

fn frontmatter_safe_filter(value: &str) -> JinjaValue {
    let escaped = value
        .split('\n')
        .map(|line| {
            let replacement = match line.trim() {
                "---" => Some(("---", r"\-\-\-")),
                "..." => Some(("...", r"\.\.\.")),
                _ => None,
            };
            replacement.map_or_else(
                || line.to_owned(),
                |(delimiter, escaped_delimiter)| {
                    let delimiter_start = line
                        .find(delimiter)
                        .expect("trimmed delimiter must occur in the source line");
                    let delimiter_end = delimiter_start + delimiter.len();
                    format!(
                        "{}{}{}",
                        &line[..delimiter_start],
                        escaped_delimiter,
                        &line[delimiter_end..]
                    )
                },
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    JinjaValue::from_safe_string(escaped)
}

fn yaml_safe_filter(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r");
    format!("\"{escaped}\"")
}

fn format_sc_compose_markup(
    out: &mut Output<'_>,
    state: &State<'_, '_>,
    value: &JinjaValue,
) -> Result<(), Error> {
    let value = if value.is_none() {
        &JinjaValue::UNDEFINED
    } else {
        value
    };
    if state.auto_escape() != AutoEscape::Custom("sc-compose-html") {
        return escape_formatter(out, state, value);
    }
    if value.is_safe() {
        return out
            .write_str(value.as_str().unwrap_or_default())
            .map_err(Error::from);
    }

    let rendered = value
        .as_str()
        .map_or_else(|| value.to_string(), ToOwned::to_owned);
    out.write_str(&xml_char_safe_string(&rendered))
        .map_err(Error::from)
}

fn map_get_unknown_method_callback(
    _state: &State<'_, '_>,
    value: &JinjaValue,
    method: &str,
    args: &[JinjaValue],
) -> Result<JinjaValue, Error> {
    if value.kind() != ValueKind::Map || method != "get" || !(1..=2).contains(&args.len()) {
        return Err(Error::from(minijinja::ErrorKind::UnknownMethod));
    }

    let found = value.get_item(&args[0])?;
    if !found.is_undefined() {
        Ok(found)
    } else if args.len() == 2 {
        Ok(args[1].clone())
    } else {
        Ok(JinjaValue::UNDEFINED)
    }
}

fn configure_environment(env: &mut Environment<'static>) {
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    // Keep sc-compose's historical extension policy when Minijinja's `json`
    // feature is enabled. JSON/YAML/JS templates are text outputs, not HTML.
    env.set_auto_escape_callback(legacy_auto_escape_callback);
    env.set_formatter(format_sc_compose_markup);
    env.add_filter("cdata_escape", cdata_escape_filter);
    env.add_filter("turtle_escape", turtle_escape_filter);
    env.add_filter("xml_char_safe", xml_char_safe_filter);
    env.add_filter("frontmatter_safe", frontmatter_safe_filter);
    env.add_filter("yaml_safe", yaml_safe_filter);
    env.set_unknown_method_callback(map_get_unknown_method_callback);
}

impl Renderer {
    /// Create a renderer with the default environment options.
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(|_| {})
    }

    /// Create a renderer with additional environment configuration.
    #[must_use]
    pub(crate) fn with_options(configure: impl FnOnce(&mut Environment<'static>)) -> Self {
        Self::try_with_options(|env| {
            configure(env);
            Ok(())
        })
        .expect("default renderer options must stay valid")
    }

    /// Create a renderer with additional environment configuration that may
    /// fail while applying syntax changes.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when `configure` installs invalid renderer
    /// syntax for the underlying template engine.
    pub(crate) fn try_with_options(
        configure: impl FnOnce(&mut Environment<'static>) -> Result<(), RenderError>,
    ) -> Result<Self, RenderError> {
        let mut env = Environment::new();
        configure_environment(&mut env);
        configure(&mut env)?;
        Ok(Self { env })
    }

    /// Create a renderer with non-default variable delimiters.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if `open` or `close` are not valid delimiter
    /// tokens accepted by the underlying template engine.
    pub fn with_delimiters(open: &str, close: &str) -> Result<Self, RenderError> {
        let open = open.to_owned();
        let close = close.to_owned();
        Self::try_with_options(|env| {
            let syntax = minijinja::syntax::SyntaxConfig::builder()
                .variable_delimiters(open, close)
                .build()
                .map_err(RenderError::render)?;
            env.set_syntax(syntax);
            Ok(())
        })
    }

    /// Render a template string with the provided serializable context.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when template parsing or rendering fails.
    pub fn render<T: serde::Serialize>(
        &self,
        template: &str,
        context: T,
    ) -> Result<String, RenderError> {
        self.render_named("inline", template, context)
    }

    /// Render a template string with an explicit template name.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when template parsing or rendering fails.
    pub fn render_named<T: serde::Serialize>(
        &self,
        template_name: &str,
        template: &str,
        context: T,
    ) -> Result<String, RenderError> {
        let template = self
            .env
            .template_from_named_str(template_name, template)
            .map_err(RenderError::render)?;
        template.render(context).map_err(RenderError::render)
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a template string with the provided serializable context.
///
/// This is the stable one-shot convenience API over [`Renderer`].
///
/// # Errors
///
/// Returns [`RenderError`] when template parsing or rendering fails.
pub fn render_template<T: serde::Serialize>(
    template: &str,
    context: T,
) -> Result<String, RenderError> {
    Renderer::new().render(template, context)
}

/// Render pre-loaded template content without taking ownership of file
/// discovery or repository traversal.
///
/// # Errors
///
/// Returns [`RenderError`] when template parsing or rendering fails.
pub fn render_loaded_template(
    request: LoadedTemplateRequest,
) -> Result<RenderedArtifact, RenderError> {
    let mut env = Environment::new();
    configure_environment(&mut env);
    for asset in request.supporting_templates {
        env.add_template_owned(asset.template_name, asset.template_text)
            .map_err(RenderError::render)?;
    }
    env.add_template_owned(request.template_name.clone(), request.template_text.clone())
        .map_err(RenderError::render)?;
    let rendered = env
        .get_template(&request.template_name)
        .map_err(RenderError::render)?
        .render(&request.context)
        .map_err(RenderError::render)?;
    Ok(RenderedArtifact {
        rendered,
        template_name: request.template_name,
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::str;

    use quick_xml::Reader;
    use quick_xml::events::Event;
    use serde_json::json;

    use super::{
        LoadedTemplateRequest, NamedTemplateAsset, Renderer, XML_REPLACEMENT_NCR,
        render_loaded_template, turtle_escape_filter,
    };

    #[test]
    fn renderer_can_render_multiple_templates_with_one_environment() {
        let renderer = Renderer::new();

        let first = renderer.render("hello {{ name }}", json!({ "name": "world" }));
        let second = renderer.render("bye {{ name }}", json!({ "name": "world" }));

        assert_eq!(first.unwrap(), "hello world");
        assert_eq!(second.unwrap(), "bye world");
    }

    #[test]
    fn renderer_supports_jinja_plus_modifier_opt_out() {
        let renderer = Renderer::new();

        let output = renderer
            .render(
                "before\n    {%+ if true %}\nvalue\n    {% endif %}\nafter\n",
                json!({}),
            )
            .unwrap();

        assert_eq!(output, "before\n    value\nafter");
    }

    #[test]
    fn renderer_keeps_auto_escape_scoped_to_html_like_names() {
        let renderer = Renderer::new();
        let context = json!({ "value": "<tag> &" });

        for template_name in [
            "payload.json5",
            "payload.js",
            "payload.yaml",
            "payload.yml",
            "payload.txt",
        ] {
            let output = renderer
                .render_named(template_name, "{{ value }}", context.clone())
                .unwrap();
            assert_eq!(output, "<tag> &", "unexpected escaping for {template_name}");
        }

        assert_eq!(
            renderer
                .render_named("payload.json", "{{ value }}", context.clone())
                .unwrap(),
            "\"<tag> &\""
        );

        for template_name in [
            "payload.html",
            "payload.htm",
            "payload.xml",
            "payload.html.j2",
        ] {
            let output = renderer
                .render_named(template_name, "{{ value }}", context.clone())
                .unwrap();
            assert_eq!(
                output, "&lt;tag&gt; &amp;",
                "missing escaping for {template_name}"
            );
        }

        let loaded = render_loaded_template(LoadedTemplateRequest {
            template_name: "payload.yml".to_owned(),
            template_text: "{{ value }}".to_owned(),
            context: context.as_object().unwrap().clone().into_iter().collect(),
            supporting_templates: Vec::new(),
        })
        .unwrap();
        assert_eq!(loaded.rendered, "<tag> &");
    }

    #[test]
    fn xml_char_safe_escapes_markup_then_all_xml_illegal_controls() {
        let renderer = Renderer::new();
        let mut input = "<".to_owned();
        for byte in 0_u8..=0x1f {
            input.push(byte as char);
        }
        input.push_str(">&\"'");

        let output = renderer
            .render("{{ value | xml_char_safe }}", json!({"value": input}))
            .unwrap();

        assert_eq!(output.matches(XML_REPLACEMENT_NCR).count(), 29);
        assert!(output.starts_with("&lt;"));
        assert!(output.contains(&format!(
            "{XML_REPLACEMENT_NCR}\t\n{XML_REPLACEMENT_NCR}{XML_REPLACEMENT_NCR}\r{XML_REPLACEMENT_NCR}"
        )));
        assert!(output.ends_with("&gt;&amp;&quot;&#x27;"));
    }

    #[test]
    fn xml_char_safe_preserves_ordinary_text() {
        let renderer = Renderer::new();
        let output = renderer
            .render(
                "{{ value | xml_char_safe }}",
                json!({"value": "ordinary text"}),
            )
            .unwrap();

        assert_eq!(output, "ordinary text");
    }

    #[test]
    fn xml_char_safe_protects_explicit_escape_and_xml_auto_escape() {
        let renderer = Renderer::new();
        let context = json!({"value": "<\0>"});

        let explicit = renderer
            .render_named(
                "report.xml.j2",
                "<root>{{ value | e }}</root>",
                context.clone(),
            )
            .unwrap();
        let implicit = renderer
            .render_named("report.xml.j2", "<root>{{ value }}</root>", context)
            .unwrap();

        assert_eq!(explicit, "<root>&lt;&#xfffd;&gt;</root>");
        assert_eq!(implicit, explicit);
    }

    #[test]
    fn xhtml_uses_the_xml_auto_escape_path() {
        let renderer = Renderer::new();
        let output = renderer
            .render_named(
                "report.xhtml.j2",
                "<root>{{ value }}</root>",
                json!({"value": "\0"}),
            )
            .unwrap();

        assert_eq!(output, "<root>&#xfffd;</root>");
    }

    #[test]
    fn renderer_preserves_slashes_in_markup_auto_escape() {
        let renderer = Renderer::new();
        let context = json!({ "value": "/tmp/path/to/report.xml" });

        for template_name in ["payload.html.j2", "payload.htm", "payload.xml"] {
            let output = renderer
                .render_named(template_name, "{{ value }}", context.clone())
                .unwrap();
            assert_eq!(output, "/tmp/path/to/report.xml", "changed {template_name}");
        }
    }

    #[test]
    fn renderer_json_auto_escape_round_trips_json_string_values() {
        let renderer = Renderer::new();
        let original = "quote \" slash \\\nline";
        let output = renderer
            .render_named("payload.json.j2", "{{ value }}", json!({"value": original}))
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed, json!(original));
    }

    #[test]
    fn renderer_json_auto_escape_prevents_injected_top_level_keys() {
        let renderer = Renderer::new();
        let injected = r#"x", "injected": true, "y": "x"#;
        let output = renderer
            .render_named(
                "payload.json.j2",
                r#"{"sprint_id": {{ sprint_id }}}"#,
                json!({"sprint_id": injected}),
            )
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["sprint_id"], json!(injected));
        assert!(parsed.get("injected").is_none());
    }

    fn sprint_plan_body() -> &'static str {
        let source = include_str!("../../../.claude/skills/codex-orchestration/sprint-plan.md.j2");
        source
            .strip_prefix("---\n")
            .and_then(|source| source.split_once("\n---\n"))
            .map(|(_, body)| body)
            .expect("sprint-plan template must have declaration frontmatter")
    }

    fn sprint_plan_context(title: &str) -> serde_json::Value {
        json!({
            "id": "1.2",
            "title": title,
            "status": "planned",
            "branch": "main",
            "target": "develop",
        })
    }

    fn standalone_frontmatter_delimiter_count(rendered: &str) -> usize {
        rendered.lines().filter(|line| *line == "---").count()
    }

    #[test]
    fn frontmatter_safe_escapes_standalone_delimiters() {
        let renderer = Renderer::new();
        let output = renderer
            .render(
                "{{ value | frontmatter_safe }}",
                json!({"value": "before\n---\nafter"}),
            )
            .unwrap();

        assert_eq!(output, "before\n\\-\\-\\-\nafter");
    }

    #[test]
    fn frontmatter_safe_escapes_yaml_document_end_marker() {
        let renderer = Renderer::new();
        let output = renderer
            .render(
                "{{ value | frontmatter_safe }}",
                json!({"value": "before\n...\nafter"}),
            )
            .unwrap();

        assert_eq!(output, "before\n\\.\\.\\.\nafter");
    }

    #[test]
    fn frontmatter_safe_leaves_non_delimiter_text_byte_identical() {
        let renderer = Renderer::new();
        for value in ["ordinary text", "a---b", "a...b"] {
            let output = renderer
                .render("{{ value | frontmatter_safe }}", json!({"value": value}))
                .unwrap();
            assert_eq!(output, value);
        }
    }

    #[test]
    fn real_sprint_plan_template_neutralizes_injected_frontmatter_delimiters() {
        let title = "Injected frontmatter break\n---\nmalicious: true\n---";
        let rendered = Renderer::new()
            .render_named(
                "sprint-plan.md.j2",
                sprint_plan_body(),
                sprint_plan_context(title),
            )
            .unwrap();

        assert_eq!(standalone_frontmatter_delimiter_count(&rendered), 2);
        assert!(rendered.contains("malicious: true"));
        assert!(rendered.contains(r"\-\-\-"));
    }

    #[test]
    fn real_sprint_plan_template_neutralizes_injected_worktree_delimiters() {
        let mut context = sprint_plan_context("ordinary title");
        context["worktree"] = json!("injected worktree\n---\nmalicious: true\n---");
        let rendered = Renderer::new()
            .render_named("sprint-plan.md.j2", sprint_plan_body(), context)
            .unwrap();

        assert_eq!(standalone_frontmatter_delimiter_count(&rendered), 2);
        assert!(rendered.contains("malicious: true"));
        assert!(rendered.contains(r"\-\-\-"));
    }

    #[test]
    fn cdata_escape_round_trips_through_xml_parser() {
        let renderer = Renderer::new();
        let original = "before ]]> after";
        let output = renderer
            .render(
                "<root><![CDATA[{{ value | cdata_escape }}]]></root>",
                json!({"value": original}),
            )
            .unwrap();

        let mut reader = Reader::from_str(&output);
        reader.config_mut().trim_text(false);
        let mut content = String::new();
        loop {
            match reader.read_event().unwrap() {
                Event::CData(value) => content.push_str(str::from_utf8(value.as_ref()).unwrap()),
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(content, original);
    }

    #[test]
    fn cdata_escape_is_identity_without_cdata_terminator() {
        let renderer = Renderer::new();
        let output = renderer
            .render("{{ value | cdata_escape }}", json!({"value": "plain text"}))
            .unwrap();

        assert_eq!(output, "plain text");
    }

    #[test]
    fn real_plan_scope_template_round_trips_cdata_payload() {
        let source =
            include_str!("../../../.claude/skills/plan-hardening/01-plan-scope-review.xml.j2");
        let cdata_block = source
            .split("  <reviewer-findings-json>\n")
            .nth(1)
            .and_then(|tail| tail.split("  </reviewer-findings-json>").next())
            .expect("plan-hardening template must contain the reviewer CDATA block");
        let template =
            format!("<root><reviewer-findings-json>{cdata_block}</reviewer-findings-json></root>");
        let original = "finding before ]]> finding after";
        let rendered = Renderer::new()
            .render_named(
                "01-plan-scope-review.xml.j2",
                &template,
                json!({
                    "task_id": "FIX272-QA",
                    "phase": "fuzz-round-2",
                    "description": "CDATA regression",
                    "worktree_path": "/tmp/sc-compose",
                    "branch": "fix/272-format-aware-escaping",
                    "pr_target": "develop",
                    "source_of_truth": "docs/sprints/fix-272-format-aware-escaping.md",
                    "references": "crates/sc-composer/src/renderer.rs",
                    "round_id": "QA-272-002",
                    "round_index": 2,
                    "replay_nonce": "test-nonce",
                    "reviewer_findings_json": original,
                }),
            )
            .unwrap();

        let mut reader = Reader::from_str(&rendered);
        reader.config_mut().trim_text(false);
        let mut cdata_content = String::new();
        loop {
            match reader.read_event().unwrap() {
                Event::CData(value) => {
                    cdata_content.push_str(str::from_utf8(value.as_ref()).unwrap());
                }
                Event::Eof => break,
                _ => {}
            }
        }
        assert!(cdata_content.contains(original), "rendered XML: {rendered}");
    }

    #[test]
    fn turtle_escape_uses_turtle_string_literal_escapes() {
        let renderer = Renderer::new();
        let input = "\"\\\n\r\t";
        let expected = "\\\"\\\\\\n\\r\\t";
        assert_eq!(turtle_escape_filter(input), expected);
        let output = renderer
            .render("{{ value | turtle_escape }}", json!({"value": input}))
            .unwrap();

        assert_eq!(output, expected);
    }

    fn sprint_plan_body() -> &'static str {
        let source = include_str!("../../../.claude/skills/codex-orchestration/sprint-plan.md.j2");
        source
            .strip_prefix("---\n")
            .and_then(|source| source.split_once("\n---\n"))
            .map(|(_, body)| body)
            .expect("sprint-plan template must have declaration frontmatter")
    }

    fn sprint_plan_context(title: &str) -> serde_json::Value {
        json!({
            "id": "1.2",
            "title": title,
            "status": "planned",
            "branch": "main",
            "worktree": "/tmp/sc-compose",
            "target": "develop",
        })
    }

    fn generated_sprint_plan_frontmatter(rendered: &str) -> &str {
        let start = rendered
            .find("---\nid:")
            .expect("rendered sprint plan must contain generated frontmatter")
            + 4;
        let body = &rendered[start..];
        let end = body
            .find("\n---\n")
            .expect("generated sprint plan frontmatter must close");
        &body[..end]
    }

    #[test]
    fn yaml_safe_quotes_colon_space_and_round_trips_yaml() {
        let renderer = Renderer::new();
        let original = "ADR-001: with colon";
        let output = renderer
            .render("{{ value | yaml_safe }}", json!({"value": original}))
            .unwrap();

        assert_eq!(output, r#""ADR-001: with colon""#);
        let parsed: String = serde_yaml::from_str(&output).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn yaml_safe_escapes_backslash_before_quote() {
        let renderer = Renderer::new();
        let original = r#"backslash \ and "quote""#;
        let output = renderer
            .render("{{ value | yaml_safe }}", json!({"value": original}))
            .unwrap();

        assert_eq!(output, r#""backslash \\ and \"quote\"""#);
        let parsed: String = serde_yaml::from_str(&output).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn yaml_safe_uses_literal_control_character_escapes() {
        let renderer = Renderer::new();
        let original = "line\nnext\tcolumn\r";
        let output = renderer
            .render("{{ value | yaml_safe }}", json!({"value": original}))
            .unwrap();

        assert_eq!(output, r#""line\nnext\tcolumn\r""#);
        assert!(!output[1..output.len() - 1].contains('\n'));
        let parsed: String = serde_yaml::from_str(&output).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn yaml_safe_round_trips_ordinary_text_unchanged() {
        let renderer = Renderer::new();
        let original = "ordinary text with punctuation";
        let output = renderer
            .render("{{ value | yaml_safe }}", json!({"value": original}))
            .unwrap();

        let parsed: String = serde_yaml::from_str(&output).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn frontmatter_safe_then_yaml_safe_handles_both_injection_shapes() {
        let renderer = Renderer::new();
        let original = "before: value\n---\nmalicious: true";
        let output = renderer
            .render(
                "{{ value | frontmatter_safe | yaml_safe }}",
                json!({"value": original}),
            )
            .unwrap();

        let parsed: String = serde_yaml::from_str(&output).unwrap();
        assert_eq!(parsed, "before: value\n\\-\\-\\-\nmalicious: true");
    }

    #[test]
    fn real_sprint_plan_template_round_trips_yaml_frontmatter() {
        let title = "Architecture: plan";
        let rendered = Renderer::new()
            .render_named(
                "sprint-plan.md.j2",
                sprint_plan_body(),
                sprint_plan_context(title),
            )
            .unwrap();

        let frontmatter: serde_yaml::Value =
            serde_yaml::from_str(generated_sprint_plan_frontmatter(&rendered)).unwrap();
        assert_eq!(frontmatter["title"], title);
        assert_eq!(frontmatter["worktree"], "/tmp/sc-compose");
    }

    #[test]
    fn renderer_supports_issue_270_dict_get_with_default() {
        let renderer = Renderer::new();
        let output = renderer
            .render(r#"{{ row.get("k", "n/a") }}"#, json!({"row": {"k": "v"}}))
            .unwrap();

        assert_eq!(output, "v");
    }

    #[test]
    fn renderer_supports_dict_get_without_default_for_present_key() {
        let renderer = Renderer::new();
        let output = renderer
            .render(r#"{{ row.get("k") }}"#, json!({"row": {"k": "v"}}))
            .unwrap();

        assert_eq!(output, "v");
    }

    #[test]
    fn renderer_returns_empty_for_missing_dict_get_without_default() {
        let renderer = Renderer::new();
        let output = renderer
            .render(r#"{{ row.get("missing") }}"#, json!({"row": {"k": "v"}}))
            .unwrap();

        assert_eq!(output, "");
    }

    #[test]
    fn renderer_returns_default_for_missing_dict_get() {
        let renderer = Renderer::new();
        let output = renderer
            .render(
                r#"{{ row.get("missing", "n/a") }}"#,
                json!({"row": {"k": "v"}}),
            )
            .unwrap();

        assert_eq!(output, "n/a");
    }

    #[test]
    fn renderer_rejects_get_on_non_map_value() {
        let renderer = Renderer::new();
        let error = renderer
            .render(r#"{{ row.get("k", "n/a") }}"#, json!({"row": "text"}))
            .unwrap_err();
        let detail = error.source().map(ToString::to_string).unwrap_or_default();

        assert!(
            detail.contains("unknown method") || detail.contains("has no method named get"),
            "expected unknown-method error, got: {detail}"
        );
    }

    #[test]
    fn renderer_rejects_unrecognized_map_method() {
        let renderer = Renderer::new();
        let error = renderer
            .render("{{ row.items() }}", json!({"row": {"k": "v"}}))
            .unwrap_err();
        let detail = error.source().map(ToString::to_string).unwrap_or_default();

        assert!(
            detail.contains("unknown method") || detail.contains("has no method named items"),
            "expected unknown-method error, got: {detail}"
        );
    }

    #[test]
    fn renderer_rejects_out_of_range_dict_get_arities() {
        let renderer = Renderer::new();

        for template in ["{{ row.get() }}", r#"{{ row.get("k", "v", "extra") }}"#] {
            let error = renderer
                .render(template, json!({"row": {"k": "v"}}))
                .unwrap_err();
            let detail = error.source().map(ToString::to_string).unwrap_or_default();

            assert!(
                detail.contains("unknown method") || detail.contains("has no method named get"),
                "expected unknown-method error for {template}, got: {detail}"
            );
        }
    }

    #[test]
    fn renderer_renders_atm_core_smoke_report_deviation_row() {
        let renderer = Renderer::new();
        let template = r#"{% set deviations = report.rows | selectattr("verdict", "ne", "PASS") | list -%}{% for row in deviations -%}{{ row.get("observed_behavior", "n/a") }}|{{ row.get("expected_behavior", "n/a") }}|{{ row.get("likely_root_cause", "n/a") }}|{{ row.get("artifact_pointer", "n/a") }}{% endfor -%}"#;
        let output = renderer
            .render(
                template,
                json!({
                    "report": {
                        "rows": [{
                            "id": "FIX270-NONPASS",
                            "verdict": "FAIL",
                            "observed_behavior": "rendered a deviation report"
                        }]
                    }
                }),
            )
            .unwrap();

        assert_eq!(output, "rendered a deviation report|n/a|n/a|n/a");
    }

    #[test]
    fn render_loaded_template_rejects_malformed_supporting_templates() {
        let error = render_loaded_template(LoadedTemplateRequest {
            template_name: "report.html.j2".to_owned(),
            template_text: "{{ value }}".to_owned(),
            context: json!({ "value": "ok" })
                .as_object()
                .unwrap()
                .clone()
                .into_iter()
                .collect(),
            supporting_templates: vec![NamedTemplateAsset {
                template_name: "shared/base.html.j2".to_owned(),
                template_text: "{% if broken %}".to_owned(),
            }],
        })
        .unwrap_err();

        let source = error.source().map(ToString::to_string).unwrap_or_default();
        assert!(
            source.contains("unexpected end of input"),
            "expected supporting-template parse failure source, got: {source}"
        );
    }

    #[test]
    fn with_delimiters_rejects_invalid_syntax_with_typed_error() {
        let error = Renderer::with_delimiters("", "}}").unwrap_err();

        assert_eq!(error.code(), None);
        assert_eq!(error.to_string(), "template rendering failed");
    }
}
