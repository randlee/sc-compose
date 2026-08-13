use std::collections::BTreeMap;
use std::path::Path;

use anyhow::anyhow;
use sc_composer::types::default_pass_number;
use sc_composer::{
    DiagnosticCode, FrontmatterInitResult, RecoveryHint, RecoveryHintKind, VariableName,
    is_json_template_path, parse_template_document,
};

use crate::cli::{FrontmatterInitArgs, TemplateInitArgs, parse_pass_inputs};
use crate::path_utils::to_forward_slash;
use crate::{CommandError, print_json};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct InitPass {
    pass_number: u8,
    variables: Vec<(VariableName, String)>,
}

#[derive(Clone, Debug)]
struct PlannedReplacement {
    pass_number: u8,
    variable_name: VariableName,
    value: String,
    brace_count: usize,
}

impl PlannedReplacement {
    fn render_token(&self) -> String {
        format!(
            "{} {} {}",
            "{".repeat(self.brace_count),
            self.variable_name.as_str(),
            "}".repeat(self.brace_count)
        )
    }
}

#[derive(serde::Serialize)]
struct MultiPassHeader {
    pass: u8,
    required_variables: Vec<String>,
    defaults: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_escape_mode: Option<&'static str>,
}

pub(crate) fn run_frontmatter_init(args: &FrontmatterInitArgs) -> Result<i32, CommandError> {
    let result = sc_composer::frontmatter_init(&args.file, args.force, args.dry_run)
        .map_err(CommandError::compose)?;
    if args.json && args.dry_run {
        print_json(
            serde_json::json!({
                "action": "frontmatter-init",
                "would_affect": [to_forward_slash(&result.target_path)],
                "changed": result.changed,
                "would_change": result.would_change,
                "skipped": !result.would_change,
                "vars": result.discovered_variables,
            }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else if args.json {
        print_json_frontmatter_init(&result).map_err(CommandError::usage)?;
    } else if args.dry_run {
        println!("{}", result.frontmatter_text);
    }
    Ok(crate::exit_codes::SUCCESS)
}

pub(crate) fn run_template_init(args: &TemplateInitArgs) -> Result<i32, CommandError> {
    let pass_inputs = parse_pass_inputs(std::env::args_os(), "template-init")
        .map_err(template_init_parse_error)?;
    if pass_inputs.is_empty() {
        return Err(CommandError::usage_with_code_and_hints(
            anyhow!("template-init requires at least one --pass N group"),
            DiagnosticCode::ErrConfigParse,
            vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "pass one or more --pass N groups followed by --var key=value replacements"
                    .to_owned(),
            })],
        ));
    }

    let init_passes = pass_inputs
        .iter()
        .map(|pass| {
            let variables = pass
                .vars
                .iter()
                .map(|(name, value)| {
                    VariableName::new(name.clone())
                        .map(|validated| (validated, value.clone()))
                        .map_err(|error| {
                            CommandError::usage_with_code_and_hints(
                                anyhow!("invalid `--var` name `{name}`: {error}"),
                                DiagnosticCode::ErrConfigParse,
                                vec![RecoveryHint::new(
                                    RecoveryHintKind::ReviewConfiguration {
                                        key: "use variable names containing only ASCII letters, digits, ., _, or -"
                                            .to_owned(),
                                    },
                                )],
                            )
                        })
                })
                .collect::<Result<Vec<_>, CommandError>>()?;
            Ok(InitPass {
                pass_number: normalize_pass_number(pass.pass_number),
                variables,
            })
        })
        .collect::<Result<Vec<_>, CommandError>>()?;

    let result = template_init_file(&args.file, &init_passes, args.force, args.dry_run)?;
    if args.json && args.dry_run {
        print_json(
            serde_json::json!({
                "action": "template-init",
                "would_affect": [to_forward_slash(&result.target_path)],
                "changed": result.changed,
                "would_change": result.would_change,
                "skipped": !result.would_change,
                "vars": result.discovered_variables,
            }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else if args.json {
        print_json(
            serde_json::json!({
                "template_path": to_forward_slash(&result.target_path),
                "template_added": result.changed,
                "would_change": result.would_change,
                "vars": result.discovered_variables,
            }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else if args.dry_run {
        println!("{}", result.template_text);
    }

    Ok(crate::exit_codes::SUCCESS)
}

fn print_json_frontmatter_init(result: &sc_composer::FrontmatterInitResult) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "template_path": to_forward_slash(&result.target_path),
        "frontmatter_added": result.changed,
        "would_change": result.would_change,
        "vars": result.discovered_variables,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&crate::json_output::envelope(payload, Vec::new()))?
    );
    Ok(())
}

fn template_init_parse_error(error: String) -> CommandError {
    CommandError::usage_with_code_and_hints(
        anyhow!(error),
        DiagnosticCode::ErrConfigParse,
        vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "declare each --pass N group before its --var replacements".to_owned(),
        })],
    )
}

fn normalize_pass_number(pass_number: u8) -> u8 {
    if pass_number == 0 {
        default_pass_number()
    } else {
        pass_number
    }
}

fn template_init_file(
    path: impl AsRef<Path>,
    passes: &[InitPass],
    force: bool,
    dry_run: bool,
) -> Result<FrontmatterInitResult, CommandError> {
    let canonical = std::fs::canonicalize(path.as_ref()).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!(error).context(format!(
                "failed to canonicalize template-init target: {}",
                path.as_ref().display()
            )),
            DiagnosticCode::ErrConfigParse,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: path.as_ref().to_path_buf(),
            })],
        )
    })?;
    let contents = std::fs::read_to_string(&canonical).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!(error).context(format!("failed to read template: {}", canonical.display())),
            DiagnosticCode::ErrConfigParse,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: canonical.clone(),
            })],
        )
    })?;
    let parsed = parse_template_document(&contents).map_err(CommandError::compose)?;
    if parsed.frontmatter().is_some() && !force {
        return Err(CommandError::usage_with_code(
            anyhow!("frontmatter already exists; rerun with --force to rewrite it"),
            DiagnosticCode::ErrConfigReadonly,
        ));
    }
    if passes.is_empty() {
        return Err(CommandError::usage_with_code(
            anyhow!("template-init requires at least one --pass N group"),
            DiagnosticCode::ErrConfigParse,
        ));
    }

    let replacements = plan_replacements(passes);
    let rewritten_body =
        apply_replacements(&contents, &replacements, is_json_template_path(&canonical))?;
    let frontmatter_text = build_stacked_frontmatter(passes, is_json_template_path(&canonical))?;
    let template_text = format!(
        "{frontmatter_text}{}",
        rewritten_body.trim_start_matches(['\n', '\r'])
    );
    let would_change = template_text != contents;

    if !dry_run && would_change {
        std::fs::write(&canonical, &template_text).map_err(|error| {
            CommandError::usage_with_code_and_hints(
                anyhow!(error)
                    .context(format!("failed to write template: {}", canonical.display())),
                DiagnosticCode::ErrConfigReadonly,
                vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                    path: canonical.clone(),
                })],
            )
        })?;
    }

    Ok(FrontmatterInitResult {
        target_path: canonical,
        frontmatter_text,
        template_text,
        discovered_variables: replacements
            .iter()
            .map(|replacement| replacement.variable_name.clone())
            .collect(),
        changed: !dry_run && would_change,
        would_change,
    })
}

fn plan_replacements(passes: &[InitPass]) -> Vec<PlannedReplacement> {
    let mut replacements = Vec::new();
    for init_pass in passes {
        let brace_count = usize::from(init_pass.pass_number) + 1;
        for (name, value) in &init_pass.variables {
            replacements.push(PlannedReplacement {
                pass_number: init_pass.pass_number,
                variable_name: name.clone(),
                value: value.clone(),
                brace_count,
            });
        }
    }
    replacements.sort_by(|left, right| {
        right
            .value
            .len()
            .cmp(&left.value.len())
            .then_with(|| right.pass_number.cmp(&left.pass_number))
    });
    replacements
}

fn apply_replacements(
    original: &str,
    replacements: &[PlannedReplacement],
    consume_json_string_quotes: bool,
) -> Result<String, CommandError> {
    let mut missing = Vec::new();
    for replacement in replacements {
        if !original.contains(&replacement.value) {
            missing.push(format!(
                "{}={}",
                replacement.variable_name.as_str(),
                replacement.value
            ));
        }
    }
    if !missing.is_empty() {
        return Err(CommandError::usage_with_code(
            anyhow!(
                "values not found in file: {}. Check for typos or differences in whitespace/encoding.",
                missing.join(", ")
            ),
            DiagnosticCode::ErrConfigParse,
        ));
    }

    let mut occupied = Vec::<(usize, usize)>::new();
    let mut planned = Vec::<(usize, usize, String)>::new();
    let mut unavailable = Vec::new();

    for replacement in replacements {
        let spans = find_available_spans(
            original,
            &replacement.value,
            &occupied,
            consume_json_string_quotes,
        );
        if spans.is_empty() {
            unavailable.push(format!(
                "{}={}",
                replacement.variable_name.as_str(),
                replacement.value
            ));
            continue;
        }
        for (start, end) in spans {
            occupied.push((start, end));
            planned.push((start, end, replacement.render_token()));
        }
    }

    if !unavailable.is_empty() {
        return Err(CommandError::usage_with_code(
            anyhow!(
                "values could not be substituted without overlap: {}. Check for typos, duplicate literal assignments, or overlapping replacements.",
                unavailable.join(", ")
            ),
            DiagnosticCode::ErrConfigParse,
        ));
    }

    planned.sort_by_key(|(start, _, _)| *start);
    let mut rewritten = String::with_capacity(original.len());
    let mut cursor = 0;
    for (start, end, token) in planned {
        rewritten.push_str(&original[cursor..start]);
        rewritten.push_str(&token);
        cursor = end;
    }
    rewritten.push_str(&original[cursor..]);
    Ok(rewritten)
}

fn find_available_spans(
    haystack: &str,
    needle: &str,
    occupied: &[(usize, usize)],
    consume_json_string_quotes: bool,
) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut search_from = 0;
    while let Some(relative_start) = haystack[search_from..].find(needle) {
        let value_start = search_from + relative_start;
        let value_end = value_start + needle.len();
        let consumes_quotes = consume_json_string_quotes
            && value_start > 0
            && value_end < haystack.len()
            && haystack.as_bytes()[value_start - 1] == b'"'
            && haystack.as_bytes()[value_end] == b'"';
        let start = if consumes_quotes {
            value_start - 1
        } else {
            value_start
        };
        let end = if consumes_quotes {
            value_end + 1
        } else {
            value_end
        };
        if !occupied
            .iter()
            .any(|(taken_start, taken_end)| start < *taken_end && end > *taken_start)
        {
            spans.push((start, end));
        }
        search_from = value_end;
    }
    spans
}

fn build_stacked_frontmatter(
    passes: &[InitPass],
    is_json_template: bool,
) -> Result<String, CommandError> {
    let single_pass_compat = passes.len() == 1 && passes[0].pass_number == default_pass_number();
    if single_pass_compat {
        let mut text = String::from("---\nrequired_variables:\n");
        for (name, _) in &passes[0].variables {
            text.push_str("  - ");
            text.push_str(name.as_str());
            text.push('\n');
        }
        if is_json_template {
            text.push_str("json_escape_mode: auto\n");
        }
        text.push_str("defaults: {}\nmetadata: {}\n---\n");
        return Ok(text);
    }

    let mut text = String::new();
    for init_pass in passes {
        let header = MultiPassHeader {
            pass: init_pass.pass_number,
            required_variables: init_pass
                .variables
                .iter()
                .map(|(name, _)| name.as_str().to_owned())
                .collect(),
            defaults: init_pass
                .variables
                .iter()
                .map(|(name, value)| (name.as_str().to_owned(), value.clone()))
                .collect(),
            json_escape_mode: is_json_template.then_some("auto"),
        };
        let yaml = serialize_header(&header)?;
        text.push_str("---\n");
        text.push_str(&yaml);
        text.push_str("---\n");
    }
    Ok(text)
}

fn serialize_header(value: &impl serde::Serialize) -> Result<String, CommandError> {
    serde_yaml::to_string(value).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context("failed to serialize template-init header"),
            DiagnosticCode::ErrConfigParse,
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{InitPass, normalize_pass_number, serialize_header, template_init_file};
    use sc_composer::{Renderer, VariableName, parse_template_document};
    use serde::ser::{Error as _, Serializer};

    #[test]
    fn template_init_builds_multi_pass_template() {
        let root = temp_root("template_init_multi_pass");
        let template = root.join("agent.md");
        write_file(&template, "deploy test for wyvern");

        let result = template_init_file(
            &template,
            &[
                InitPass {
                    pass_number: 2,
                    variables: vec![(VariableName::new("team").unwrap(), "wyvern".to_owned())],
                },
                InitPass {
                    pass_number: 1,
                    variables: vec![(VariableName::new("task").unwrap(), "test".to_owned())],
                },
            ],
            false,
            true,
        )
        .unwrap();

        assert!(result.template_text.contains("pass: 2"));
        assert!(result.template_text.contains("pass: 1"));
        assert!(result.template_text.contains("{{{ team }}}"));
        assert!(result.template_text.contains("{{ task }}"));
    }

    #[test]
    fn template_init_single_pass_matches_legacy_frontmatter_shape() {
        let root = temp_root("template_init_single_pass");
        let template = root.join("agent.md");
        write_file(&template, "deploy test");

        let result = template_init_file(
            &template,
            &[InitPass {
                pass_number: 1,
                variables: vec![(VariableName::new("task").unwrap(), "test".to_owned())],
            }],
            false,
            true,
        )
        .unwrap();

        assert_eq!(
            result.frontmatter_text,
            "---\nrequired_variables:\n  - task\ndefaults: {}\nmetadata: {}\n---\n"
        );
        assert!(result.template_text.contains("{{ task }}"));
        assert!(!result.template_text.contains("pass: 1"));
    }

    #[test]
    fn template_init_json_round_trips_string_values_through_render() {
        let root = temp_root("template_init_json_round_trip");
        let template = root.join("payload.json");
        let original = "{\n  \"worktree_path\": \"/tmp/wt\",\n  \"enabled\": true\n}";
        write_file(&template, original);

        let result = template_init_file(
            &template,
            &[InitPass {
                pass_number: 1,
                variables: vec![(
                    VariableName::new("worktree_path").unwrap(),
                    "/tmp/wt".to_owned(),
                )],
            }],
            false,
            true,
        )
        .unwrap();

        assert!(result.template_text.contains("json_escape_mode: auto"));
        let parsed = parse_template_document(&result.template_text).unwrap();
        assert!(
            parsed
                .body()
                .contains("\"worktree_path\": {{ worktree_path }}")
        );

        let rendered = Renderer::new()
            .render_named(
                "payload.json.j2",
                parsed.body(),
                serde_json::json!({"worktree_path": "/tmp/wt"}),
            )
            .unwrap();
        assert_eq!(rendered, original);
    }

    #[test]
    fn fuzz_001_template_init_uses_shared_json_path_detector() {
        let root = temp_root("template_init_json_path_variants");
        for (fixture, file_name) in [
            ("uppercase-content", "payload.JSON.j2"),
            ("uppercase-template", "payload.json.J2"),
            ("stacked-suffix", "payload.json.j2.j2"),
        ] {
            let template = root.join(fixture).join(file_name);
            write_file(&template, "{\"worktree_path\": \"/tmp/wt\"}\n");

            let result = template_init_file(
                &template,
                &[InitPass {
                    pass_number: 1,
                    variables: vec![(
                        VariableName::new("worktree_path").unwrap(),
                        "/tmp/wt".to_owned(),
                    )],
                }],
                false,
                true,
            )
            .unwrap_or_else(|error| panic!("template-init failed for {file_name}: {error}"));

            assert!(
                result.template_text.contains("json_escape_mode: auto"),
                "template-init misclassified {file_name}: {}",
                result.template_text
            );
            assert!(
                result
                    .template_text
                    .contains("\"worktree_path\": {{ worktree_path }}"),
                "template-init retained legacy quoting for {file_name}: {}",
                result.template_text
            );
        }
    }

    #[test]
    fn template_init_replaces_longest_value_first() {
        let root = temp_root("template_init_longest_first");
        let template = root.join("agent.md");
        write_file(&template, "/home/wyvern/worktrees/wyvern owned by wyvern");

        let result = template_init_file(
            &template,
            &[
                InitPass {
                    pass_number: 2,
                    variables: vec![(
                        VariableName::new("worktree_path").unwrap(),
                        "/home/wyvern/worktrees/wyvern".to_owned(),
                    )],
                },
                InitPass {
                    pass_number: 1,
                    variables: vec![(VariableName::new("team").unwrap(), "wyvern".to_owned())],
                },
            ],
            false,
            true,
        )
        .unwrap();

        assert!(
            result
                .template_text
                .contains("{{{ worktree_path }}} owned by {{ team }}")
        );
        assert!(
            !result
                .template_text
                .contains("{{ team }}/worktrees/{{ team }}")
        );
    }

    #[test]
    fn template_init_rejects_duplicate_literal_assignments() {
        let root = temp_root("template_init_duplicate_literal");
        let template = root.join("agent.md");
        write_file(&template, "alpha alpha");

        let error = template_init_file(
            &template,
            &[
                InitPass {
                    pass_number: 1,
                    variables: vec![(VariableName::new("first").unwrap(), "alpha".to_owned())],
                },
                InitPass {
                    pass_number: 1,
                    variables: vec![(VariableName::new("second").unwrap(), "alpha".to_owned())],
                },
            ],
            false,
            true,
        )
        .unwrap_err();

        assert!(format!("{error:#}").contains("could not be substituted without overlap"));
        assert!(format!("{error:#}").contains("second=alpha"));
    }

    #[test]
    fn template_init_does_not_rewrite_inside_inserted_tokens() {
        let root = temp_root("template_init_token_overlap");
        let template = root.join("agent.md");
        write_file(&template, "team me");

        let result = template_init_file(
            &template,
            &[
                InitPass {
                    pass_number: 2,
                    variables: vec![(VariableName::new("team_name").unwrap(), "team".to_owned())],
                },
                InitPass {
                    pass_number: 1,
                    variables: vec![(VariableName::new("suffix").unwrap(), "me".to_owned())],
                },
            ],
            false,
            true,
        )
        .unwrap();

        assert!(
            result
                .template_text
                .contains("{{{ team_name }}} {{ suffix }}")
        );
        assert!(
            !result
                .template_text
                .contains("{{{ tea{{ suffix }}_name }}}")
        );
    }

    #[test]
    fn template_init_reports_values_not_found() {
        let root = temp_root("template_init_missing_value");
        let template = root.join("agent.md");
        write_file(&template, "deploy test");

        let error = template_init_file(
            &template,
            &[InitPass {
                pass_number: 1,
                variables: vec![(VariableName::new("team").unwrap(), "wyvern".to_owned())],
            }],
            false,
            true,
        )
        .unwrap_err();

        assert!(format!("{error:#}").contains("values not found in file"));
        assert!(format!("{error:#}").contains("team=wyvern"));
    }

    #[test]
    fn init_pass_zero_normalizes_to_default_pass_number() {
        assert_eq!(normalize_pass_number(0), 1);
    }

    #[test]
    fn build_stacked_frontmatter_serialization_failure_is_structured() {
        struct FailingSerialize;

        impl serde::Serialize for FailingSerialize {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                Err(S::Error::custom("boom"))
            }
        }

        let error = serialize_header(&FailingSerialize).unwrap_err();
        assert!(format!("{error:#}").contains("failed to serialize template-init header"));
        assert!(format!("{error:#}").contains("boom"));
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}
