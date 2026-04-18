use anyhow::anyhow;
use sc_composer::{CompositionObserver, DiagnosticCode};

use crate::commands::{pack_not_found_error, print_pack_list};
use crate::render_request::{build_named_request, read_block_pair};
use crate::template_store::TemplateStore;
use crate::{CommandError, ExamplesArgs, ListArgs, execute_render};

pub(crate) fn run_examples_list(args: &ListArgs) -> Result<i32, CommandError> {
    let store = TemplateStore::examples()
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))?;
    let packs = store
        .list()
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))?;
    print_pack_list(&packs, args.json).map_err(CommandError::usage)?;
    Ok(crate::exit_codes::SUCCESS)
}

pub(crate) fn run_examples_render(
    args: &ExamplesArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let name = args
        .name
        .as_deref()
        .ok_or_else(|| CommandError::usage(anyhow!("missing example pack name")))?;
    let store = TemplateStore::examples()
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))?;
    let pack = store
        .get_example(name)
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))?
        .ok_or_else(|| pack_not_found_error("example", name, "sc-compose examples list"))?;
    let request = build_named_request(
        &pack,
        &args.input,
        read_block_pair(&args.input, &args.render)?,
    )?;
    execute_render(&request, &args.render, observer)
}
