//! Template failures and output-write failures have distinct diagnostics.
#[path = "support/mod.rs"]
mod support;

use support::{sc_compose, temp_root, write_file};

#[test]
fn template_engine_failures_have_template_code_in_text_and_json() {
    let root = temp_root("adr0026-d2-template-errors");
    for source in ["ok-prefix {{ name \n", "{{ name | no_such_filter }}\n"] {
        write_file(&root.join("invalid.j2"), source);
        for command in ["render", "validate"] {
            for json in [false, true] {
                let mut cli = sc_compose();
                cli.args([command, "--mode", "file", "--root"])
                    .arg(&root)
                    .args(["--file", "invalid.j2", "--var", "name=x"]);
                if command == "validate" {
                    cli.arg("--check-render");
                }
                if json {
                    cli.arg("--json");
                }
                let output = cli.output().unwrap();
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    output.status.code(),
                    Some(2),
                    "{command} json={json}: {text}"
                );
                assert!(
                    text.contains("ERR_RENDER_TEMPLATE"),
                    "{command} json={json}: {text}"
                );
                assert!(
                    text.contains("invalid.j2") && text.contains(":1"),
                    "{command} json={json}: {text}"
                );
                assert!(
                    !text.contains("ERR_RENDER_WRITE"),
                    "{command} json={json}: {text}"
                );
            }
        }
    }
}

#[test]
fn output_write_failure_retains_write_code() {
    let root = temp_root("adr0026-d2-write-error");
    write_file(&root.join("valid.j2"), "hello\n");
    for json in [false, true] {
        let mut cli = sc_compose();
        cli.args(["render", "--mode", "file", "--root"])
            .arg(&root)
            .args(["--file", "valid.j2", "--output"])
            .arg(&root);
        if json {
            cli.arg("--json");
        }
        let output = cli.output().unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.status.code(), Some(2), "json={json}: {text}");
        assert!(text.contains("ERR_RENDER_WRITE"), "json={json}: {text}");
    }
}
