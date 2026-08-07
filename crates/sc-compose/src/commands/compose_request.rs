use std::collections::BTreeMap;

use anyhow::anyhow;
use sc_composer::{
    ComposeRequest, ExpandedTemplate, Frontmatter, RecoveryHint, RecoveryHintKind, ResolveResult,
};

use crate::CommandError;
use crate::cli::{RenderArgs, ValidateArgs, parse_pass_inputs};
use crate::render_request::{
    build_multi_pass_request, build_request, read_block_pair,
    read_block_pair_with_extra_stdin_reads,
};

pub(super) fn build_render_request(args: &RenderArgs) -> Result<ComposeRequest, CommandError> {
    build_request(
        &args.common,
        read_block_pair(&args.common.input, &args.render)?,
        BTreeMap::default(),
    )
}

pub(super) fn build_multi_pass_render_request(
    args: &RenderArgs,
    pass_inputs: &[crate::cli::PassInputArgs],
) -> Result<ComposeRequest, CommandError> {
    build_multi_pass_request(
        &args.common,
        read_block_pair_with_extra_stdin_reads(
            &args.common.input,
            &args.render,
            stdin_reads(pass_inputs),
        )?,
        BTreeMap::default(),
        pass_inputs,
    )
}

pub(super) fn build_validate_request(
    args: &ValidateArgs,
    pass_inputs: &[crate::cli::PassInputArgs],
) -> Result<ComposeRequest, CommandError> {
    build_multi_pass_request(
        &args.common,
        read_block_pair_with_extra_stdin_reads(
            &args.common.input,
            &crate::cli::RenderBehaviorArgs::default(),
            stdin_reads(pass_inputs),
        )?,
        BTreeMap::default(),
        pass_inputs,
    )
}

pub(super) fn parse_pass_input_groups(
    command: &str,
) -> Result<Vec<crate::cli::PassInputArgs>, CommandError> {
    parse_pass_inputs(std::env::args_os(), command).map_err(pass_inputs_parse_error)
}

pub(super) fn preflight_template(
    request: &ComposeRequest,
) -> Result<(ResolveResult, ExpandedTemplate, Vec<Frontmatter>), CommandError> {
    let resolve_result =
        sc_composer::resolve_template_path(request).map_err(CommandError::compose)?;
    let expanded = sc_composer::expand_includes(
        &resolve_result.resolved_path,
        &request.root,
        &request.policy,
    )
    .map_err(CommandError::compose)?;
    let root_passes = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| (path == &resolve_result.resolved_path).then(|| passes.clone()))
        .unwrap_or_default();
    Ok((resolve_result, expanded, root_passes))
}

fn stdin_reads(pass_inputs: &[crate::cli::PassInputArgs]) -> usize {
    pass_inputs
        .iter()
        .flat_map(|pass| pass.var_files.iter())
        .filter(|path| path.as_str() == "-")
        .count()
}

fn pass_inputs_parse_error(error: String) -> CommandError {
    CommandError::usage_with_code_and_hints(
        anyhow!(error),
        sc_composer::DiagnosticCode::ErrConfigParse,
        vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "when using --all, declare each --pass N group before its --var/--var-file arguments"
                .to_owned(),
        })],
    )
}
