use anyhow::anyhow;
use sc_composer::{CompositionObserver, DiagnosticCode};

use crate::commands::{
    pack_not_found_error, pack_not_renderable_error, print_pack_list, store_root_error,
    template_exists_error,
};
use crate::render_request::{build_named_request, read_block_pair};
use crate::template_store::{AddError, GetTemplateError, TemplateStore};
use crate::{CommandError, ListArgs, TemplatesAddArgs, TemplatesArgs, execute_render, print_json};

pub(crate) fn run_templates_list(args: &ListArgs) -> Result<i32, CommandError> {
    let store = TemplateStore::templates()
        .map_err(|error| store_root_error(error, "SC_COMPOSE_TEMPLATE_DIR"))?;
    let packs = store
        .list()
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))?;
    print_pack_list(&packs, args.json).map_err(CommandError::usage)?;
    Ok(crate::exit_codes::SUCCESS)
}

pub(crate) fn run_templates_render(
    args: &TemplatesArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError> {
    let name = args
        .name
        .as_deref()
        .ok_or_else(|| CommandError::usage(anyhow!("missing template pack name")))?;
    let store = TemplateStore::templates()
        .map_err(|error| store_root_error(error, "SC_COMPOSE_TEMPLATE_DIR"))?;
    let pack = match store.get_template(name) {
        Ok(Some(pack)) => pack,
        Ok(None) => {
            return Err(pack_not_found_error(
                "template",
                name,
                "sc-compose templates list",
            ));
        }
        Err(GetTemplateError::Parse(error)) => {
            return Err(CommandError::usage_with_code(
                error,
                DiagnosticCode::ErrConfigParse,
            ));
        }
        Err(GetTemplateError::NotRenderable(error)) => {
            return Err(pack_not_renderable_error(error));
        }
    };
    let request = build_named_request(
        &pack,
        &args.input,
        read_block_pair(&args.input, &args.render)?,
    )?;
    execute_render(&request, &args.render, observer)
}

pub(crate) fn run_templates_add(args: &TemplatesAddArgs) -> Result<i32, CommandError> {
    let store = TemplateStore::templates()
        .map_err(|error| store_root_error(error, "SC_COMPOSE_TEMPLATE_DIR"))?;
    let result = match store.add(&args.src, args.name.as_deref()) {
        Ok(result) => result,
        Err(AddError::AlreadyExists { destination }) => {
            return Err(template_exists_error(destination));
        }
        Err(AddError::Other(error)) => {
            return Err(CommandError::usage_with_code(
                error,
                DiagnosticCode::ErrConfigParse,
            ));
        }
    };
    if args.json {
        print_json(
            serde_json::json!({
                "name": result.name,
                "source": result.source.display().to_string(),
                "destination": result.destination.display().to_string(),
                "changed": result.changed,
            }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else {
        println!("{}", result.destination.display());
    }
    Ok(crate::exit_codes::SUCCESS)
}
