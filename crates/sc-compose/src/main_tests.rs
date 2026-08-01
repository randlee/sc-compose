use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use clap::Parser;
use sc_composer::{CompositionObserver, DiagnosticCode};
use sc_observability_types::{MaintenanceWorkerState, QueryHealthState};

use crate::CommandError;
use crate::cli::Cli;
use crate::commands::dispatch::observe_command;
use crate::exit_codes;
use crate::observability::build_logger_for_root;
use crate::observer_impl::{
    CliObserver, CommandEndEvent, CommandLifecycleObserver, CommandStartEvent,
};

#[derive(Default)]
struct CapturingObserver {
    started: Vec<CommandStartEvent>,
    ended: Vec<CommandEndEvent>,
}

impl CompositionObserver for CapturingObserver {}

impl CommandLifecycleObserver for CapturingObserver {
    fn on_command_start(&mut self, event: &CommandStartEvent) {
        self.started.push(event.clone());
    }

    fn on_command_end(&mut self, event: &CommandEndEvent) {
        self.ended.push(event.clone());
    }
}

#[test]
fn observe_command_emits_start_and_end_for_success() {
    let mut observer = CapturingObserver::default();
    let result = observe_command(&mut observer, "render", false, |_observer| Ok(0));
    assert_eq!(result.unwrap(), 0);
    assert_eq!(observer.started.len(), 1);
    assert_eq!(observer.ended.len(), 1);
    assert_eq!(observer.started[0].command_name, "render");
    assert!(!observer.started[0].json_output);
    assert_eq!(observer.ended[0].exit_code, 0);
    assert!(observer.ended[0].success);
}

#[test]
fn observe_command_treats_successful_nonzero_exit_as_success() {
    let mut observer = CapturingObserver::default();
    let result = observe_command(&mut observer, "validate", true, |_observer| Ok(2));
    assert_eq!(result.unwrap(), 2);
    assert_eq!(observer.started.len(), 1);
    assert_eq!(observer.ended.len(), 1);
    assert_eq!(observer.ended[0].exit_code, 2);
    assert!(observer.ended[0].success);
}

#[test]
fn observe_command_emits_start_and_end_for_failure() {
    let mut observer = CapturingObserver::default();
    let result = observe_command(&mut observer, "render", true, |_observer| {
        Err(CommandError::usage_with_code(
            anyhow!("boom"),
            DiagnosticCode::ErrRenderStdinDoubleRead,
        ))
    });
    let _ = result.unwrap_err();
    assert_eq!(observer.started.len(), 1);
    assert_eq!(observer.ended.len(), 1);
    assert!(observer.started[0].json_output);
    assert_eq!(observer.ended[0].exit_code, exit_codes::USAGE_FAIL);
    assert!(!observer.ended[0].success);
    assert_eq!(
        observer.ended[0].diagnostic_code.as_deref(),
        Some(DiagnosticCode::ErrRenderStdinDoubleRead.as_str())
    );
}

#[test]
fn build_logger_disables_console_sink_for_json_output() {
    let logger = build_logger_for_root(temp_root("logger-json"), true).expect("logger");
    let health = logger.health();
    assert_eq!(health.sink_statuses.len(), 1);
    assert_eq!(health.sink_statuses[0].name.as_str(), "jsonl-file");
}

#[test]
fn build_logger_enables_console_sink_for_text_output() {
    let logger = build_logger_for_root(temp_root("logger-text"), false).expect("logger");
    let health = logger.health();
    assert_eq!(health.sink_statuses.len(), 2);
    assert!(
        health
            .sink_statuses
            .iter()
            .any(|sink| sink.name.as_str() == "console")
    );
}

#[test]
fn build_logger_enables_retained_log_maintenance_by_default() {
    let logger = build_logger_for_root(temp_root("logger-maintenance"), false).expect("logger");
    let health = logger.health();
    assert_eq!(
        health
            .maintenance
            .expect("maintenance health present")
            .state,
        MaintenanceWorkerState::Running
    );
}

#[test]
fn shutdown_marks_query_health_unavailable() {
    let logger = build_logger_for_root(temp_root("logger-shutdown"), false).expect("logger");
    let mut observer = crate::observer_impl::CliObserver::new(logger);
    assert_eq!(
        observer.health().query.expect("query health present").state,
        QueryHealthState::Healthy
    );
    assert_eq!(
        observer
            .health()
            .maintenance
            .expect("maintenance health present")
            .state,
        MaintenanceWorkerState::Running
    );
    observer.shutdown();
    assert_eq!(
        observer.health().query.expect("query health present").state,
        QueryHealthState::Unavailable
    );
    assert_eq!(
        observer
            .health()
            .maintenance
            .expect("maintenance health present")
            .state,
        MaintenanceWorkerState::Stopped
    );
}

#[test]
fn cli_runner_preserves_logger_startup_failure_code() {
    let cli = Cli::try_parse_from(["sc-compose", "resolve", "--mode", "file"])
        .expect("test CLI arguments");
    let code = super::run_cli_with_logger(cli, |_wants_json| {
        Err(CommandError::usage(anyhow!("logger startup failed")))
    });

    assert_eq!(code, exit_codes::USAGE_FAIL);
}

#[test]
fn cli_runner_reports_text_and_json_command_errors() {
    let text_cli = Cli::try_parse_from(["sc-compose", "resolve", "--mode", "file"])
        .expect("test CLI arguments");
    let text_observer = test_observer("runner-text-error");
    let (text_code, text_observer) = super::run_cli_with_observer(text_cli, false, text_observer);
    assert_eq!(text_code, exit_codes::USAGE_FAIL);
    assert_observer_is_stopped(&text_observer);

    let json_cli = Cli::try_parse_from(["sc-compose", "resolve", "--mode", "file", "--json"])
        .expect("test CLI arguments");
    let json_observer = test_observer("runner-json-error");
    let (json_code, json_observer) = super::run_cli_with_observer(json_cli, true, json_observer);
    assert_eq!(json_code, exit_codes::USAGE_FAIL);
    assert_observer_is_stopped(&json_observer);
}

#[test]
fn cli_runner_shutdown_precedes_returning_success_code() {
    let root = temp_root("runner-success");
    let cli = Cli::try_parse_from([
        "sc-compose",
        "init",
        "--root",
        root.to_str().expect("UTF-8 temp root"),
        "--dry-run",
    ])
    .expect("test CLI arguments");
    let observer = test_observer("runner-success");
    let (code, observer) = super::run_cli_with_observer(cli, false, observer);

    assert_eq!(code, exit_codes::SUCCESS);
    assert_observer_is_stopped(&observer);
}

fn test_observer(label: &str) -> CliObserver {
    CliObserver::new(build_logger_for_root(temp_root(label), false).expect("test logger"))
}

fn assert_observer_is_stopped(observer: &CliObserver) {
    assert_eq!(
        observer
            .health()
            .maintenance
            .expect("maintenance health present")
            .state,
        MaintenanceWorkerState::Stopped
    );
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&root).expect("create temp root");
    root
}
