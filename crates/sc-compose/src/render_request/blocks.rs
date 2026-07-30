use std::io::Read;

use anyhow::anyhow;
use sc_composer::DiagnosticCode;

use crate::CommandError;
use crate::cli::{InputArgs, RenderBehaviorArgs};

pub(crate) fn read_block_pair(
    input: &InputArgs,
    render: &RenderBehaviorArgs,
) -> Result<(Option<String>, Option<String>), CommandError> {
    read_block_pair_with_extra_stdin_reads(input, render, 0)
}

pub(crate) fn read_block_pair_with_extra_stdin_reads(
    input: &InputArgs,
    render: &RenderBehaviorArgs,
    extra_stdin_reads: usize,
) -> Result<(Option<String>, Option<String>), CommandError> {
    validate_block_sources(input, render, extra_stdin_reads)?;
    let guidance = read_optional_block(render.guidance.clone(), render.guidance_file.as_deref())?;
    let prompt = read_optional_block(render.prompt.clone(), render.prompt_file.as_deref())?;
    Ok((guidance, prompt))
}

fn validate_block_sources(
    input: &InputArgs,
    render: &RenderBehaviorArgs,
    extra_stdin_reads: usize,
) -> Result<(), CommandError> {
    if render.guidance.is_some() && render.guidance_file.is_some() {
        return Err(CommandError::usage_with_code(
            anyhow!("--guidance and --guidance-file are mutually exclusive"),
            DiagnosticCode::ErrConfigParse,
        ));
    }
    if render.prompt.is_some() && render.prompt_file.is_some() {
        return Err(CommandError::usage_with_code(
            anyhow!("--prompt and --prompt-file are mutually exclusive"),
            DiagnosticCode::ErrConfigParse,
        ));
    }
    let stdin_reads = input
        .var_file
        .iter()
        .filter(|path| path.as_str() == "-")
        .count()
        + usize::from(render.guidance_file.as_deref() == Some("-"))
        + usize::from(render.prompt_file.as_deref() == Some("-"));
    if stdin_reads + extra_stdin_reads > 1 {
        return Err(CommandError::stdin_double_read());
    }
    Ok(())
}

fn read_optional_block(
    inline: Option<String>,
    file: Option<&str>,
) -> Result<Option<String>, CommandError> {
    read_optional_block_with(inline, file, read_stdin_to_string, read_file_to_string)
}

pub(super) fn read_optional_block_with(
    inline: Option<String>,
    file: Option<&str>,
    read_stdin: impl FnOnce() -> Result<String, CommandError>,
    read_file: impl FnOnce(&str) -> Result<String, CommandError>,
) -> Result<Option<String>, CommandError> {
    if let Some(inline) = inline {
        return Ok(Some(inline));
    }
    match file {
        Some("-") => read_stdin().map(Some),
        Some(path) => read_file(path).map(Some),
        None => Ok(None),
    }
}

fn read_stdin_to_string() -> Result<String, CommandError> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| {
            CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
        })?;
    Ok(input)
}

fn read_file_to_string(path: &str) -> Result<String, CommandError> {
    std::fs::read_to_string(path).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigParse)
    })
}
