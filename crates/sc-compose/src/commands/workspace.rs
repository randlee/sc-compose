use std::path::{Path, PathBuf};

use crate::cli::{InitArgs, ObservabilityHealthArgs};
use crate::observer_impl::CliObserver;
use crate::path_utils::to_forward_slash;
use crate::{CommandError, observability, print_diagnostic_messages, print_json};
use anyhow::{Result, anyhow};
use sc_composer::{DiagnosticCode, RecoveryHint, RecoveryHintKind};

pub(crate) fn run_init(args: &InitArgs) -> Result<i32, CommandError> {
    let canonical_root = std::fs::canonicalize(&args.root).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!(error).context(format!(
                "failed to canonicalize workspace root {}",
                args.root.display()
            )),
            DiagnosticCode::ErrConfigParse,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: args.root.clone(),
            })],
        )
    })?;
    let prompts_dir_missing = !canonical_root.join(".prompts").exists();
    let planned_changes = planned_init_changes(&canonical_root)?;
    let result =
        sc_composer::init_workspace(&args.root, args.dry_run).map_err(CommandError::compose)?;
    if args.json {
        let payload = if args.dry_run {
            serde_json::json!({
                "action": "init",
                "would_affect": planned_changes
                    .iter()
                    .map(|path| to_forward_slash(path))
                    .collect::<Vec<_>>(),
                "changed": false,
                "would_change": !planned_changes.is_empty(),
                "skipped": planned_changes.is_empty(),
            })
        } else {
            serde_json::json!({
                "workspace_root": to_forward_slash(&canonical_root),
                "created_files": actual_init_created_files(
                    prompts_dir_missing,
                    result.gitignore_updated,
                ),
            })
        };
        print_json(payload, result.recommendations).map_err(CommandError::usage)?;
    } else if args.dry_run {
        for path in &planned_changes {
            println!("would_affect: {}", path.display());
        }
        print_diagnostic_messages(&result.recommendations);
    } else {
        println!("workspace_root: {}", canonical_root.display());
        print_diagnostic_messages(&result.recommendations);
    }
    Ok(if result.validation_passed {
        crate::exit_codes::SUCCESS
    } else {
        crate::exit_codes::VALIDATION_OR_RENDER_FAIL
    })
}

pub(crate) fn run_observability_health(
    args: &ObservabilityHealthArgs,
    observer: &mut CliObserver,
) -> Result<i32, CommandError> {
    if std::env::var_os("SC_COMPOSE_TEST_FORCE_QUERY_UNAVAILABLE").is_some() {
        observer.shutdown();
    }
    let health = observer.health();
    if args.json {
        print_json(
            serde_json::json!({ "logging": observability::health_json_value(&health) }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else {
        observability::print_observability_health(&health);
    }
    Ok(crate::exit_codes::SUCCESS)
}

fn planned_init_changes(root: &Path) -> Result<Vec<PathBuf>, CommandError> {
    let mut changes = Vec::new();
    let prompts_dir = root.join(".prompts");
    if !prompts_dir.exists() {
        changes.push(prompts_dir);
    }

    let gitignore = root.join(".gitignore");
    let current = read_optional_gitignore(&gitignore)?;
    if !current.lines().any(|line| line.trim() == ".prompts/") {
        changes.push(gitignore);
    }

    Ok(changes)
}

fn actual_init_created_files(prompts_dir_missing: bool, gitignore_updated: bool) -> Vec<String> {
    let mut created = Vec::new();
    if prompts_dir_missing {
        created.push(".prompts/".to_owned());
    }
    if gitignore_updated {
        created.push(".gitignore".to_owned());
    }
    created
}

fn read_optional_gitignore(path: &Path) -> Result<String, CommandError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(CommandError::usage_with_code_and_hints(
            anyhow!(error).context(format!("failed to read {}", path.display())),
            DiagnosticCode::ErrConfigParse,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: path.to_path_buf(),
            })],
        )),
    }
}
