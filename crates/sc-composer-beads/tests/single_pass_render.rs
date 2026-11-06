//! Focused retained single-pass and legacy escaping regressions.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use sc_composer_beads::*;
use serde_json::{Map, json};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct NoBd;
impl ProcessRunner for NoBd {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        panic!("render must not run bd: {spec:?}")
    }
}

struct Case(PathBuf);
impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "sc-escape-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(fs::canonicalize(root).unwrap())
    }

    fn render(
        &self,
        name: &str,
        text: &str,
        vars: Map<String, serde_json::Value>,
        mode: BeadRenderBy,
    ) -> String {
        let template = self.0.join(name);
        let output = self.0.join("out.formula.toml");
        fs::write(&template, text).unwrap();
        let request = BeadComposeRequest {
            schema: BEADS_SCHEMA_V1.into(),
            operation: BeadOperation::Render,
            working_directory: self.0.clone(),
            template,
            rendered_formula: output.clone(),
            compose_variables: vars,
            formula_name: None,
            bead_variables: BTreeMap::new(),
            bd_executable: None,
            pour_authorization: None,
            parent: None,
            ref_: None,
            relations: Vec::new(),
        };
        let options = BeadExecutionOptions {
            render_by: mode,
            strict: false,
        };
        let receipt =
            execute_bead_request_with_options_and_runner(&request, &options, &NoBd).unwrap();
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:?}");
        fs::read_to_string(output).unwrap()
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn bd_mode_preserves_v17_bytes_for_formula_names() {
    let case = Case::new();
    let text = "---\nrequired_variables:\n  - y\n---\nd = \"{{{ y }}}\"\n";
    for (name, expected) in [
        ("t.formula.toml", "d = \"a\"b\\c\td/é\""),
        ("t.formula.json", "d = \"\"a\\\"b\\\\c\\td/é\"\""),
        ("t.formula.toml.j2", "d = \"a\\\"b\\\\c\\td/é\""),
        ("t.formula.json.j2", "d = \"a\\\"b\\\\c\\td/é\""),
    ] {
        assert_eq!(
            case.render(
                name,
                text,
                Map::from_iter([("y".into(), json!("a\"b\\c\td/é"))]),
                BeadRenderBy::Bd
            ),
            expected,
            "{name}"
        );
    }
}

#[test]
fn advanced_mode_retains_escaping_for_direct_formula_names() {
    let case = Case::new();
    let text = "---\nrequired_variables:\n  - y\n---\nd = \"{{{ y }}}\"\n";
    for name in ["t.formula.toml", "t.formula.json"] {
        assert_eq!(
            case.render(
                name,
                text,
                Map::from_iter([("y".into(), json!("a\"b\\c\td/é"))]),
                BeadRenderBy::ScCompose
            ),
            "d = \"a\\\"b\\\\c\\td/é\"",
            "{name}"
        );
    }
}

// Adapted from #804 06fc2e2d: bd-owned expressions and raw blocks stay literal.
#[test]
fn bd_mode_keeps_ordinary_expressions_byte_exact() {
    let case = Case::new();
    let text = "---\nrequired_variables:\n  - t\n---\na = \"{{{ t }}}\"\nb = \"{{ x | upper }}\"\nc = \"{% raw %}{{ y }}{% endraw %}\"\n";
    assert_eq!(
        case.render(
            "t.formula.toml.j2",
            text,
            Map::from_iter([("t".into(), json!("T"))]),
            BeadRenderBy::Bd
        ),
        "a = \"T\"\nb = \"{{ x | upper }}\"\nc = \"{{ y }}\""
    );
}
