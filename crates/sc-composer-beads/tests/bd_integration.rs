//! Real pinned-`bd` integration coverage.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
#[path = "../../test-support/fake_exec.rs"]
mod fake_exec;

use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeRequest, BeadExecutionOptions, BeadOperation, BeadOutcome,
    BeadRenderBy, PourAuthorization, execute_bead_request_with_options,
};
use serde_json::{Map, json};

const FIXTURES: &str = "tests/fixtures/beads";
static WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
// The pinned `bd` executable is a single mutable external tool installation.
// Run its real integration scenarios serially so concurrent `bd init` calls
// cannot contend with the executable or its shared tool state in CI.
static PINNED_BD_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn pinned_bd_cooks_and_previews_rendered_toml_and_json_formulas() {
    let Some(pinned_bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let _guard = lock_pinned_bd_scenarios();
    let root = temporary_workspace();
    let bd = isolated_bd(&root, &pinned_bd);
    initialize_beads(&bd, &root);

    for (fixture, formula_name, extension, has_markdown_value) in [
        (
            "toml-workflow.formula.toml.j2",
            "toml-workflow",
            "toml",
            true,
        ),
        (
            "json-workflow.formula.json.j2",
            "json-workflow",
            "json",
            false,
        ),
    ] {
        let output = root
            .join(".beads")
            .join("formulas")
            .join(format!("{formula_name}.formula.{extension}"));
        let template = root.join("templates").join(fixture);
        copy_fixture(fixture, &template);
        let receipt = execute_bead_request(&request(
            &root,
            &template,
            output.clone(),
            formula_name,
            &bd,
        ))
        .expect("valid Beads request");
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Succeeded,
            "{fixture}: {receipt:#?}"
        );
        assert_eq!(receipt.stages.len(), 4, "{fixture}");
        let rendered = fs::read_to_string(output).expect("rendered formula");
        assert!(rendered.contains("{{ release_name }}"), "{fixture}");
        if has_markdown_value {
            assert!(rendered.contains("café"), "{fixture}");
            assert!(
                rendered.contains("multiline Markdown evidence"),
                "{fixture}"
            );
        }
        assert!(rendered.contains("Ada"), "{fixture}");
    }

    missing_executable_is_reported_before_any_real_beads_stage(&root);
    unauthorized_pour_does_not_start_the_pinned_beads_binary(&root, &bd);
    invalid_formula_fails_cook(&root, &bd);
    redirected_registry_is_resolved_by_bd_where(&root, &bd);
    same_name_extension_shadowing_blocks_preview(&root, &bd);

    remove_temporary_workspace(&root).expect("remove temporary Beads workspace");
}

#[test]
#[ignore = "real bd: run with scripts/test_real_bd.py (needs BD_EXECUTABLE)"]
fn pinned_bd_missing_required_release_name_returns_a_failure_receipt() {
    let Some(pinned_bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        panic!(
            "real-bd test run without BD_EXECUTABLE: refusing a vacuous pass (run via scripts/test_real_bd.py with a pinned bd)"
        );
    };
    let _guard = lock_pinned_bd_scenarios();
    let root = temporary_workspace();
    let bd = isolated_bd(&root, &pinned_bd);
    initialize_beads(&bd, &root);
    let template = root
        .join("templates")
        .join("missing-release-name.formula.toml.j2");
    fs::create_dir_all(template.parent().expect("template parent")).expect("create template dir");
    fs::write(
        &template,
        "formula = \"missing-release-name\"\ndescription = \"Missing runtime variable proof\"\nversion = 1\ntype = \"workflow\"\n\n[vars.release_name]\nrequired = true\n\n[[steps]]\nid = \"proof\"\ntitle = \"Release {{release_name}}\"\n",
    )
    .expect("write required-variable template");
    let output = root
        .join(".beads")
        .join("formulas")
        .join("missing-release-name.formula.toml");
    let mut missing_variable = request(&root, &template, output, "missing-release-name", &bd);
    // `bd` v1.3.1 permits unresolved placeholders for cook and dry-run pour.
    // The real missing-required-variable rejection occurs before a persistent
    // pour; this isolated temporary registry prevents any lasting state.
    missing_variable.operation = BeadOperation::Pour;
    missing_variable.bead_variables.clear();
    missing_variable.pour_authorization = Some(PourAuthorization::CreatePersistentBeads);

    let receipt = execute_bead_request(&missing_variable)
        .expect("a required Beads variable failure must return a receipt");

    assert_eq!(
        receipt.outcome,
        BeadOutcome::Failed {
            code: "BEADS_POUR_FAILED".to_owned()
        },
        "pinned bd must reject a formula whose required release_name was not supplied: {receipt:#?}"
    );
    assert_eq!(receipt.stages.len(), 4, "{receipt:#?}");
    remove_temporary_workspace(&root).expect("remove temporary Beads workspace");
}

fn execute_bead_request(
    request: &BeadComposeRequest,
) -> Result<sc_composer_beads::BeadComposeReceipt, sc_composer_beads::BeadComposeError> {
    execute_bead_request_with_options(
        request,
        &BeadExecutionOptions {
            render_by: BeadRenderBy::Bd,
            strict: false,
        },
    )
}

fn remove_temporary_workspace(root: &Path) -> io::Result<()> {
    cleanup_once(root, |path| fs::remove_dir_all(path))
}

fn cleanup_once(root: &Path, remove: impl FnOnce(&Path) -> io::Result<()>) -> io::Result<()> {
    match remove(root) {
        // A detached Windows file handle can defer cleanup; do not make the
        // behavior test depend on how soon the OS releases it.
        Err(error) if error.raw_os_error() == Some(32) => {
            eprintln!("temporary workspace cleanup deferred: {}", root.display());
            Ok(())
        }
        result => result,
    }
}

#[test]
fn cleanup_does_not_depend_on_lock_release_timing() {
    let mut calls = 0;
    cleanup_once(Path::new("temporary-workspace"), |_| {
        calls += 1;
        Err(io::Error::from_raw_os_error(32))
    })
    .unwrap();
    assert_eq!(calls, 1);
    let error = cleanup_once(Path::new("temporary-workspace"), |_| {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    })
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}

// FTQ-001: a fork on another libtest thread can inherit a just-written
// script's write descriptor. Holding such a descriptor open reproduces that
// interleaving deterministically instead of waiting for it.
#[cfg(unix)]
#[test]
fn fake_executable_runs_while_its_written_script_is_held_open_for_writing() {
    let root = temporary_workspace();
    let fake = root.join("fake-bd");
    fake_exec::install(&fake, "printf '%s' \"ran:$1\"\n");
    let _inherited_writer = fs::OpenOptions::new()
        .append(true)
        .open(root.join("fake-bd.sh"))
        .expect("hold the script's write descriptor open");
    let output = Command::new(&fake)
        .arg("cook")
        .output()
        .expect("spawn fake executable");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"ran:cook");
    remove_temporary_workspace(&root).expect("remove temporary workspace");
}

// Control for the test above: the replaced write-then-exec pattern fails with
// ETXTBSY under the same held descriptor. Linux enforces ETXTBSY on exec;
// macOS does not, so the control runs on Linux only.
#[cfg(target_os = "linux")]
#[test]
fn exec_of_a_script_held_open_for_writing_fails_with_etxtbsy() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    const ETXTBSY: i32 = 26;
    let root = temporary_workspace();
    let script = root.join("written-then-exec");
    let mut writer = fs::File::create(&script).expect("create script");
    writer
        .write_all(b"#!/bin/sh\nexit 0\n")
        .expect("write script");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("chmod script");
    let error = Command::new(&script)
        .status()
        .expect_err("exec must fail while a write descriptor is open");
    assert_eq!(error.raw_os_error(), Some(ETXTBSY), "{error}");
    drop(writer);
    remove_temporary_workspace(&root).expect("remove temporary workspace");
}

#[cfg(unix)]
fn isolated_bd(root: &Path, bd: &Path) -> PathBuf {
    let wrapper = root.join("bd-no-daemon");
    fake_exec::install(
        &wrapper,
        &format!(
            "#!/bin/sh\nexport BEADS_NO_DAEMON=1\nexec {} \"$@\"\n",
            shell_quote(bd)
        ),
    );
    wrapper
}

#[cfg(windows)]
fn isolated_bd(root: &Path, bd: &Path) -> PathBuf {
    let wrapper = root.join("bd-no-daemon.cmd");
    fs::write(
        &wrapper,
        format!(
            "@echo off\r\nsetlocal\r\nset \"BEADS_NO_DAEMON=1\"\r\n\"{}\" %*\r\nexit /b %ERRORLEVEL%\r\n",
            bd.display()
        ),
    )
    .expect("write no-daemon Beads wrapper");
    wrapper
}

fn unauthorized_pour_does_not_start_the_pinned_beads_binary(root: &Path, bd: &Path) {
    let marker = root.join("unauthorized-pour-started");
    let probe = process_probe(root, bd, &marker);
    let template = root.join("templates").join("toml-workflow.formula.toml.j2");
    let output = root
        .join(".beads")
        .join("formulas")
        .join("unauthorized-pour.formula.toml");
    let mut request = request(root, &template, output, "unauthorized-pour", &probe);
    request.operation = BeadOperation::Pour;

    let error = execute_bead_request(&request)
        .expect_err("persistent pour without authorization must be refused");
    assert_eq!(error.code(), "BEADS_POUR_AUTH_REQUIRED");
    assert!(
        !marker.exists(),
        "the process probe delegates to pinned bd only if composition tried to spawn it"
    );
}

#[cfg(unix)]
fn process_probe(root: &Path, bd: &Path, marker: &Path) -> PathBuf {
    let probe = root.join("bd-process-probe");
    fake_exec::install(
        &probe,
        &format!(
            "#!/bin/sh\nprintf 'started\\n' > {}\nexec {} \"$@\"\n",
            shell_quote(marker),
            shell_quote(bd)
        ),
    );
    probe
}

#[cfg(unix)]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\\"'\\\"'"))
}

#[cfg(windows)]
fn process_probe(root: &Path, bd: &Path, marker: &Path) -> PathBuf {
    let probe = root.join("bd-process-probe.cmd");
    fs::write(
        &probe,
        format!(
            "@echo off\r\necho started>\"{}\"\r\n\"{}\" %*\r\n",
            marker.display(),
            bd.display()
        ),
    )
    .expect("write process probe");
    probe
}

fn missing_executable_is_reported_before_any_real_beads_stage(root: &Path) {
    let template = root.join("templates").join("toml-workflow.formula.toml.j2");
    let output = root
        .join(".beads")
        .join("formulas")
        .join("unavailable.formula.toml");
    let error = execute_bead_request(&request(
        root,
        &template,
        output,
        "unavailable",
        Path::new("missing-bd-executable"),
    ))
    .expect_err("a missing executable must not be treated as a process failure receipt");
    assert_eq!(error.code(), "BEADS_BD_UNAVAILABLE");
}

fn invalid_formula_fails_cook(root: &Path, bd: &Path) {
    let invalid_template = root.join("templates").join("invalid.formula.toml.j2");
    fs::write(&invalid_template, "formula = ").expect("write malformed TOML template");
    let invalid_output = root
        .join(".beads")
        .join("formulas")
        .join("invalid.formula.toml");
    let invalid = execute_bead_request(&request(
        root,
        &invalid_template,
        invalid_output,
        "invalid",
        bd,
    ))
    .expect("invalid formula creates a failure receipt");
    assert_eq!(
        invalid.outcome,
        BeadOutcome::Failed {
            code: "BEADS_COOK_FAILED".to_owned()
        }
    );
}

fn redirected_registry_is_resolved_by_bd_where(root: &Path, bd: &Path) {
    let redirected_worktree = root.join("redirected-worktree");
    let redirect_dir = redirected_worktree.join(".beads");
    fs::create_dir_all(&redirect_dir).expect("create redirect directory");
    fs::write(
        redirect_dir.join("redirect"),
        format!("{}\n", root.join(".beads").display()),
    )
    .expect("write Beads redirect");
    let template = redirected_worktree
        .join("templates")
        .join("redirect-workflow.formula.toml.j2");
    fs::create_dir_all(template.parent().expect("template parent"))
        .expect("create template parent");
    fs::write(
        &template,
        "formula = \"redirect-workflow\"\ndescription = \"redirect proof\"\nversion = 1\ntype = \"workflow\"\n\n[[steps]]\nid = \"step\"\ntitle = \"Redirect {{ release_name }}\"\n",
    )
    .expect("write redirect template");
    let output = root
        .join(".beads")
        .join("formulas")
        .join("redirect-workflow.formula.toml");
    let receipt = execute_bead_request(&request(
        &redirected_worktree,
        &template,
        output,
        "redirect-workflow",
        bd,
    ))
    .expect("redirected registry request");
    assert_eq!(receipt.outcome, BeadOutcome::Succeeded);
}

fn same_name_extension_shadowing_blocks_preview(root: &Path, bd: &Path) {
    let shadow = root
        .join(".beads")
        .join("formulas")
        .join("toml-workflow.formula.json");
    fs::write(
        &shadow,
        r#"{"formula":"toml-workflow","version":1,"type":"workflow","steps":[]}"#,
    )
    .expect("write shadow formula");
    let template = root.join("templates").join("toml-workflow.formula.toml.j2");
    let output = root
        .join(".beads")
        .join("formulas")
        .join("toml-workflow.formula.toml");
    let receipt = execute_bead_request(&request(root, &template, output, "toml-workflow", bd))
        .expect("shadowing creates a failure receipt");
    assert_eq!(
        receipt.outcome,
        BeadOutcome::Failed {
            code: "BEADS_FORMULA_REGISTRY_AMBIGUOUS".to_owned()
        }
    );
    assert_eq!(receipt.stages.len(), 3);
}

fn request(
    root: &Path,
    template: &Path,
    rendered_formula: PathBuf,
    formula_name: &str,
    bd: &Path,
) -> BeadComposeRequest {
    BeadComposeRequest {
        schema: BEADS_SCHEMA_V1.to_owned(),
        operation: BeadOperation::PreviewPour,
        working_directory: root.into(),
        template: template.into(),
        rendered_formula,
        compose_variables: Map::from_iter([
            (
                "project".to_owned(),
                json!({
                    "name": "sc-compose",
                    "notes": "Unicode café\\nmultiline Markdown evidence"
                }),
            ),
            (
                "reviewers".to_owned(),
                json!([
                    { "id": "ada", "name": "Ada" },
                    { "id": "lin", "name": "Lin" }
                ]),
            ),
        ]),
        formula_name: Some(
            sc_composer_beads::FormulaName::new(formula_name).expect("formula name"),
        ),
        bead_variables: BTreeMap::from([(String::from("release_name"), String::from("1.5.0"))]),
        bd_executable: Some(bd.into()),
        pour_authorization: None,
        parent: None,
        ref_: None,
        relations: Vec::new(),
    }
}

fn initialize_beads(bd: &Path, root: &Path) {
    // bd discovers a workspace by walking up from the working directory, so an
    // ancestor `.beads` (for example `$HOME/.beads` when the temp dir lives under
    // `$HOME`) would otherwise receive this init and later `bd where`. Pin only
    // the init child to this workspace; the parent process environment is not
    // touched, and afterwards the workspace's own `.beads` is the nearest one.
    fs::create_dir_all(root.join(".beads")).expect("create workspace .beads");
    let output = Command::new(bd)
        .args([
            "init",
            "--non-interactive",
            "--quiet",
            "--skip-agents",
            "--skip-hooks",
        ])
        .env("BEADS_DIR", root.join(".beads"))
        .current_dir(root)
        .output()
        .expect("start pinned bd init");
    assert!(
        output.status.success(),
        "bd init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::create_dir_all(root.join(".beads").join("formulas")).expect("create formula registry");
}

fn lock_pinned_bd_scenarios() -> std::sync::MutexGuard<'static, ()> {
    PINNED_BD_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn copy_fixture(name: &str, destination: &Path) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES)
        .join(name);
    fs::create_dir_all(destination.parent().expect("template parent"))
        .expect("create template dir");
    fs::copy(source, destination).expect("copy fixture");
}

fn temporary_workspace() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = WORKSPACE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "sc-composer-beads-integration-{}-{unique}-{sequence}",
        std::process::id(),
    ));
    fs::create_dir_all(&root).expect("create temporary Beads workspace");
    root
}
