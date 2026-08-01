mod cli;
mod command_error;
mod commands;
mod exit_codes;
mod json_output;
#[cfg(test)]
mod main_tests;
mod observability;
mod observer_impl;
mod path_utils;
mod render_request;
mod reporting;
mod template_store;
mod var_file;

use anyhow::Result;
use mimalloc::MiMalloc;
use sc_composer::Diagnostic;
use serde::Serialize;

use crate::cli::{Cli, command_wants_json, parse_cli};
pub(crate) use crate::command_error::CommandError;
use sc_observability::Logger;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    std::process::exit(run_cli(parse_cli()));
}

fn run_cli(cli: Cli) -> i32 {
    run_cli_with_logger(cli, observability::build_logger)
}

fn run_cli_with_logger<F>(cli: Cli, build_logger: F) -> i32
where
    F: FnOnce(bool) -> Result<Logger, CommandError>,
{
    let wants_json = command_wants_json(&cli.command);
    let logger = match build_logger(wants_json) {
        Ok(logger) => logger,
        Err(error) => {
            report_error(&error, wants_json);
            return error.exit_code;
        }
    };
    let (code, _observer) =
        run_cli_with_observer(cli, wants_json, observer_impl::CliObserver::new(logger));
    code
}

fn run_cli_with_observer(
    cli: Cli,
    wants_json: bool,
    mut observer: observer_impl::CliObserver,
) -> (i32, observer_impl::CliObserver) {
    let code = CliRun {
        cli,
        observer: &mut observer,
        wants_json,
    }
    .execute();
    observer.shutdown();
    (code, observer)
}

struct CliRun<'a> {
    cli: Cli,
    observer: &'a mut observer_impl::CliObserver,
    wants_json: bool,
}

impl CliRun<'_> {
    fn execute(self) -> i32 {
        match commands::dispatch::run(self.cli, self.observer) {
            Ok(code) => code,
            Err(error) => {
                report_error(&error, self.wants_json);
                error.exit_code
            }
        }
    }
}

pub(crate) fn print_diagnostic_messages(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        println!("{}", diagnostic.message);
    }
}

pub(crate) fn print_json(
    payload: impl Serialize,
    diagnostics: Vec<sc_composer::Diagnostic>,
) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&json_output::envelope(payload, diagnostics))?
    );
    Ok(())
}

fn print_json_error(error: &CommandError, diagnostics: Vec<Diagnostic>) {
    if let Err(print_error) = print_json(serde_json::json!({}), diagnostics) {
        eprintln!("{error}");
        eprintln!("{print_error:#}");
    }
}

fn report_error(error: &CommandError, wants_json: bool) {
    if wants_json {
        print_json_error(error, error.diagnostics.clone());
    } else {
        eprintln!("{error}");
    }
}
