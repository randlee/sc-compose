//! Convert the live clap command tree into docli's language-neutral model.
use clap::{Command, CommandFactory};
use serde::Serialize;
use serde_json::{Value, json};

use crate::cli::Cli;

#[derive(Serialize)]
struct CommandModel {
    name: String,
    version: Option<String>,
    description: String,
    long_description: String,
    epilogue: String,
    usage: String,
    options: Vec<OptionModel>,
    arguments: Vec<ArgumentModel>,
    subcommands: Vec<CommandModel>,
}

#[derive(Serialize)]
struct OptionModel {
    name: String,
    long: Option<String>,
    short: Option<String>,
    help: String,
    long_help: String,
    value_name: Option<String>,
    required: bool,
    default_value: Option<String>,
    choices: Vec<String>,
    min_values: Option<usize>,
    max_values: Option<usize>,
}

#[derive(Serialize)]
struct ArgumentModel {
    name: String,
    help: String,
    required: bool,
    default_value: Option<String>,
    choices: Vec<String>,
}

pub(crate) fn model() -> Value {
    let command = Cli::command();
    json!(command_model(&command, command.get_name()))
}

fn command_model(command: &Command, path: &str) -> CommandModel {
    let (options, arguments) = command_arguments(command);
    let mut subcommands: Vec<_> = command
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| command_model(c, &format!("{path} {}", c.get_name())))
        .collect();
    subcommands.sort_by(|a, b| a.name.cmp(&b.name));
    CommandModel {
        name: command.get_name().to_owned(),
        version: command.get_version().map(str::to_owned),
        description: command
            .get_about()
            .map(ToString::to_string)
            .unwrap_or_default(),
        long_description: command
            .get_long_about()
            .map(ToString::to_string)
            .unwrap_or_default(),
        epilogue: command
            .get_after_help()
            .map(ToString::to_string)
            .unwrap_or_default(),
        usage: command
            .clone()
            .bin_name(path.to_owned())
            .render_usage()
            .to_string(),
        options,
        arguments,
        subcommands,
    }
}

fn command_arguments(command: &Command) -> (Vec<OptionModel>, Vec<ArgumentModel>) {
    let mut options = Vec::new();
    let mut arguments = Vec::new();
    for arg in command.get_arguments().filter(|arg| !arg.is_hide_set()) {
        let name = arg.get_id().as_str();
        if name == "help" || name == "version" {
            continue;
        }
        let long = arg.get_long().map(|value| format!("--{value}"));
        let short = arg.get_short().map(|value| format!("-{value}"));
        let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
        let default_value = arg
            .get_default_values()
            .first()
            .map(|v| v.to_string_lossy().into_owned());
        let choices = arg
            .get_possible_values()
            .iter()
            .filter(|v| !v.is_hide_set())
            .map(|v| v.get_name().to_owned())
            .collect();
        if long.is_none() && short.is_none() {
            arguments.push(ArgumentModel {
                name: name.to_owned(),
                help,
                required: arg.is_required_set(),
                default_value,
                choices,
            });
        } else {
            options.push(option_model(arg, long, short, help, default_value, choices));
        }
    }
    options.sort_by(|a, b| a.name.cmp(&b.name));
    arguments.sort_by(|a, b| a.name.cmp(&b.name));
    (options, arguments)
}

fn option_model(
    arg: &clap::Arg,
    long: Option<String>,
    short: Option<String>,
    help: String,
    default_value: Option<String>,
    choices: Vec<String>,
) -> OptionModel {
    let range = arg.get_num_args();
    OptionModel {
        name: arg.get_id().as_str().to_owned(),
        long,
        short,
        help,
        long_help: arg
            .get_long_help()
            .map(ToString::to_string)
            .unwrap_or_default(),
        value_name: arg
            .get_value_names()
            .and_then(|names| names.first())
            .map(ToString::to_string),
        required: arg.is_required_set(),
        default_value,
        choices,
        min_values: range.map(|v| v.min_values()),
        max_values: range.map(|v| v.max_values()),
    }
}

#[cfg(test)]
mod tests {
    use super::command_model;
    use clap::{Arg, Command};

    #[test]
    fn walks_new_nested_commands_and_argument_metadata() {
        let command = Command::new("demo").subcommand(
            Command::new("future").arg(
                Arg::new("format")
                    .long("format")
                    .value_parser(["json", "text"])
                    .default_value("json"),
            ),
        );
        let model = serde_json::to_value(command_model(&command, "demo")).unwrap();
        let child = &model["subcommands"][0];
        assert_eq!(child["name"], "future");
        assert_eq!(
            child["options"][0]["choices"],
            serde_json::json!(["json", "text"])
        );
        assert_eq!(child["options"][0]["default_value"], "json");
        assert!(child["usage"].as_str().unwrap().contains("demo future"));
    }
}
