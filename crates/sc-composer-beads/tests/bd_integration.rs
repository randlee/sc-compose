//! Real pinned-`bd` integration coverage.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeRequest, BeadOperation, BeadOutcome, execute_bead_request,
};
use serde_json::{Map, json};

const FIXTURES: &str = "tests/fixtures/beads";
static WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn pinned_bd_cooks_and_previews_rendered_toml_and_json_formulas() {
    let Some(pinned_bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        eprintln!("skipping real Beads integration: BD_EXECUTABLE is not configured");
        return;
    };
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

fn remove_temporary_workspace(root: &Path) -> io::Result<()> {
    #[cfg(windows)]
    const ATTEMPTS: usize = 10;
    #[cfg(not(windows))]
    const ATTEMPTS: usize = 1;

    remove_dir_all_with_retry(root, ATTEMPTS, Duration::from_millis(100), |path| {
        fs::remove_dir_all(path)
    })
}

fn remove_dir_all_with_retry<F>(
    root: &Path,
    attempts: usize,
    retry_delay: Duration,
    mut remove_dir_all: F,
) -> io::Result<()>
where
    F: FnMut(&Path) -> io::Result<()>,
{
    debug_assert!(attempts > 0, "cleanup must make at least one attempt");
    let mut last_error = None;

    for attempt in 0..attempts {
        match remove_dir_all(root) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        if attempt + 1 < attempts {
            thread::sleep(retry_delay);
        }
    }

    Err(last_error.expect("cleanup attempted at least once"))
}

#[test]
fn cleanup_retries_transient_file_locks() {
    let mut calls = 0;
    remove_dir_all_with_retry(Path::new("temporary-workspace"), 3, Duration::ZERO, |_| {
        calls += 1;
        if calls < 3 {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            Ok(())
        }
    })
    .expect("transient cleanup failure is retried");
    assert_eq!(calls, 3);
}

#[cfg(unix)]
fn isolated_bd(root: &Path, bd: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let wrapper = root.join("bd-no-daemon");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nexport BEADS_NO_DAEMON=1\nexec {} \"$@\"\n",
            shell_quote(bd)
        ),
    )
    .expect("write no-daemon Beads wrapper");
    let mut permissions = fs::metadata(&wrapper)
        .expect("no-daemon wrapper metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&wrapper, permissions).expect("make no-daemon wrapper executable");
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
    use std::os::unix::fs::PermissionsExt;

    let probe = root.join("bd-process-probe");
    fs::write(
        &probe,
        format!(
            "#!/bin/sh\nprintf 'started\\n' > {}\nexec {} \"$@\"\n",
            shell_quote(marker),
            shell_quote(bd)
        ),
    )
    .expect("write process probe");
    let mut permissions = fs::metadata(&probe).expect("probe metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&probe, permissions).expect("make process probe executable");
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
        formula_name: Some(formula_name.to_owned()),
        bead_variables: BTreeMap::from([(String::from("release_name"), String::from("1.5.0"))]),
        bd_executable: Some(bd.into()),
        pour_authorization: None,
    }
}

fn initialize_beads(bd: &Path, root: &Path) {
    let output = Command::new(bd)
        .args([
            "init",
            "--non-interactive",
            "--quiet",
            "--skip-agents",
            "--skip-hooks",
        ])
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
