//! Real pinned-`bd` integration coverage.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeRequest, BeadOperation, BeadOutcome, execute_bead_request,
};
use serde_json::{Map, json};

const FIXTURES: &str = "tests/fixtures/beads";
static WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn pinned_bd_cooks_and_previews_rendered_toml_and_json_formulas() {
    let Some(bd) = std::env::var_os("BD_EXECUTABLE").map(PathBuf::from) else {
        eprintln!("skipping real Beads integration: BD_EXECUTABLE is not configured");
        return;
    };
    let root = temporary_workspace();
    initialize_beads(&bd, &root);

    for (fixture, formula_name, extension) in [
        ("toml-workflow.formula.toml.j2", "toml-workflow", "toml"),
        ("json-workflow.formula.json.j2", "json-workflow", "json"),
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
        assert!(rendered.contains("café"), "{fixture}");
        assert!(rendered.contains("Ada"), "{fixture}");
    }

    fs::remove_dir_all(root).expect("remove temporary Beads workspace");
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
