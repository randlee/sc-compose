use std::time::Instant;

use sc_composer::CompositionObserver;

use crate::CommandError;
use crate::cli::{Cli, Command, ExamplesSubcommand, TemplatesSubcommand};
use crate::commands::compose::{run_render, run_resolve, run_validate};
use crate::commands::examples::{run_examples_list, run_examples_render};
use crate::commands::extract::run_extract;
use crate::commands::reports::{
    ReportsArgs, ReportsSubcommand, run_report_catalog, run_report_render_many,
    run_reports_finalize, run_reports_index, run_reports_init, run_reports_publish_manifest,
    run_reports_render_spec, run_reports_smoke, run_reports_verify,
};
use crate::commands::sc_lint::run_sc_lint_command;
use crate::commands::template_init::{run_frontmatter_init, run_template_init};
use crate::commands::templates::{run_templates_add, run_templates_list, run_templates_render};
use crate::commands::verify::run_verify;
use crate::commands::workspace::{run_init, run_observability_health};
use crate::observer_impl::{
    CliObserver, CommandEndEvent, CommandLifecycleObserver, CommandStartEvent,
};

pub(crate) fn run(cli: Cli, observer: &mut CliObserver) -> Result<i32, CommandError> {
    match cli.command {
        Command::Lint(args) => observe_command(observer, "lint", args.json, |_observer| {
            run_sc_lint_command(&args)
        }),
        Command::Render(args) => {
            observe_command(observer, "render", args.render.json, |observer| {
                run_render(&args, observer)
            })
        }
        Command::Resolve(args) => observe_command(observer, "resolve", args.json, |observer| {
            run_resolve(&args, observer)
        }),
        Command::Validate(args) => observe_command(observer, "validate", args.json, |observer| {
            run_validate(&args, observer)
        }),
        Command::Verify(args) => observe_command(observer, "verify", args.json, |observer| {
            run_verify(&args, observer)
        }),
        Command::Extract(args) => observe_command(observer, "extract", args.json, |_observer| {
            run_extract(&args)
        }),
        Command::TemplateInit(args) => {
            observe_command(observer, "template-init", args.json, |_observer| {
                run_template_init(&args)
            })
        }
        Command::FrontmatterInit(args) => {
            observe_command(observer, "frontmatter-init", args.json, |_observer| {
                run_frontmatter_init(&args)
            })
        }
        Command::Init(args) => {
            observe_command(observer, "init", args.json, |_observer| run_init(&args))
        }
        Command::ObservabilityHealth(args) => {
            observe_command(observer, "observability-health", args.json, |observer| {
                run_observability_health(&args, observer)
            })
        }
        Command::Examples(args) => run_examples_command(&args, observer),
        Command::Templates(args) => run_templates_command(&args, observer),
        Command::Reports(args) => run_reports_command(&args, observer),
        Command::ReportRenderMany(args) => {
            observe_command(observer, "report-render-many", args.json, |_observer| {
                run_report_render_many(&args)
            })
        }
        Command::ReportCatalog(args) => {
            observe_command(observer, "report-catalog", args.json, |_observer| {
                run_report_catalog(&args)
            })
        }
    }
}

fn run_reports_command(
    args: &ReportsArgs,
    observer: &mut CliObserver,
) -> Result<i32, CommandError> {
    match &args.command {
        ReportsSubcommand::Init(init_args) => {
            observe_command(observer, "reports-init", init_args.json, |_observer| {
                run_reports_init(init_args)
            })
        }
        ReportsSubcommand::Smoke(smoke_args) => {
            observe_command(observer, "reports-smoke", smoke_args.json, |observer| {
                run_reports_smoke(smoke_args, observer)
            })
        }
        ReportsSubcommand::Finalize(finalize_args) => observe_command(
            observer,
            "reports-finalize",
            finalize_args.json,
            |_observer| run_reports_finalize(finalize_args),
        ),
        ReportsSubcommand::RenderSpec(render_args) => observe_command(
            observer,
            "reports-render-spec",
            render_args.json,
            |_observer| run_reports_render_spec(render_args),
        ),
        ReportsSubcommand::Index(index_args) => {
            observe_command(observer, "reports-index", index_args.json, |_observer| {
                run_reports_index(index_args)
            })
        }
        ReportsSubcommand::Verify(verify_args) => {
            observe_command(observer, "reports-verify", verify_args.json, |_observer| {
                run_reports_verify(verify_args)
            })
        }
        ReportsSubcommand::PublishManifest(publish_args) => observe_command(
            observer,
            "reports-publish-manifest",
            publish_args.json,
            |_observer| run_reports_publish_manifest(publish_args),
        ),
    }
}

pub(crate) fn observe_command<O>(
    observer: &mut O,
    command_name: &str,
    json_output: bool,
    action: impl FnOnce(&mut O) -> Result<i32, CommandError>,
) -> Result<i32, CommandError>
where
    O: CompositionObserver + CommandLifecycleObserver,
{
    let started = Instant::now();
    observer.on_command_start(&CommandStartEvent {
        command_name: command_name.to_owned(),
        json_output,
    });
    let result = action(observer);
    let exit_code = match &result {
        Ok(code) => *code,
        Err(error) => error.exit_code,
    };
    observer.on_command_end(&CommandEndEvent {
        command_name: command_name.to_owned(),
        exit_code,
        success: result.is_ok(),
        elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        json_output,
        diagnostic_code: result
            .as_ref()
            .err()
            .and_then(|error| error.diagnostic_code.map(|code| code.as_str().to_owned())),
        diagnostic_message: result.as_ref().err().and_then(|error| {
            error
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
        }),
    });
    result
}

fn run_examples_command(
    args: &crate::cli::ExamplesArgs,
    observer: &mut CliObserver,
) -> Result<i32, CommandError> {
    match &args.command {
        Some(ExamplesSubcommand::List(list_args)) => {
            observe_command(observer, "examples", list_args.json, |_observer| {
                run_examples_list(list_args)
            })
        }
        None => observe_command(observer, "examples", args.render.json, |observer| {
            run_examples_render(args, observer)
        }),
    }
}

fn run_templates_command(
    args: &crate::cli::TemplatesArgs,
    observer: &mut CliObserver,
) -> Result<i32, CommandError> {
    match &args.command {
        Some(TemplatesSubcommand::List(list_args)) => {
            observe_command(observer, "templates", list_args.json, |_observer| {
                run_templates_list(list_args)
            })
        }
        Some(TemplatesSubcommand::Add(add_args)) => {
            observe_command(observer, "templates", add_args.json, |_observer| {
                run_templates_add(add_args)
            })
        }
        None => observe_command(observer, "templates", args.render.json, |observer| {
            run_templates_render(args, observer)
        }),
    }
}
