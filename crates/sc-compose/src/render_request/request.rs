use std::collections::BTreeMap;

use anyhow::anyhow;
use sc_composer::{ComposeRequest, ConfiningRoot, DiagnosticCode, InputValue, VariableName};

use crate::CommandError;
use crate::cli::{CommonArgs, InputArgs, PassInputArgs};
use crate::render_request::mode::{build_mode, build_root, runtime_kind};
use crate::render_request::vars::{
    build_pass_configs, collect_pass_union, compose_policy, load_cli_and_file_vars,
    load_prefixed_env_vars,
};
use crate::template_store::TemplatePack;

pub(crate) fn build_request(
    args: &CommonArgs,
    blocks: (Option<String>, Option<String>),
    vars_defaults: BTreeMap<VariableName, InputValue>,
) -> Result<ComposeRequest, CommandError> {
    let root = build_root(&args.root)?;
    let mode = build_mode(args)?;

    Ok(ComposeRequest {
        runtime: args.runtime.map(runtime_kind),
        mode,
        root,
        vars_input: load_cli_and_file_vars(&args.input)?,
        vars_env: load_prefixed_env_vars(&args.input)?,
        vars_defaults,
        guidance_block: blocks.0,
        user_prompt: blocks.1,
        policy: compose_policy(&args.input),
    })
}

pub(crate) fn build_multi_pass_request(
    args: &CommonArgs,
    blocks: (Option<String>, Option<String>),
    vars_defaults: BTreeMap<VariableName, InputValue>,
    pass_inputs: &[PassInputArgs],
) -> Result<ComposeRequest, CommandError> {
    let root = build_root(&args.root)?;
    let mode = build_mode(args)?;

    let vars_input = collect_pass_union(pass_inputs)?;
    let vars_env = load_prefixed_env_vars(&args.input)?;
    let policy = sc_composer::ComposePolicy {
        passes: build_pass_configs(pass_inputs)?,
        ..compose_policy(&args.input)
    };

    Ok(ComposeRequest {
        runtime: args.runtime.map(runtime_kind),
        mode,
        root,
        vars_input,
        vars_env,
        vars_defaults,
        guidance_block: blocks.0,
        user_prompt: blocks.1,
        policy,
    })
}

pub(crate) fn build_named_request(
    pack: &TemplatePack,
    input: &InputArgs,
    blocks: (Option<String>, Option<String>),
) -> Result<ComposeRequest, CommandError> {
    let root = ConfiningRoot::new(&pack.root).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!(error).context(format!(
                "failed to canonicalize root {}",
                pack.root.display()
            )),
            DiagnosticCode::ErrConfigParse,
        )
    })?;

    Ok(ComposeRequest {
        runtime: None,
        mode: sc_composer::ComposeMode::File {
            template_path: pack.template_path.clone(),
        },
        root,
        vars_input: load_cli_and_file_vars(input)?,
        vars_env: load_prefixed_env_vars(input)?,
        vars_defaults: pack.input_defaults.clone(),
        guidance_block: blocks.0,
        user_prompt: blocks.1,
        policy: compose_policy(input),
    })
}
