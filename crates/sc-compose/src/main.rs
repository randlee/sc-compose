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
use clap::Parser;
use mimalloc::MiMalloc;
use sc_composer::Diagnostic;
use serde::Serialize;

use crate::cli::{Cli, command_wants_json};
pub(crate) use crate::command_error::CommandError;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    let cli = Cli::parse();
    let wants_json = command_wants_json(&cli.command);
    let mut observer =
        match observability::build_logger(wants_json).map(observer_impl::CliObserver::new) {
            Ok(observer) => observer,
            Err(error) => {
                if wants_json {
                    print_json_error(&error, error.diagnostics.clone());
                } else {
                    eprintln!("{error}");
                }
                std::process::exit(error.exit_code);
            }
        };
    let code = match commands::dispatch::run(cli, &mut observer) {
        Ok(code) => code,
        Err(error) => {
            if wants_json {
                print_json_error(&error, error.diagnostics.clone());
            } else {
                eprintln!("{error}");
            }
            error.exit_code
        }
    };
    observer.shutdown();
    std::process::exit(code);
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
