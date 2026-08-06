use std::collections::BTreeSet;
use std::path::Path;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;

use crate::ExpandedTemplate;
use crate::discovery::{discover_all_pass_tokens, discover_tokens};
use crate::frontmatter::{Frontmatter, ParsedTemplate};
use crate::types::{ComposeRequest, VariableName, VariableSource};

use super::ValidationState;

const RENDER_DATE_FORMAT: &[time::format_description::FormatItem<'static>] =
    format_description!("[year]-[month]-[day]");

pub(crate) fn collect_validation_state(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
) -> ValidationState {
    let mut state = ValidationState::default();
    state.source_texts = expanded.source_texts.clone();
    if state.source_texts.is_empty()
        && let Some(root) = expanded.resolved_files.first()
    {
        state
            .source_texts
            .insert(root.clone(), expanded.text.clone());
    }

    for (path, frontmatters) in &expanded.frontmatters {
        if !frontmatters.is_empty() {
            let is_root = expanded
                .resolved_files
                .first()
                .is_some_and(|root| root == path);
            for frontmatter in frontmatters {
                merge_frontmatter(path, frontmatter, expanded, &mut state, is_root);
            }
        }
    }

    for (name, value) in &request.vars_defaults {
        state.context.insert(name.clone(), value.clone());
        state.default_origins.insert(name.clone(), None);
        state
            .variable_sources
            .insert(name.clone(), VariableSource::TemplateInputDefault);
    }
    for (name, value) in &request.vars_env {
        state.context.insert(name.clone(), value.clone());
        state
            .variable_sources
            .insert(name.clone(), VariableSource::Environment);
    }
    for (name, value) in &request.vars_input {
        state.context.insert(name.clone(), value.clone());
        state
            .variable_sources
            .insert(name.clone(), VariableSource::ExplicitInput);
    }

    declare_builtin_variables(&mut state);
    populate_pass_validation_maps(&mut state, expanded);
    state
}

fn populate_pass_validation_maps(state: &mut ValidationState, expanded: &ExpandedTemplate) {
    let root_path = expanded.resolved_files.first();
    let root_passes = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| {
            root_path
                .is_some_and(|root| path == root)
                .then(|| passes.clone())
        })
        .unwrap_or_default();
    if root_passes.is_empty() {
        state.referenced_variables = discover_tokens(&expanded.text);
        return;
    }

    let parsed = ParsedTemplate::from_parts(root_passes, expanded.text.clone());

    let referenced_variables_by_pass = discover_all_pass_tokens(&parsed);
    let root_pass_declared = parsed
        .passes()
        .iter()
        .flat_map(|pass| {
            pass.required_variables()
                .iter()
                .chain(pass.defaults().keys())
                .cloned()
        })
        .collect::<BTreeSet<_>>();
    let shared_declared = state
        .declared_variables
        .difference(&root_pass_declared)
        .cloned()
        .collect::<BTreeSet<_>>();

    state.referenced_variables = referenced_variables_by_pass
        .values()
        .flatten()
        .cloned()
        .collect();
    state.referenced_variables_by_pass = referenced_variables_by_pass;
    state.declared_variables_by_pass = parsed
        .passes()
        .iter()
        .map(|pass| {
            let declared = shared_declared
                .iter()
                .cloned()
                .chain(pass.required_variables().iter().cloned())
                .chain(pass.defaults().keys().cloned())
                .collect::<BTreeSet<_>>();
            (usize::from(pass.pass_number()), declared)
        })
        .collect();

    for pass in parsed.passes() {
        let pass_number = usize::from(pass.pass_number());
        for variable in pass.defaults().keys() {
            state
                .default_pass_numbers
                .entry(variable.clone())
                .or_default()
                .insert(pass_number);
        }
    }
}

pub(crate) fn inject_builtin_vars(state: &mut ValidationState, template_path: &Path) {
    let now = OffsetDateTime::now_utc();
    insert_builtin_var(
        state,
        super::BUILTIN_VARIABLE_NAMES[0],
        template_path.file_name().map_or_else(
            || "unknown".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        ),
    );
    insert_builtin_var(state, super::BUILTIN_VARIABLE_NAMES[1], current_hostname());
    insert_builtin_var(state, super::BUILTIN_VARIABLE_NAMES[2], current_username());
    insert_builtin_var(
        state,
        super::BUILTIN_VARIABLE_NAMES[3],
        format_render_date(now),
    );
    insert_builtin_var(
        state,
        super::BUILTIN_VARIABLE_NAMES[4],
        format_render_timestamp(now),
    );
}

fn declare_builtin_variables(state: &mut ValidationState) {
    for raw_name in super::BUILTIN_VARIABLE_NAMES {
        if let Ok(name) = VariableName::new(raw_name) {
            state.declared_variables.insert(name);
        }
    }
}

fn insert_builtin_var(state: &mut ValidationState, raw_name: &'static str, value: String) {
    let Ok(name) = VariableName::new(raw_name) else {
        return;
    };
    let preserve_caller_value = matches!(
        state.variable_sources.get(&name),
        Some(VariableSource::Environment | VariableSource::ExplicitInput)
    );
    if !preserve_caller_value {
        state
            .context
            .insert(name.clone(), serde_json::Value::String(value));
        state
            .variable_sources
            .insert(name.clone(), VariableSource::Builtin);
    }
    state.declared_variables.insert(name);
}

fn format_render_date(now: OffsetDateTime) -> String {
    now.format(RENDER_DATE_FORMAT)
        .unwrap_or_else(|_| render_date_fallback(now))
}

fn format_render_timestamp(now: OffsetDateTime) -> String {
    now.format(&Rfc3339)
        .unwrap_or_else(|_| render_timestamp_fallback(now))
}

fn render_date_fallback(now: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        u8::from(now.month()),
        now.day()
    )
}

fn render_timestamp_fallback(now: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn current_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn current_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn merge_frontmatter(
    path: &Path,
    frontmatter: &Frontmatter,
    expanded: &ExpandedTemplate,
    state: &mut ValidationState,
    is_root: bool,
) {
    for variable in frontmatter.required_variables() {
        state
            .required_origins
            .entry(variable.clone())
            .or_insert_with(|| path.to_path_buf());
        state
            .required_include_chains
            .entry(variable.clone())
            .or_insert_with(|| {
                expanded
                    .include_chains
                    .get(path)
                    .cloned()
                    .unwrap_or_default()
            });
        state.declared_variables.insert(variable.clone());
    }

    for (variable, value) in frontmatter.defaults() {
        state.declared_variables.insert(variable.clone());
        state
            .default_origins
            .entry(variable.clone())
            .or_insert_with(|| Some(path.to_path_buf()));
        state
            .context
            .entry(variable.clone())
            .or_insert_with(|| value.clone());
        state
            .variable_sources
            .entry(variable.clone())
            .or_insert(if is_root {
                VariableSource::FrontmatterDefault
            } else {
                VariableSource::IncludedDefault
            });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use serde_json::json;

    use super::{collect_validation_state, inject_builtin_vars};
    use crate::ExpandedTemplate;
    use crate::frontmatter::parse_template_document;
    use crate::types::{
        ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, VariableName, VariableSource,
    };

    #[test]
    fn collect_validation_state_characterizes_i5_loop_context_maps() {
        let root = PathBuf::from("/workspace/root.md.j2");
        let document = "---\npass: 1\nrequired_variables:\n  - items\n  - task\n---\n---\npass: 2\nrequired_variables:\n  - higher\n---\n{% for item in items %}{{ loop.index }}:{{ item }}{% endfor %} {{ task }} {{{ higher }}}";
        let parsed = parse_template_document(document).unwrap();
        let expanded = ExpandedTemplate {
            text: parsed.body().to_owned(),
            resolved_files: vec![root.clone()],
            frontmatters: vec![(root, parsed.passes().to_vec())],
            include_chains: BTreeMap::new(),
            source_texts: BTreeMap::new(),
        };

        let state = collect_validation_state(&empty_request(), &expanded);
        let pass_one_references = &state.referenced_variables_by_pass[&1];
        let pass_two_references = &state.referenced_variables_by_pass[&2];

        assert!(pass_one_references.contains(&variable("items")));
        assert!(pass_one_references.contains(&variable("task")));
        assert!(!pass_one_references.contains(&variable("item")));
        assert!(!pass_one_references.contains(&variable("loop.index")));
        assert!(pass_two_references.contains(&variable("items")));
        assert!(pass_two_references.contains(&variable("higher")));
        assert!(!pass_two_references.contains(&variable("task")));
        assert!(!pass_two_references.contains(&variable("loop.index")));

        assert_eq!(
            state.declared_variables_by_pass[&1],
            expected_declared(&["items", "task"])
        );
        assert_eq!(
            state.declared_variables_by_pass[&2],
            expected_declared(&["higher"])
        );
    }

    #[test]
    fn collect_validation_state_characterizes_default_merge_precedence() {
        let root = PathBuf::from("/workspace/root.md.j2");
        let parsed = parse_template_document(
            "---\ndefaults:\n  frontmatter_only: frontmatter\n  fallback: frontmatter\n  env_value: frontmatter\n  explicit_value: frontmatter\n  HOSTNAME: frontmatter\n  RENDER_DATE: frontmatter-date\n---\n{{ fallback }}",
        )
        .unwrap();
        let expanded = ExpandedTemplate {
            text: parsed.body().to_owned(),
            resolved_files: vec![root.clone()],
            frontmatters: vec![(root.clone(), parsed.passes().to_vec())],
            include_chains: BTreeMap::new(),
            source_texts: BTreeMap::new(),
        };
        let mut request = empty_request();
        request
            .vars_defaults
            .insert(variable("fallback"), json!("input-default"));
        request
            .vars_defaults
            .insert(variable("env_value"), json!("input-default"));
        request
            .vars_defaults
            .insert(variable("explicit_value"), json!("input-default"));
        request
            .vars_defaults
            .insert(variable("HOSTNAME"), json!("input-default"));
        request
            .vars_defaults
            .insert(variable("RENDER_DATE"), json!("input-default"));
        request
            .vars_env
            .insert(variable("env_value"), json!("environment"));
        request
            .vars_env
            .insert(variable("explicit_value"), json!("environment"));
        request
            .vars_input
            .insert(variable("explicit_value"), json!("explicit"));

        let mut state = collect_validation_state(&request, &expanded);
        inject_builtin_vars(&mut state, &root);

        assert_eq!(
            state.context[&variable("frontmatter_only")],
            json!("frontmatter")
        );
        assert_eq!(
            state.variable_sources[&variable("frontmatter_only")],
            VariableSource::FrontmatterDefault
        );
        assert_eq!(state.context[&variable("fallback")], json!("input-default"));
        assert_eq!(
            state.variable_sources[&variable("fallback")],
            VariableSource::TemplateInputDefault
        );
        assert_eq!(state.context[&variable("env_value")], json!("environment"));
        assert_eq!(
            state.variable_sources[&variable("env_value")],
            VariableSource::Environment
        );
        assert_eq!(
            state.context[&variable("explicit_value")],
            json!("explicit")
        );
        assert_eq!(
            state.variable_sources[&variable("explicit_value")],
            VariableSource::ExplicitInput
        );
        assert_eq!(
            state.variable_sources[&variable("HOSTNAME")],
            VariableSource::Builtin
        );
        assert_eq!(
            state.variable_sources[&variable("RENDER_DATE")],
            VariableSource::Builtin
        );
    }

    #[test]
    fn collect_validation_state_characterizes_required_origins_and_include_chains() {
        let root = PathBuf::from("/workspace/root.md.j2");
        let child = PathBuf::from("/workspace/child.md");
        let root_frontmatter =
            parse_template_document("---\nrequired_variables:\n  - shared\n---\nroot body")
                .unwrap();
        let child_frontmatter = parse_template_document(
            "---\nrequired_variables:\n  - shared\n  - child_only\n---\nchild body",
        )
        .unwrap();
        let mut include_chains = BTreeMap::new();
        include_chains.insert(root.clone(), vec![root.clone()]);
        include_chains.insert(child.clone(), vec![root.clone(), child.clone()]);
        let expanded = ExpandedTemplate {
            text: "root body\nchild body".to_owned(),
            resolved_files: vec![root.clone(), child.clone()],
            frontmatters: vec![
                (root.clone(), root_frontmatter.passes().to_vec()),
                (child.clone(), child_frontmatter.passes().to_vec()),
            ],
            include_chains,
            source_texts: BTreeMap::new(),
        };

        let state = collect_validation_state(&empty_request(), &expanded);
        let shared = variable("shared");
        let child_only = variable("child_only");

        assert_eq!(state.required_origins[&shared], root);
        assert_eq!(state.required_origins[&child_only], child);
        assert_eq!(
            state.required_include_chains[&shared],
            vec![PathBuf::from("/workspace/root.md.j2")]
        );
        assert_eq!(
            state.required_include_chains[&child_only],
            vec![
                PathBuf::from("/workspace/root.md.j2"),
                PathBuf::from("/workspace/child.md")
            ]
        );
    }

    #[test]
    fn inject_builtin_vars_characterizes_render_context_values() {
        let mut state = super::ValidationState::default();
        inject_builtin_vars(
            &mut state,
            PathBuf::from("/workspace/report.md.j2").as_path(),
        );

        assert_eq!(
            state.context[&variable("TEMPLATE_NAME")],
            json!("report.md.j2")
        );
        for name in [
            "TEMPLATE_NAME",
            "HOSTNAME",
            "USERNAME",
            "RENDER_DATE",
            "RENDER_TIMESTAMP",
        ] {
            let name = variable(name);
            assert!(state.declared_variables.contains(&name));
            assert_eq!(state.variable_sources[&name], VariableSource::Builtin);
            assert!(state.context[&name].is_string());
        }

        let render_date = state.context[&variable("RENDER_DATE")].as_str().unwrap();
        assert_eq!(render_date.len(), 10);
        assert_eq!(&render_date[4..5], "-");
        assert_eq!(&render_date[7..8], "-");
        let render_timestamp = state.context[&variable("RENDER_TIMESTAMP")]
            .as_str()
            .unwrap();
        assert!(render_timestamp.contains('T'));
        assert!(render_timestamp.ends_with('Z'));
    }

    fn empty_request() -> ComposeRequest {
        ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("root.md.j2"),
            },
            root: ConfiningRoot::from_path_buf(PathBuf::from("/workspace")),
            vars_input: BTreeMap::new(),
            vars_env: BTreeMap::new(),
            vars_defaults: BTreeMap::new(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        }
    }

    fn variable(name: &str) -> VariableName {
        VariableName::new(name).unwrap()
    }

    fn expected_declared(pass_variables: &[&str]) -> BTreeSet<VariableName> {
        let mut declared: BTreeSet<VariableName> = [
            "TEMPLATE_NAME",
            "HOSTNAME",
            "USERNAME",
            "RENDER_DATE",
            "RENDER_TIMESTAMP",
        ]
        .into_iter()
        .map(variable)
        .collect();
        declared.extend(pass_variables.iter().copied().map(variable));
        declared
    }
}
