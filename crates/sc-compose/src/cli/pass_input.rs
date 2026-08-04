use std::ffi::OsString;

use clap::Parser;

use super::Cli;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PassInputArgs {
    pub(crate) pass_number: u8,
    pub(crate) vars: Vec<(String, String)>,
    pub(crate) var_files: Vec<String>,
}

pub(crate) fn parse_var(input: &str) -> Result<(String, String), String> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| "expected key=value".to_owned())?;
    Ok((key.to_owned(), value.to_owned()))
}

pub(crate) fn parse_pass_inputs<I>(
    args: I,
    command_name: &str,
) -> Result<Vec<PassInputArgs>, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let mut found_command = false;
    let mut current: Option<PassInputArgs> = None;
    let mut parsed = Vec::new();

    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if !found_command {
            if arg == command_name {
                found_command = true;
            }
            continue;
        }

        match arg.as_ref() {
            "--pass" => {
                if let Some(group) = current.take() {
                    parsed.push(group);
                }
                let Some(value) = args.next() else {
                    return Err("--pass requires a numeric pass number".to_owned());
                };
                let value = value.to_string_lossy();
                let pass_number = value
                    .parse::<u8>()
                    .map_err(|error| format!("invalid pass number `{value}`: {error}"))?;
                current = Some(PassInputArgs {
                    pass_number,
                    vars: Vec::new(),
                    var_files: Vec::new(),
                });
            }
            _ if arg.starts_with("--pass=") => {
                if let Some(group) = current.take() {
                    parsed.push(group);
                }
                let value = arg.strip_prefix("--pass=").unwrap_or(arg.as_ref());
                let pass_number = value
                    .parse::<u8>()
                    .map_err(|error| format!("invalid pass number `{value}`: {error}"))?;
                current = Some(PassInputArgs {
                    pass_number,
                    vars: Vec::new(),
                    var_files: Vec::new(),
                });
            }
            "--var" => {
                let Some(value) = args.next() else {
                    return Err("--var requires key=value".to_owned());
                };
                let value = value.to_string_lossy();
                let current = current
                    .as_mut()
                    .ok_or_else(|| "--var must appear after --pass".to_owned())?;
                current.vars.push(parse_var(&value)?);
            }
            _ if arg.starts_with("--var=") => {
                let current = current
                    .as_mut()
                    .ok_or_else(|| "--var must appear after --pass".to_owned())?;
                current.vars.push(parse_var(
                    arg.strip_prefix("--var=").unwrap_or(arg.as_ref()),
                )?);
            }
            "--var-file" => {
                let Some(value) = args.next() else {
                    return Err("--var-file requires a path".to_owned());
                };
                let current = current
                    .as_mut()
                    .ok_or_else(|| "--var-file must appear after --pass".to_owned())?;
                current.var_files.push(value.to_string_lossy().into_owned());
            }
            _ if arg.starts_with("--var-file=") => {
                let current = current
                    .as_mut()
                    .ok_or_else(|| "--var-file must appear after --pass".to_owned())?;
                current.var_files.push(
                    arg.strip_prefix("--var-file=")
                        .unwrap_or(arg.as_ref())
                        .to_owned(),
                );
            }
            _ => {}
        }
    }

    if let Some(group) = current {
        parsed.push(group);
    }

    Ok(parsed)
}

pub(crate) fn parse_cli() -> Cli {
    Cli::parse_from(super::filtered_args_for_clap(std::env::args_os()))
}

pub(crate) fn filtered_args_for_clap<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut filtered = Vec::new();
    let mut args = args.into_iter();
    let Some(program) = args.next() else {
        return filtered;
    };
    filtered.push(program);

    let mut command_name: Option<String> = None;
    let mut in_pass_group = false;

    while let Some(arg) = args.next() {
        let arg_text = arg.to_string_lossy();
        if command_name.is_none() {
            if matches!(
                arg_text.as_ref(),
                "render" | "validate" | "verify" | "template-init"
            ) {
                command_name = Some(arg_text.into_owned());
                in_pass_group = false;
            }
            filtered.push(arg);
            continue;
        }

        if matches!(arg_text.as_ref(), "--pass") {
            in_pass_group = true;
            let _ = args.next();
            continue;
        }
        if arg_text.starts_with("--pass=") {
            in_pass_group = true;
            continue;
        }
        if in_pass_group && matches!(arg_text.as_ref(), "--var" | "--var-file") {
            let _ = args.next();
            continue;
        }
        if in_pass_group && (arg_text.starts_with("--var=") || arg_text.starts_with("--var-file="))
        {
            continue;
        }

        in_pass_group = false;
        filtered.push(arg);
    }

    filtered
}
