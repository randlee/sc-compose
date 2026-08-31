use super::{
    BeadArgs, BeadSubcommand, Command, ExamplesArgs, ExamplesSubcommand, ReportsArgs,
    ReportsSubcommand, TemplatesArgs, TemplatesSubcommand,
};

pub(crate) fn command_wants_json(command: &Command) -> bool {
    match command {
        Command::Lint(args) => args.json,
        Command::Help(args) => args.json,
        Command::Render(args) => args.render.json,
        Command::Resolve(args) => args.json,
        Command::Validate(args) => args.json,
        Command::Verify(args) => args.json,
        Command::Extract(args) => args.json,
        Command::TemplateInit(args) => args.json,
        Command::FrontmatterInit(args) => args.json,
        Command::Init(args) => args.json,
        Command::ObservabilityHealth(args) => args.json,
        Command::Examples(args) => examples_want_json(args),
        Command::Templates(args) => templates_want_json(args),
        Command::Reports(args) => reports_want_json(args),
        Command::Bead(args) => bead_wants_json(args),
        Command::ReportRenderMany(args) => args.json,
        Command::ReportCatalog(args) => args.json,
    }
}

fn examples_want_json(args: &ExamplesArgs) -> bool {
    args.command
        .as_ref()
        .map_or(args.render.json, examples_subcommand_wants_json)
}

const fn examples_subcommand_wants_json(command: &ExamplesSubcommand) -> bool {
    match command {
        ExamplesSubcommand::List(args) => args.json,
    }
}

fn templates_want_json(args: &TemplatesArgs) -> bool {
    args.command
        .as_ref()
        .map_or(args.render.json, templates_subcommand_wants_json)
}

const fn templates_subcommand_wants_json(command: &TemplatesSubcommand) -> bool {
    match command {
        TemplatesSubcommand::List(args) => args.json,
        TemplatesSubcommand::Add(args) => args.json,
    }
}

const fn reports_want_json(args: &ReportsArgs) -> bool {
    match &args.command {
        ReportsSubcommand::Init(args) => args.json,
        ReportsSubcommand::Smoke(args) => args.json,
        ReportsSubcommand::Finalize(args) => args.json,
        ReportsSubcommand::RenderSpec(args) => args.json,
        ReportsSubcommand::Index(args) => args.json,
        ReportsSubcommand::Verify(args) => args.json,
        ReportsSubcommand::PublishManifest(args) => args.json,
    }
}

const fn bead_wants_json(args: &BeadArgs) -> bool {
    match &args.command {
        BeadSubcommand::Render(args)
        | BeadSubcommand::Validate(args)
        | BeadSubcommand::PreviewPour(args)
        | BeadSubcommand::Pour(args) => args.json,
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{super::Cli, command_wants_json};

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the flat table is an auditable exhaustive record of the public command matrix"
    )]
    fn command_json_capability_matrix_is_exhaustive() {
        let cases: &[(&[&str], bool)] = &[
            (&["sc-compose", "lint"], false),
            (&["sc-compose", "lint", "--json"], true),
            (&["sc-compose", "help"], false),
            (&["sc-compose", "help", "--json"], true),
            (&["sc-compose", "render"], false),
            (&["sc-compose", "render", "--json"], true),
            (&["sc-compose", "resolve"], false),
            (&["sc-compose", "resolve", "--json"], true),
            (&["sc-compose", "validate"], false),
            (&["sc-compose", "validate", "--json"], true),
            (&["sc-compose", "verify", "deployed.txt"], false),
            (&["sc-compose", "verify", "deployed.txt", "--json"], true),
            (
                &["sc-compose", "extract", "template.xml.j2", "rendered.xml"],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "extract",
                    "template.xml.j2",
                    "rendered.xml",
                    "--json",
                ],
                true,
            ),
            (&["sc-compose", "template-init", "template.txt"], false),
            (
                &["sc-compose", "template-init", "template.txt", "--json"],
                true,
            ),
            (
                &["sc-compose", "frontmatter-init", "--file", "template.txt"],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "frontmatter-init",
                    "--file",
                    "template.txt",
                    "--json",
                ],
                true,
            ),
            (&["sc-compose", "init"], false),
            (&["sc-compose", "init", "--json"], true),
            (&["sc-compose", "observability-health"], false),
            (&["sc-compose", "observability-health", "--json"], true),
            (&["sc-compose", "examples"], false),
            (&["sc-compose", "examples", "--json"], true),
            (&["sc-compose", "examples", "list"], false),
            (&["sc-compose", "examples", "list", "--json"], true),
            (&["sc-compose", "templates"], false),
            (&["sc-compose", "templates", "--json"], true),
            (&["sc-compose", "templates", "list"], false),
            (&["sc-compose", "templates", "list", "--json"], true),
            (&["sc-compose", "templates", "add", "pack"], false),
            (&["sc-compose", "templates", "add", "pack", "--json"], true),
            (&["sc-compose", "reports", "init"], false),
            (&["sc-compose", "reports", "init", "--json"], true),
            (
                &[
                    "sc-compose",
                    "reports",
                    "smoke",
                    "--fixture",
                    "fixture",
                    "--vars",
                    "vars",
                ],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "reports",
                    "smoke",
                    "--fixture",
                    "fixture",
                    "--vars",
                    "vars",
                    "--json",
                ],
                true,
            ),
            (
                &[
                    "sc-compose",
                    "reports",
                    "finalize",
                    "--report-id",
                    "id",
                    "--kind",
                    "kind",
                    "--entrypoint",
                    "report.html",
                ],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "reports",
                    "finalize",
                    "--report-id",
                    "id",
                    "--kind",
                    "kind",
                    "--entrypoint",
                    "report.html",
                    "--json",
                ],
                true,
            ),
            (
                &[
                    "sc-compose",
                    "reports",
                    "render-spec",
                    "--spec",
                    "spec.toml",
                ],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "reports",
                    "render-spec",
                    "--spec",
                    "spec.toml",
                    "--json",
                ],
                true,
            ),
            (&["sc-compose", "reports", "index"], false),
            (&["sc-compose", "reports", "index", "--json"], true),
            (&["sc-compose", "reports", "verify"], false),
            (&["sc-compose", "reports", "verify", "--json"], true),
            (&["sc-compose", "reports", "publish-manifest"], false),
            (
                &["sc-compose", "reports", "publish-manifest", "--json"],
                true,
            ),
            (
                &["sc-compose", "bead", "render", "--request", "request.json"],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "bead",
                    "render",
                    "--request",
                    "request.json",
                    "--json",
                ],
                true,
            ),
            (
                &[
                    "sc-compose",
                    "bead",
                    "validate",
                    "--request",
                    "request.json",
                ],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "bead",
                    "validate",
                    "--request",
                    "request.json",
                    "--json",
                ],
                true,
            ),
            (
                &[
                    "sc-compose",
                    "bead",
                    "preview-pour",
                    "--request",
                    "request.json",
                ],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "bead",
                    "preview-pour",
                    "--request",
                    "request.json",
                    "--json",
                ],
                true,
            ),
            (
                &["sc-compose", "bead", "pour", "--request", "request.json"],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "bead",
                    "pour",
                    "--request",
                    "request.json",
                    "--json",
                ],
                true,
            ),
            (
                &[
                    "sc-compose",
                    "report-render-many",
                    "--id",
                    "id",
                    "--glob",
                    "*.txt",
                    "--output-dir",
                    "out",
                ],
                false,
            ),
            (
                &[
                    "sc-compose",
                    "report-render-many",
                    "--id",
                    "id",
                    "--glob",
                    "*.txt",
                    "--output-dir",
                    "out",
                    "--json",
                ],
                true,
            ),
            (&["sc-compose", "report-catalog"], false),
            (&["sc-compose", "report-catalog", "--json"], true),
        ];

        for (args, expected) in cases {
            let cli = Cli::try_parse_from(*args).expect("valid CLI arguments");
            assert_eq!(
                command_wants_json(&cli.command),
                *expected,
                "unexpected JSON capability for {args:?}",
            );
        }
    }
}
