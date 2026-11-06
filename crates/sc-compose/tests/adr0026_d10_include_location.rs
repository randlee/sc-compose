//! Errors raised inside included content name the included file, its own
//! line and the include chain, in text and JSON output (ADR-0026 D10).
#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};

use serde_json::Value;
use support::{sc_compose, temp_root, write_file};

struct Run {
    code: Option<i32>,
    text: String,
    json: Value,
}

fn run(root: &Path, command: &str, file: &str, extra: &[&str]) -> Run {
    let mut text_cli = sc_compose();
    text_cli
        .args([command, "--mode", "file", "--root"])
        .arg(root)
        .args(["--file", file])
        .args(extra);
    let text_output = text_cli.output().unwrap();
    let mut json_cli = sc_compose();
    json_cli
        .args([command, "--mode", "file", "--root"])
        .arg(root)
        .args(["--file", file, "--json"])
        .args(extra);
    let json_output = json_cli.output().unwrap();
    assert_eq!(text_output.status.code(), json_output.status.code());
    Run {
        code: text_output.status.code(),
        text: format!(
            "{}{}",
            String::from_utf8_lossy(&text_output.stdout),
            String::from_utf8_lossy(&text_output.stderr)
        ),
        json: serde_json::from_slice(&json_output.stdout).unwrap(),
    }
}

/// The single error-severity diagnostic (validation may add warnings).
fn only_diagnostic(run: &Run) -> &Value {
    let errors = run.json["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
        .collect::<Vec<_>>();
    assert_eq!(errors.len(), 1, "{}", run.json);
    errors[0]
}

fn chain(root: &Path, files: &[&str]) -> Value {
    Value::from(
        files
            .iter()
            .map(|file| root.join(file).display().to_string())
            .collect::<Vec<_>>(),
    )
}

fn canonical_root(label: &str) -> PathBuf {
    temp_root(label).canonicalize().unwrap()
}

/// Assert the location of the one diagnostic and the text-mode error line.
fn assert_located(
    run: &Run,
    root: &Path,
    file: &str,
    line: u64,
    column: Option<u64>,
    files: &[&str],
) {
    assert_eq!(run.code, Some(2), "{}", run.text);
    let diagnostic = only_diagnostic(run);
    assert_eq!(diagnostic["code"], "ERR_RENDER_TEMPLATE", "{diagnostic}");
    assert_eq!(diagnostic["path"], root.join(file).display().to_string());
    assert_eq!(diagnostic["line"], line, "{diagnostic}");
    assert_eq!(
        diagnostic["column"],
        column.map_or(Value::Null, Value::from)
    );
    assert_eq!(diagnostic["include_chain"], chain(root, files));
    let location = format!("(in {file}:{line})");
    assert!(
        diagnostic["message"].as_str().unwrap().contains(&location),
        "{diagnostic}"
    );
    assert!(run.text.contains(&location), "{}", run.text);
}

#[test]
fn error_in_direct_child_names_child_line_and_chain() {
    let root = canonical_root("adr0026-d10-direct");
    write_file(
        &root.join("main.md"),
        "---\nrequired_variables: []\n---\nline one\n@<child.md>\nafter\n",
    );
    write_file(&root.join("child.md"), "c1\n{{ bad syntax }}\nc3\n");
    for command in ["render", "validate"] {
        let extra: &[&str] = if command == "validate" {
            &["--check-render"]
        } else {
            &[]
        };
        let run = run(&root, command, "main.md", extra);
        assert_located(
            &run,
            &root,
            "child.md",
            2,
            Some(8),
            &["main.md", "child.md"],
        );
        if command == "render" {
            let chain_text = format!(
                "include_chain={} -> {}",
                root.join("main.md").display(),
                root.join("child.md").display()
            );
            assert!(run.text.contains(&chain_text), "{}", run.text);
        }
    }
}

#[test]
fn error_in_nested_grandchild_names_grandchild_and_full_chain() {
    let root = canonical_root("adr0026-d10-nested");
    write_file(&root.join("top.md"), "top\n@<parts/mid.md>\nend\n");
    write_file(&root.join("parts/mid.md"), "m1\nm2\n@<leaf.md>\n");
    write_file(&root.join("parts/leaf.md"), "g1\ng2\n{% if %}\n");
    let run = run(&root, "render", "top.md", &[]);
    assert_located(
        &run,
        &root,
        "parts/leaf.md",
        3,
        Some(7),
        &["top.md", "parts/mid.md", "parts/leaf.md"],
    );
}

#[test]
fn error_in_parent_counts_frontmatter_lines() {
    let root = canonical_root("adr0026-d10-parent");
    write_file(&root.join("child.md"), "fine\n");
    write_file(
        &root.join("main.md"),
        "---\nrequired_variables: []\n---\n@<child.md>\nok\n{{ bad syntax }}\n",
    );
    let run = run(&root, "render", "main.md", &[]);
    assert_located(&run, &root, "main.md", 6, Some(8), &["main.md"]);
    // Without includes the chain is the template alone; text output adds none.
    assert!(!run.text.contains("include_chain="), "{}", run.text);
}

#[test]
fn error_in_crlf_child_counts_crlf_lines() {
    let root = canonical_root("adr0026-d10-crlf");
    write_file(&root.join("main.md"), "top\n@<crlf.md>\n");
    write_file(&root.join("crlf.md"), "c1\r\nc2\r\n{{ bad syntax }}\r\n");
    let run = run(&root, "render", "main.md", &[]);
    assert_located(&run, &root, "crlf.md", 3, Some(8), &["main.md", "crlf.md"]);
}

#[test]
fn undefined_variable_in_child_names_child_line() {
    let root = canonical_root("adr0026-d10-undefined");
    write_file(
        &root.join("main.md"),
        "---\nrequired_variables: []\n---\nm1\n@<child.md>\n",
    );
    write_file(&root.join("child.md"), "c1\n{{ missing.field }}\n");
    let run = run(&root, "render", "main.md", &[]);
    assert_eq!(run.code, Some(2), "{}", run.text);
    let diagnostic = only_diagnostic(&run);
    assert_eq!(
        diagnostic["code"], "ERR_VAL_UNBOUND_VARIABLE",
        "{diagnostic}"
    );
    assert_eq!(
        diagnostic["path"],
        root.join("child.md").display().to_string()
    );
    assert_eq!(diagnostic["line"], 2);
    assert_eq!(
        diagnostic["include_chain"],
        chain(&root, &["main.md", "child.md"])
    );
    assert!(run.text.contains("child.md:2)"), "{}", run.text);
}

#[test]
fn first_rendered_pass_error_names_source_line() {
    // Stacked passes render highest first; pass 2 renders the source text.
    let root = canonical_root("adr0026-d10-pass-first");
    write_file(
        &root.join("stacked.md"),
        "---\npass: 2\n---\n---\npass: 1\n---\nfirst\n{{{ bad syntax }}}\n",
    );
    let run = run(&root, "render", "stacked.md", &[]);
    assert_located(&run, &root, "stacked.md", 8, Some(9), &["stacked.md"]);
}

#[test]
fn later_pass_error_says_it_is_in_the_pass_input() {
    // Pass 1 renders the output of pass 2, so its lines are not source lines.
    let root = canonical_root("adr0026-d10-pass-later");
    write_file(
        &root.join("stacked.md"),
        "---\npass: 2\n---\n---\npass: 1\n---\nfirst\n{{ bad syntax }}\n",
    );
    let run = run(&root, "render", "stacked.md", &[]);
    assert_eq!(run.code, Some(2), "{}", run.text);
    let diagnostic = only_diagnostic(&run);
    assert_eq!(diagnostic["code"], "ERR_RENDER_TEMPLATE", "{diagnostic}");
    assert_eq!(diagnostic["line"], Value::Null, "{diagnostic}");
    assert!(
        diagnostic["message"]
            .as_str()
            .unwrap()
            .contains("(in stacked.md:2 of the pass 1 input)"),
        "{diagnostic}"
    );
}
