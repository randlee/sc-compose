use anyhow::anyhow;
use sc_composer::DiagnosticCode;

use crate::CommandError;
use crate::cli::HelpArgs;
use crate::exit_codes;
use crate::help_topics;
use crate::print_json;

fn print_topic_index_json(topics: &[&'static str]) -> Result<(), CommandError> {
    print_json(serde_json::json!({ "topics": topics }), Vec::new()).map_err(CommandError::usage)
}

pub(crate) fn run_help(args: &HelpArgs) -> Result<i32, CommandError> {
    if args.list {
        let topics = help_topics::topic_names();
        if args.json {
            print_topic_index_json(&topics)?;
        } else {
            for topic in topics {
                println!("{topic}");
            }
        }
        return Ok(exit_codes::SUCCESS);
    }

    if let Some(topic) = args.topic.as_deref() {
        if let Some(manual) = help_topics::find(topic) {
            if args.json {
                print_json(
                    serde_json::json!({ "topic": topic, "manual": manual }),
                    Vec::new(),
                )
                .map_err(CommandError::usage)?;
            } else {
                print!("{manual}");
            }
            return Ok(exit_codes::SUCCESS);
        }

        let valid_topics = help_topics::topic_names().join(", ");
        return Err(CommandError::usage_with_code_and_hints(
            anyhow!("unknown manual topic `{topic}`; valid topics: {valid_topics}"),
            DiagnosticCode::ErrConfigHelpTopicNotFound,
            vec![sc_composer::RecoveryHint::new(
                sc_composer::RecoveryHintKind::RunCommand {
                    command: "sc-compose help --list".to_owned(),
                },
            )],
        ));
    }

    if args.json {
        let topics = help_topics::topic_names();
        print_topic_index_json(&topics)?;
    } else {
        print!("{}", help_topics::index());
    }
    Ok(exit_codes::SUCCESS)
}
