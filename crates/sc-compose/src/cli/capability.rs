use super::{Command, ExamplesSubcommand, TemplatesSubcommand};

pub(crate) fn command_wants_json(command: &Command) -> bool {
    match command {
        Command::Lint(args) => args.json,
        Command::Help(_) => false,
        Command::Render(args) => args.render.json,
        Command::Resolve(args) => args.json,
        Command::Validate(args) => args.json,
        Command::Verify(args) => args.json,
        Command::Extract(args) => args.json,
        Command::TemplateInit(args) => args.json,
        Command::FrontmatterInit(args) => args.json,
        Command::Init(args) => args.json,
        Command::ObservabilityHealth(args) => args.json,
        Command::Examples(args) => args
            .command
            .as_ref()
            .map_or(args.render.json, |subcommand| match subcommand {
                ExamplesSubcommand::List(args) => args.json,
            }),
        Command::Templates(args) => args
            .command
            .as_ref()
            .map_or(args.render.json, |subcommand| match subcommand {
                TemplatesSubcommand::List(args) => args.json,
                TemplatesSubcommand::Add(args) => args.json,
            }),
        Command::Reports(args) => match &args.command {
            super::ReportsSubcommand::Init(args) => args.json,
            super::ReportsSubcommand::Smoke(args) => args.json,
            super::ReportsSubcommand::Finalize(args) => args.json,
            super::ReportsSubcommand::RenderSpec(args) => args.json,
            super::ReportsSubcommand::Index(args) => args.json,
            super::ReportsSubcommand::Verify(args) => args.json,
            super::ReportsSubcommand::PublishManifest(args) => args.json,
        },
        Command::ReportRenderMany(args) => args.json,
        Command::ReportCatalog(args) => args.json,
    }
}
