use anyhow::anyhow;
use sc_composer::{CompositionObserver, DiagnosticCode, RecoveryHint, RecoveryHintKind};

use crate::CommandError;
use crate::cli::{ExamplesArgs, ListArgs};
use crate::commands::compose::execute_render;
use crate::commands::{pack_not_found_error, pack_not_renderable_error, store_root_error};
use crate::render_request::{build_named_request, read_block_pair};
use crate::template_store::TemplateStore;

mod catalog;

pub(crate) fn run_examples_list(args: &ListArgs) -> Result<i32, CommandError> {
    let store = TemplateStore::examples()
        .map_err(|error| store_root_error(error, crate::template_store::DATA_DIR_ENV))?;
    let packs = catalog::load(&store).map_err(pack_not_renderable_error)?;
    catalog::print(&packs, args.json).map_err(CommandError::usage)?;
    Ok(crate::exit_codes::SUCCESS)
}

pub(crate) fn run_examples_render(
    args: &ExamplesArgs,
    name: &str,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let store = TemplateStore::examples()
        .map_err(|error| store_root_error(error, crate::template_store::DATA_DIR_ENV))?;
    let pack = store
        .get_example(name)
        .map_err(pack_not_renderable_error)?
        .ok_or_else(|| example_not_found(&store, name))?;
    let request = build_named_request(
        &pack,
        &args.input,
        read_block_pair(&args.input, &args.render)?,
    )?;
    execute_render(&request, &args.render, observer)
}

fn example_not_found(store: &TemplateStore, name: &str) -> CommandError {
    // Suggestions are best effort; a broken unrelated pack must not mask the
    // stable missing-pack diagnostic or its list recovery.
    let suggestion = catalog::load(store)
        .ok()
        .and_then(|packs| catalog::nearest(name, &packs).map(str::to_owned));
    let Some(suggestion) = suggestion else {
        return pack_not_found_error("example", name, "sc-compose examples list");
    };
    CommandError::usage_with_code_and_hints(
        anyhow!("example pack `{name}` was not found; did you mean `{suggestion}`?"),
        DiagnosticCode::ErrConfigPackNotFound,
        vec![RecoveryHint::new(RecoveryHintKind::RunCommand {
            command: "sc-compose examples list".to_owned(),
        })],
    )
}
