//! Convert the live clap command tree into docli's language-neutral model.
use clap::{Command, CommandFactory};
use serde_json::{Value, json};

use crate::cli::Cli;

pub(crate) fn model() -> Value {
    let command = Cli::command();
    command_model(&command, command.get_name())
}

fn command_model(command: &Command, path: &str) -> Value {
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
        let choices: Vec<_> = arg
            .get_possible_values()
            .iter()
            .filter(|v| !v.is_hide_set())
            .map(|v| v.get_name().to_owned())
            .collect();
        if long.is_none() && short.is_none() {
            arguments.push(json!({"name": name, "help": help, "required": arg.is_required_set(), "default_value": default_value, "choices": choices}));
        } else {
            let range = arg.get_num_args();
            options.push(json!({
                "name": name, "long": long, "short": short, "help": help,
                "long_help": arg.get_long_help().map(ToString::to_string).unwrap_or_default(),
                "value_name": arg.get_value_names().and_then(|names| names.first()).map(ToString::to_string),
                "required": arg.is_required_set(), "default_value": default_value, "choices": choices,
                "min_values": range.map(|v| v.min_values()), "max_values": range.map(|v| v.max_values())
            }));
        }
    }
    options.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    arguments.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    let mut subcommands: Vec<_> = command
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| command_model(c, &format!("{path} {}", c.get_name())))
        .collect();
    subcommands.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    json!({
        "name": command.get_name(), "version": command.get_version(),
        "description": command.get_about().map(ToString::to_string).unwrap_or_default(),
        "long_description": command.get_long_about().map(ToString::to_string).unwrap_or_default(),
        "epilogue": command.get_after_help().map(ToString::to_string).unwrap_or_default(),
        "usage": command.clone().bin_name(path.to_owned()).render_usage().to_string(),
        "options": options, "arguments": arguments, "subcommands": subcommands
    })
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
        let model = command_model(&command, "demo");
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
