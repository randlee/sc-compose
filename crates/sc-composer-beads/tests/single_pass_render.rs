//! FUZZ-171 regressions: a formula renders in one pass.
//!
//! Each case is a minimized repro from the 1.7.1 fuzz of the former two-pass
//! bead render, where the `{{ }}` pass re-parsed the `{{{ }}}` pass's output.

use sc_composer_beads::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct NoBd;

impl ProcessRunner for NoBd {
    fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
        panic!("render must not run bd: {spec:?}")
    }
}

struct Case {
    root: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "sc-bead-single-pass-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("root");
        Self {
            root: fs::canonicalize(root).expect("canonical root"),
        }
    }

    fn write(&self, name: &str, contents: &str) {
        fs::write(self.root.join(name), contents).expect("write");
    }

    /// Render `t.formula.toml.j2`; `Ok` is the output, `Err` the error code
    /// and message.
    fn render(
        &self,
        template: &str,
        compose: Value,
        bead: &[(&str, &str)],
        mode: BeadVariableRendering,
    ) -> Result<String, String> {
        self.write("t.formula.toml.j2", template);
        let Value::Object(compose_variables) = compose else {
            panic!("compose variables are an object")
        };
        let request = BeadComposeRequest {
            schema: BEADS_SCHEMA_V1.into(),
            operation: BeadOperation::Render,
            working_directory: self.root.clone(),
            template: self.root.join("t.formula.toml.j2"),
            rendered_formula: self.root.join("t.formula.toml"),
            compose_variables,
            formula_name: None,
            bead_variables: bead
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect::<BTreeMap<_, _>>(),
            bead_variable_rendering: mode,
            bd_executable: None,
            pour_authorization: None,
            parent: None,
            ref_: None,
            relations: Vec::new(),
        };
        match execute_bead_request_with_runner(&request, &NoBd) {
            Err(error) => Err(format!("{}: {error}", error.code())),
            Ok(receipt) => match receipt.outcome {
                BeadOutcome::Failed { code } => Err(format!(
                    "{code}: {}",
                    receipt.stages.last().expect("stage").stderr_excerpt
                )),
                _ => Ok(fs::read_to_string(&request.rendered_formula).expect("output")),
            },
        }
    }
}

fn render(template: &str, compose: Value, bead: &[(&str, &str)]) -> Result<String, String> {
    Case::new().render(template, compose, bead, BeadVariableRendering::ScCompose)
}

fn fm(names: &[&str]) -> String {
    let mut text = String::from("---\nrequired_variables:\n");
    for name in names {
        text.push_str("  - ");
        text.push_str(name);
        text.push('\n');
    }
    text.push_str("---\n");
    text
}

// FUZZ-171-01: a raw block stays literal; nothing renders its body later.
#[test]
fn fuzz_171_01_raw_blocks_stay_literal() {
    assert_eq!(
        render("y = \"{% raw %}{{ n }}{% endraw %}\"\n", json!({}), &[]),
        Ok("y = \"{{ n }}\"".into())
    );
    assert_eq!(
        render(
            &(fm(&["z"]) + "y = \"{% raw %}{{ z }}{% endraw %}\"\n"),
            json!({}),
            &[("z", "VAL")]
        ),
        Ok("y = \"{{ z }}\"".into())
    );
    assert_eq!(
        render("y = \"{% raw %}{{{ z }}}{% endraw %}\"\n", json!({}), &[]),
        Ok("y = \"{{{ z }}}\"".into())
    );
}

// FUZZ-171-02: braces an expression outputs are text, never evaluated.
#[test]
fn fuzz_171_02_output_braces_are_never_evaluated() {
    assert_eq!(
        render(
            "y = \"{{{ '{' ~ '{ z }' ~ '}' }}}\"\n",
            json!({"z": "SECRET"}),
            &[]
        ),
        Ok("y = \"{{ z }}\"".into())
    );
}

// FUZZ-171-03: set and loop locals win over caller globals of the same name.
#[test]
fn fuzz_171_03_locals_shadow_caller_globals() {
    assert_eq!(
        render(
            "{% set z = \"W\" %}y = \"{{ z }}\"\n",
            json!({"z": "SECRET"}),
            &[]
        ),
        Ok("y = \"W\"".into())
    );
    assert_eq!(
        render(
            &(fm(&["xs"]) + "y = \"{% for v in xs %}{{ v }}{% endfor %}\"\n"),
            json!({"xs": ["a", "b"], "v": "SECRET"}),
            &[]
        ),
        Ok("y = \"ab\"".into())
    );
}

// FUZZ-171-04: private-use characters are ordinary text (no marker pool).
#[test]
fn fuzz_171_04_every_private_use_character_renders() {
    let all: String = (0xE000_u32..0xF900)
        .map(|code| char::from_u32(code).expect("private use"))
        .collect();
    assert_eq!(
        render(
            &(fm(&["v"]) + "n = {{{ v|length }}}\n"),
            json!({ "v": all }),
            &[]
        ),
        Ok("n = 6400".into())
    );
    assert_eq!(
        render(&format!("x = \"{all}\"\n"), json!({}), &[]),
        Ok(format!("x = \"{all}\""))
    );
}

// FUZZ-171-05: object keys are values; their braces are never syntax.
#[test]
fn fuzz_171_05_object_keys_stay_literal() {
    assert_eq!(
        render(
            &(fm(&["o", "s"]) + "{% for k, x in o|items %}k = \"{{{ k }}}\"{% endfor %}\n"),
            json!({"o": {"{{ s }}": 1}, "s": "LEAK"}),
            &[]
        ),
        Ok("k = \"{{ s }}\"".into())
    );
    assert_eq!(
        render(
            &(fm(&["o"]) + "k = \"{{{ o|list|first }}}\"\n"),
            json!({"o": {"{%y": 1}}),
            &[]
        ),
        Ok("k = \"{%y\"".into())
    );
}

// FUZZ-171-06: adjacent outputs cannot form a comment or statement opener.
#[test]
fn fuzz_171_06_adjacent_outputs_cannot_form_delimiters() {
    for value in ["#", "%"] {
        assert_eq!(
            render(
                &(fm(&["v"]) + "k = \"{{{ '{' }}}{{{ v }}}\"\n"),
                json!({ "v": value }),
                &[]
            ),
            Ok(format!("k = \"{{{value}\""))
        );
    }
}

// FUZZ-171-07: a list prints its braces, not an escaped marker.
#[test]
fn fuzz_171_07_lists_print_literal_braces() {
    let rendered = render(
        &(fm(&["v"]) + "k = \"{{{ v }}}\"\n"),
        json!({"v": ["{"]}),
        &[],
    )
    .expect("render");
    assert!(!rendered.contains("\\u{"), "{rendered}");
    assert_eq!(rendered, "k = \"[\\\"{\\\"]\"");
}

// FUZZ-171-08: a genuine U+E000 is not turned into `{`. `pad` holds every
// other private-use character, which left U+E000 as the old marker.
#[test]
fn fuzz_171_08_private_use_text_is_not_restored_to_a_brace() {
    let pad: String = (0xE001_u32..0xF900)
        .map(|code| char::from_u32(code).expect("private use"))
        .collect();
    assert_eq!(
        render(
            "k = \"{{{ '\\ue000' }}}\"\n",
            json!({ "pad": pad.clone() }),
            &[]
        ),
        Ok("k = \"\u{e000}\"".into())
    );
    assert_eq!(
        render(
            &(fm(&["o"]) + "k = \"{{{ o|list|first }}}\"\n"),
            json!({"o": {"\u{e000}": 1}, "pad": pad}),
            &[]
        ),
        Ok("k = \"\u{e000}\"".into())
    );
}

// FUZZ-171-09: filters see `{`, not a marker.
#[test]
fn fuzz_171_09_filters_see_literal_braces() {
    let cases = [
        ("{{{ v|sort|join }}}", json!({"v": ["|", "{"]}), "{|"),
        ("{{{ v|max }}}", json!({"v": ["|", "{"]}), "|"),
        ("{{{ v|title }}}", json!({"v": "{é"}), "{É"),
    ];
    for (expression, compose, expected) in cases {
        assert_eq!(
            render(&(fm(&["v"]) + "k = \"" + expression + "\"\n"), compose, &[]),
            Ok(format!("k = \"{expected}\"")),
            "{expression}"
        );
    }
    assert_eq!(
        render(
            &(fm(&["b"]) + "k = \"{{ b|list|sort|join }}\"\n"),
            json!({}),
            &[("b", "|{")]
        ),
        Ok("k = \"{|\"".into())
    );
}

// FUZZ-171-11: an included fragment's frontmatter defaults apply.
#[test]
fn fuzz_171_11_include_defaults_reach_the_render_context() {
    let case = Case::new();
    case.write(
        "inc.md",
        "---\ndefaults: {\"g\": \"G\"}\n---\ng = \"{{{ g }}}\"\n",
    );
    assert_eq!(
        case.render(
            "@<inc.md>\n",
            json!({}),
            &[],
            BeadVariableRendering::ScCompose
        ),
        Ok("g = \"G\"".into())
    );
}

// FUZZ-171-12: one render drops one trailing newline, exactly as v1.7.0 did,
// in both modes.
#[test]
fn fuzz_171_12_trailing_newlines_match_a_single_render() {
    for mode in [BeadVariableRendering::ScCompose, BeadVariableRendering::Bd] {
        assert_eq!(
            Case::new().render("x = 1\n\n", json!({}), &[], mode),
            Ok("x = 1\n".into()),
            "{mode:?}"
        );
    }
}

// FUZZ-171-14: a name supplied as both a compose and a Beads variable has no
// single value and is refused.
#[test]
fn fuzz_171_14_a_name_in_both_variable_maps_is_refused() {
    let error = render(
        &(fm(&["v"]) + "b = \"{{ v }}\"\n"),
        json!({"v": "FROMCOMPOSE"}),
        &[("v", "FROMBEAD")],
    )
    .expect_err("duplicate name");
    assert!(
        error.starts_with("BEADS_VARIABLE_KEY_DUPLICATE") && error.contains('v'),
        "{error}"
    );
}

// bd mode: ordinary expressions reach bd byte-for-byte, raw blocks included.
#[test]
fn bd_mode_keeps_ordinary_expressions_byte_exact() {
    assert_eq!(
        Case::new().render(
            &(fm(&["t"]) + "a = \"{{{ t }}}\"\nb = \"{{ x | upper }}\"\nc = \"{% raw %}{{ y }}{% endraw %}\"\n"),
            json!({"t": "T"}),
            &[],
            BeadVariableRendering::Bd
        ),
        Ok("a = \"T\"\nb = \"{{ x | upper }}\"\nc = \"{{ y }}\"".into())
    );
}

// Both expression forms are validated in one strict scan.
#[test]
fn both_expression_forms_are_validated_strictly() {
    let error =
        render("a = \"{{{ x }}}\"\nb = \"{{ y }}\"\n", json!({}), &[]).expect_err("undeclared");
    assert!(
        error.contains("undeclared referenced token: x")
            && error.contains("undeclared referenced token: y"),
        "{error}"
    );
}
