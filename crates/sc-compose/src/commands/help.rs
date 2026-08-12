use anyhow::anyhow;
use sc_composer::DiagnosticCode;

use crate::CommandError;
use crate::cli::HelpArgs;
use crate::exit_codes;
use crate::help_topics;

pub(crate) fn run_help(args: &HelpArgs) -> Result<i32, CommandError> {
    if args.list {
        for topic in help_topics::topic_names() {
            println!("{topic}");
        }
        return Ok(exit_codes::SUCCESS);
    }

    if let Some(topic) = args.topic.as_deref() {
        if let Some(manual) = help_topics::find(topic) {
            print!("{manual}");
            return Ok(exit_codes::SUCCESS);
        }

        let valid_topics = help_topics::topic_names().join(", ");
        return Err(CommandError::usage_with_code(
            anyhow!("unknown manual topic `{topic}`; valid topics: {valid_topics}"),
            DiagnosticCode::ErrConfigParse,
        ));
    }

    print!("{}", help_topics::index());
    Ok(exit_codes::SUCCESS)
}
