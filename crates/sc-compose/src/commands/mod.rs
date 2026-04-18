pub(crate) mod examples;
pub(crate) mod templates;

use std::path::PathBuf;

use anyhow::anyhow;
use sc_composer::{DiagnosticCode, RecoveryHint, RecoveryHintKind};

use crate::template_store::TemplateMeta;
use crate::{CommandError, print_json};

fn print_pack_list(packs: &[TemplateMeta], json: bool) -> anyhow::Result<()> {
    if json {
        print_json(
            serde_json::json!({
                "packs": packs
                    .iter()
                    .map(|pack| serde_json::json!({
                        "name": pack.name,
                        "path": pack.path.display().to_string(),
                    }))
                    .collect::<Vec<_>>(),
            }),
            Vec::new(),
        )?;
    } else {
        for pack in packs {
            match (&pack.description, &pack.version) {
                (Some(description), Some(version)) => {
                    println!("{} - {} ({version})", pack.name, description);
                }
                (Some(description), None) => println!("{} - {}", pack.name, description),
                (None, Some(version)) => println!("{} ({version})", pack.name),
                (None, None) => println!("{}", pack.name),
            }
        }
    }
    Ok(())
}

fn pack_not_found_error(kind: &str, name: &str, list_command: &str) -> CommandError {
    CommandError::usage_with_code_and_hints(
        anyhow!("{kind} pack `{name}` was not found"),
        DiagnosticCode::ErrConfigPackNotFound,
        vec![RecoveryHint::new(RecoveryHintKind::RunCommand {
            command: list_command.to_owned(),
        })],
    )
}

fn pack_not_renderable_error(error: anyhow::Error) -> CommandError {
    CommandError::usage_with_code_and_hints(
        error,
        DiagnosticCode::ErrConfigPackNotRenderable,
        vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "add a .j2 file to the template pack directory".to_owned(),
        })],
    )
}

fn template_exists_error(destination: PathBuf) -> CommandError {
    CommandError::usage_with_code_and_hints(
        anyhow!("template pack already exists at {}", destination.display()),
        DiagnosticCode::ErrConfigTemplateExists,
        vec![
            RecoveryHint::new(RecoveryHintKind::InspectPath { path: destination }),
            RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "delete the existing template or use a different name".to_owned(),
            }),
        ],
    )
}
