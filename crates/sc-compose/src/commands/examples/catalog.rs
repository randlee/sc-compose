//! Read-only example discovery metadata and deterministic recovery suggestions.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use sc_composer::parse_template_document;

use crate::path_utils::to_forward_slash;
use crate::template_store::{TemplateMeta, TemplateStore};

pub(super) struct Example {
    meta: TemplateMeta,
    required_variables: Option<Vec<String>>,
    sample_vars: Option<PathBuf>,
    extensions: BTreeSet<String>,
}

pub(super) fn load(store: &TemplateStore) -> Result<Vec<Example>> {
    store
        .list()?
        .into_iter()
        .map(|meta| {
            let source = fs::read_to_string(&meta.path)
                .with_context(|| format!("failed to read example {}", meta.path.display()))?;
            // Discovery still lists a malformed or schema-less template. Do not
            // claim that its required inputs are known to be empty.
            let required_variables = parse_template_document(&source).ok().and_then(|parsed| {
                parsed
                    .passes()
                    .iter()
                    .any(sc_composer::Frontmatter::has_variable_schema)
                    .then(|| {
                        parsed
                            .passes()
                            .iter()
                            .flat_map(sc_composer::Frontmatter::required_variables)
                            .map(|variable| variable.as_str().to_owned())
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .collect()
                    })
            });
            let sample = store
                .source_dir
                .join(format!("{}.sample-vars.json", meta.name));
            let filename = meta.path.file_name().unwrap_or_default().to_string_lossy();
            let extensions = template_extensions(&format!("{filename}\n{source}"));
            Ok(Example {
                meta,
                required_variables,
                extensions,
                sample_vars: sample.is_file().then_some(sample),
            })
        })
        .collect()
}

pub(super) fn print(packs: &[Example], json: bool) -> Result<()> {
    if json {
        crate::print_json(
            serde_json::json!({ "packs": packs.iter().map(|pack| {
            serde_json::json!({
                "name": pack.meta.name,
                "path": to_forward_slash(&pack.meta.path),
                "required_variables": pack.required_variables,
                "sample_vars": pack.sample_vars.as_deref().map(to_forward_slash),
            })
        }).collect::<Vec<_>>() }),
            Vec::new(),
        )?;
    } else {
        for pack in packs {
            println!("{}", pack.meta.name);
            if let Some(sample) = &pack.sample_vars {
                println!("  sample vars (--var-file): {}", to_forward_slash(sample));
            } else if let Some(required) = &pack.required_variables {
                let names = if required.is_empty() {
                    "none declared".into()
                } else {
                    required.join(", ")
                };
                println!("  required variables: {names}");
            } else {
                println!(
                    "  required variables: inspect {}",
                    to_forward_slash(&pack.meta.path)
                );
            }
        }
    }
    Ok(())
}

fn template_extensions(source: &str) -> BTreeSet<String> {
    source
        .split(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '_' | '-'))
        .filter_map(|token| {
            let token = token.strip_suffix(".j2")?;
            let (_, extension) = token.rsplit_once('.')?;
            (!extension.is_empty()).then(|| extension.to_ascii_lowercase())
        })
        .collect()
}

/// Exact extension matches win, then small edit distance; names break ties.
pub(super) fn nearest<'a>(name: &str, packs: &'a [Example]) -> Option<&'a str> {
    let lower = name.to_ascii_lowercase();
    let query = lower
        .trim_start_matches('.')
        .strip_suffix(".j2")
        .unwrap_or(lower.trim_start_matches('.'));
    if query.is_empty() {
        return None;
    }
    let limit = if query.chars().count() < 4 { 1 } else { 2 };
    packs
        .iter()
        .filter_map(|pack| {
            let score = std::iter::once(pack.meta.name.to_ascii_lowercase())
                .chain(pack.extensions.iter().cloned())
                .filter_map(|candidate| distance(query, &candidate, limit))
                .min()?;
            Some((score, pack.meta.name.as_str()))
        })
        .min()
        .map(|(_, name)| name)
}

/// Bounded Levenshtein distance; unrelated names are never suggestions.
fn distance(left: &str, right: &str, limit: usize) -> Option<usize> {
    let left: Vec<_> = left.chars().collect();
    let right: Vec<_> = right.chars().collect();
    if left.len().abs_diff(right.len()) > limit {
        return None;
    }
    let mut previous: Vec<_> = (0..=right.len()).collect();
    for (row, left_char) in left.iter().enumerate() {
        let mut current = vec![row + 1];
        for (column, right_char) in right.iter().enumerate() {
            current.push(
                (previous[column + 1] + 1)
                    .min(current[column] + 1)
                    .min(previous[column] + usize::from(left_char != right_char)),
            );
        }
        if current.iter().all(|cost| *cost > limit) {
            return None;
        }
        previous = current;
    }
    let result = previous[right.len()];
    (result <= limit).then_some(result)
}
