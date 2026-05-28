use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use sc_composer::{CompositionObserver, DiagnosticCode};
use sc_observability_types::{MaintenanceWorkerState, QueryHealthState};

use crate::CommandError;
use crate::commands::dispatch::observe_command;
use crate::exit_codes;
use crate::observability::build_logger_for_root;
use crate::observer_impl::{CommandEndEvent, CommandLifecycleObserver, CommandStartEvent};

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

#[cfg(not(windows))]
#[test]
fn build_logger_reports_usage_error_when_current_directory_is_unavailable() {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("lock current-dir guard");
    let original_dir = std::env::current_dir().expect("current dir");
    let missing_dir = temp_root("logger-missing-cwd").join("gone");
    fs::create_dir_all(&missing_dir).expect("create missing dir");
    std::env::set_current_dir(&missing_dir).expect("enter missing dir");
    fs::remove_dir_all(&missing_dir).expect("remove current dir");
    let result = crate::observability::build_logger(false);
    std::env::set_current_dir(&original_dir).expect("restore current dir");
    let Err(error) = result else {
        panic!("logger build should fail");
    };
    assert_eq!(error.exit_code, exit_codes::USAGE_FAIL);
    assert!(format!("{error}").contains("failed to determine current directory"));
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
