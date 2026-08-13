#![allow(
    dead_code,
    reason = "shared helpers are selected by separate integration-test binaries"
)]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

// macOS can report the same wall-clock timestamp for parallel test threads.
// Keep fixture paths unique within a test process so one fixture's Drop cannot
// remove another test's current directory or subprocess executable.
static TEMP_ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn temp_root(label: &str) -> PathBuf {
    temp_root_with_prefix(label, "sc-compose-test")
}

fn temp_root_with_prefix(label: &str, prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = TEMP_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{label}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    root
}

pub struct TempFixture {
    pub path: PathBuf,
}

#[derive(Clone, Copy)]
pub struct CheckedInFixture<'a> {
    pub group: &'a str,
    pub name: &'a str,
    pub target: &'a str,
}

#[derive(Clone, Copy)]
pub struct FakeCargoOptions {
    pub xwin_available: bool,
    pub test_failure: bool,
    pub fail_closed: bool,
}

impl TempFixture {
    pub fn new(label: &str) -> Self {
        Self {
            path: temp_root_with_prefix(label, "sc-compose-fixture"),
        }
    }

    pub fn from_checked_in_fixture(spec: CheckedInFixture<'_>) -> Self {
        let fixture_root = Self::new(&format!("{}-{}", spec.group, spec.name));
        let source = repo_root()
            .join("tests/fixtures/sc-lint")
            .join(spec.group)
            .join(spec.name);
        copy_directory(&source, &fixture_root.path);

        let target_dir = fixture_root.path.join(".sc/sc-lint/targets");
        fs::create_dir_all(&target_dir).unwrap();
        fs::copy(
            repo_root()
                .join(".sc/sc-lint/targets")
                .join(format!("{}.toml", spec.target)),
            target_dir.join(format!("{}.toml", spec.target)),
        )
        .unwrap();
        fixture_root
    }

    pub fn path_with_fake_tools(&self) -> String {
        let mut paths = vec![self.path.join("fake-bin")];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths)
            .expect("PATH entries")
            .to_string_lossy()
            .into_owned()
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

pub struct ScComposeCommand {
    command: Command,
    log_root: TempFixture,
}

impl ScComposeCommand {
    pub fn log_root_path(&self) -> &Path {
        &self.log_root.path
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.command.env(key, value);
        self
    }

    pub fn stdin(&mut self, cfg: Stdio) -> &mut Self {
        self.command.stdin(cfg);
        self
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.command.current_dir(dir);
        self
    }

    pub fn status(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.command.status()
    }

    pub fn output(&mut self) -> std::io::Result<std::process::Output> {
        self.command.output()
    }
}

pub fn sc_compose() -> ScComposeCommand {
    let log_root = TempFixture::new("sc-compose-logs");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sc-compose"));
    command.env("SC_LOG_ROOT", &log_root.path);
    ScComposeCommand { command, log_root }
}

pub fn sc_lint_just_root(required_files: &[&str]) -> PathBuf {
    try_sc_lint_just_root(required_files).unwrap_or_else(|| {
        panic!(
            "sc-lint Python utilities are unavailable; run the setup-sc-lint action or set SC_LINT_SOURCE_ROOT"
        )
    })
}

pub fn try_sc_lint_just_root(required_files: &[&str]) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(source_root) = std::env::var_os("SC_LINT_SOURCE_ROOT") {
        candidates.push(PathBuf::from(source_root).join(".just"));
    }
    candidates.push(repo_root().join(".just"));
    for ancestor in repo_root().ancestors() {
        candidates.push(ancestor.join("sc-lint").join(".just"));
    }

    candidates.into_iter().find(|candidate| {
        required_files
            .iter()
            .all(|file| candidate.join(file).is_file())
    })
}

pub fn materialize_sc_lint_runtime(root: &Path, required_files: &[&str]) {
    let source = sc_lint_just_root(required_files);
    materialize_sc_lint_runtime_from(root, &source, required_files, false);
}

pub fn materialize_sc_lint_runtime_with_config(root: &Path, required_files: &[&str]) {
    let source = sc_lint_just_root(required_files);
    materialize_sc_lint_runtime_from(root, &source, required_files, true);
}

fn materialize_sc_lint_runtime_from(
    root: &Path,
    source: &Path,
    required_files: &[&str],
    include_config: bool,
) {
    let destination = root.join(".just");
    fs::create_dir_all(&destination).unwrap();
    for file in required_files {
        fs::copy(source.join(file), destination.join(file))
            .unwrap_or_else(|error| panic!("materialize sc-lint utility {file}: {error}"));
    }
    if include_config {
        let config = source.join("lint-config.toml");
        if config.is_file() {
            fs::copy(config, destination.join("lint-config.toml"))
                .expect("materialize sc-lint lint config");
        }
    }
}

pub const SC_LINT_PYTHON_TOOLS: &[&str] = &[
    "lint_cargo_deny.py",
    "lint_cargo_shear.py",
    "check_version_sync.py",
    "lint_manifests.py",
    "lint_codespell.py",
    "run_pytests.py",
    "lint_sc_boundary.py",
    "lint_sc_portability.py",
    "lint_line_counts.py",
    "lint_identity_literals.py",
];

// Serialize rustc shim compilation so concurrent tests do not race on its output path.
static FAKE_CARGO_COMPILE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn write_fake_cargo(root: &Path, options: FakeCargoOptions) {
    let bin = root.join("fake-bin");
    fs::create_dir_all(&bin).expect("fake tools directory");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let xwin_code = if options.xwin_available { "0" } else { "1" };
        let test_branch = if options.test_failure {
            "if [ \"$1\" = \"test\" ]; then\n  printf '%s\\n' '{\"findings\":[{\"rule_id\":\"CI-TEST-FINDING-001\",\"path\":\"tests/fixture\",\"message\":\"workspace test failed\"}]}' >&2\n  exit 1\nfi\n"
        } else {
            ""
        };
        let fallback = if options.fail_closed {
            "exit 1"
        } else {
            "exit 0"
        };
        let cargo = bin.join("cargo");
        fs::write(
            &cargo,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"xwin\" ] && {{ [ \"$2\" = \"--version\" ] || [ \"$2\" = \"check\" ]; }}; then\n  exit {xwin_code}\nfi\n{test_branch}{fallback}\n"
            ),
        )
        .expect("fake cargo");
        let mut permissions = fs::metadata(&cargo)
            .expect("fake cargo metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(cargo, permissions).expect("fake cargo permissions");
    }

    #[cfg(windows)]
    {
        let xwin_code = if options.xwin_available { "0" } else { "1" };
        let source = bin.join("fake-cargo.rs");
        let executable = bin.join("cargo.exe");
        fs::write(
            &source,
            format!(
                "fn main() {{\n    let mut args = std::env::args().skip(1);\n    let first = args.next();\n    let second = args.next();\n    if first.as_deref() == Some(\"xwin\") && matches!(second.as_deref(), Some(\"--version\") | Some(\"check\")) {{\n        std::process::exit({xwin_code});\n    }}\n    if {test_failure} && first.as_deref() == Some(\"test\") {{\n        eprintln!(\"{{{{\\\"findings\\\":[{{{{\\\"rule_id\\\":\\\"CI-TEST-FINDING-001\\\",\\\"path\\\":\\\"tests/fixture\\\",\\\"message\\\":\\\"workspace test failed\\\"}}}}]}}}}\");\n        std::process::exit(1);\n    }}\n    std::process::exit({fallback});\n}}\n",
                xwin_code = xwin_code,
                test_failure = options.test_failure,
                fallback = i32::from(options.fail_closed),
            ),
        )
        .expect("fake cargo source");
        let _guard = FAKE_CARGO_COMPILE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("fake cargo compile lock");
        let status = Command::new("rustc")
            .args([
                "--edition",
                "2021",
                source.to_str().expect("fake cargo source path"),
                "-o",
                executable.to_str().expect("fake cargo executable path"),
            ])
            .status()
            .expect("compile fake cargo");
        assert!(status.success(), "fake cargo compilation failed: {status}");
        fs::remove_file(source).expect("remove fake cargo source");
    }
}

pub fn parse_stdout(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "sc-compose did not emit JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

pub fn assert_envelope(value: &Value) {
    assert_eq!(value["schema_version"], "1");
    assert!(value.get("payload").is_some());
    assert!(!value["payload"].is_null(), "payload must not be null");
    assert!(
        value["diagnostics"].is_array(),
        "diagnostics must be a JSON array, got: {:?}",
        value["diagnostics"]
    );
}

pub fn assert_first_code(value: &Value, code: &str) {
    assert_eq!(value["diagnostics"][0]["code"], code);
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

pub fn normalize_path_str(p: impl AsRef<Path>) -> String {
    let path = p.as_ref().to_string_lossy();
    let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
    path.replace('\\', "/")
}

pub fn write_report_catalog(root: &Path, contents: &str) {
    write_file(
        &root.join("reports").join("catalog").join("reports.toml"),
        contents,
    );
}

#[allow(
    dead_code,
    reason = "shared CLI support is only used by selected branch-local tests"
)]
pub fn valid_report_catalog() -> &'static str {
    r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#
}

pub fn write_smoke_fixture(root: &Path) {
    write_file(
        &root
            .join("reports")
            .join("smoke")
            .join("reference-template.html.j2"),
        "---\nrequired_variables:\n  - title\n  - summary\n---\n<html><body><h1>{{ title }}</h1><p>{{ summary }}</p></body></html>\n",
    );
    write_file(
        &root.join("reports").join("smoke").join("sample-vars.json"),
        "{ \"title\": \"Smoke Report\", \"summary\": \"fixture\" }\n",
    );
}

pub fn write_render_many_fixture(root: &Path) {
    write_file(
        &root.join("reports").join("templates").join("panel.html.j2"),
        "<article>{{ metadata.title }}|{{ body }}|{{ output_path }}{% if sets %}|{{ sets | join(\",\") }}{% endif %}</article>\n",
    );
}

pub fn write_report_family_override(root: &Path) {
    write_file(
        &root
            .join("reports")
            .join("templates")
            .join("lint")
            .join("report.html.j2"),
        "{% extends \"base/report.html.j2\" %}\n{% block report_header %}<header class=\"report-header report-header-lint\"><h1>Lint override</h1><p>Lint override</p></header>{% endblock %}\n{% block panel_body %}<div class=\"panel-body panel-body-lint\">Override body marker</div>{% endblock %}\n",
    );
}

pub fn write_state_machine_spec(root: &Path, relative: &str) {
    write_file(
        &root.join(relative),
        r#"[spec]
kind = "state_machine"
id = "state-diagrams"
title = "State Diagrams"
renderer_targets = ["mermaid"]

[metadata]
sets = ["publish", "diagram"]

[[states]]
id = "accepted"
label = "Accepted"

[[states]]
id = "validated"
label = "Validated"
terminal = true

[[transitions]]
from = "accepted"
to = "validated"
event = "validate_ok"
guard = "input_valid"
effect = "store message"
"#,
    );
}

pub fn write_sql_query_spec(root: &Path, relative: &str) {
    write_file(
        &root.join(relative),
        r#"[spec]
kind = "sql_query"
id = "sql-diagrams"
title = "SQL Diagrams"
renderer_targets = ["mermaid"]

[sql_query]
purpose = "Summarize shipped orders"
tables_read = ["orders", "customers"]
tables_written = ["report_cache"]
filters = ["status = shipped"]
ordering = ["created_at DESC"]
cardinality = "many"
transactional_assumptions = ["read committed"]

[metadata]
sets = ["publish", "diagram"]
"#,
    );
}

pub fn copy_directory(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_directory(&path, &target);
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(&path, &target).unwrap();
        }
    }
}

pub fn stage_phase_b_reference_assets(root: &Path) {
    copy_directory(&repo_root().join("examples"), &root.join("examples"));
    copy_directory(&repo_root().join("reports"), &root.join("reports"));
}

pub fn render_report_summary(root: &Path, vars_path: &str, output_path: &str) {
    if let Some(parent) = root.join(output_path).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(root)
        .arg("--file")
        .arg("examples/report-evidence-summary.html.j2")
        .arg("--var-file")
        .arg(root.join(vars_path))
        .arg("--output")
        .arg(root.join(output_path))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

pub fn write_large_report_evidence_vars(path: &Path) {
    let items = (0..160)
        .map(|idx| {
            serde_json::json!({
                "section_key": if idx % 2 == 0 { "evidence" } else { "diagrams" },
                "label": format!("Generated item {idx}"),
                "href": format!("reports/latest/generated/{idx}.html"),
                "status": if idx % 3 == 0 { serde_json::Value::String("latest".to_owned()) } else { serde_json::Value::Null },
                "note": format!("Generated note {}", "x".repeat(24)),
            })
        })
        .collect::<Vec<_>>();
    let payload = serde_json::json!({
        "report": {
            "title": "Large Report Evidence Summary",
            "family": "Phase B proof vehicle",
            "generated_at": "2026-05-27T03:40:00Z"
        },
        "summary": {
            "status": "PASS",
            "note": "Large payload compatibility check."
        },
        "sections": [
            { "key": "evidence", "title": "Evidence" },
            { "key": "diagrams", "title": "Diagrams" }
        ],
        "items": items
    });
    write_file(path, &serde_json::to_string_pretty(&payload).unwrap());
}

pub fn finalize_report(
    root: &Path,
    report_id: &str,
    kind: &str,
    entrypoint: &str,
    artifacts: &[&str],
) {
    let mut command = sc_compose();
    command
        .arg("reports")
        .arg("finalize")
        .arg("--root")
        .arg(root)
        .arg("--report-id")
        .arg(report_id)
        .arg("--kind")
        .arg(kind)
        .arg("--entrypoint")
        .arg(entrypoint)
        .arg("--archive");
    for artifact in artifacts {
        command.arg("--artifact").arg(artifact);
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[cfg(unix)]
pub fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
pub fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
    use std::os::windows::fs::symlink_file;

    match symlink_file(target, link) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(_) => false,
    }
}
