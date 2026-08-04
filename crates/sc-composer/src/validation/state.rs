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
