//! Include replacement keeps parent and child line boundaries.
#[path = "support/mod.rs"]
mod support;
use support::{sc_compose, temp_root, write_file};

#[test]
fn includes_preserve_line_terminators_and_layout() {
    let root = temp_root("adr0026-d8-layout");
    for (name, parent, child, expected) in [
        (
            "final-crlf",
            "A\r\n@<child.md>\r\nB\r\n",
            "C",
            "A\r\nC\r\nB\r\n",
        ),
        (
            "crlf-parent",
            "A\r\n@<child.md>\r\nB\r\nZ",
            "C\n",
            "A\r\nC\nB\r\nZ\n",
        ),
        (
            "crlf-child",
            "A\n@<child.md>\nB\nZ",
            "C\r\n",
            "A\nC\r\nB\nZ\n",
        ),
        (
            "no-terminator",
            "A\r\n@<child.md>\r\nB\r\nZ",
            "C",
            "A\r\nC\r\nB\r\nZ\n",
        ),
        (
            "indented",
            "A\n  @<child.md>\nB\nZ",
            "C\nD\n",
            "A\nC\nD\nB\nZ\n",
        ),
        (
            "raw",
            "{% raw %}\n@<child.md>\n{% endraw %}Z",
            "C\n",
            "C\nZ\n",
        ),
    ] {
        write_file(&root.join("template.j2"), parent);
        write_file(&root.join("child.md"), child);
        let output = sc_compose()
            .args(["render", "--mode", "file", "--root"])
            .arg(&root)
            .args(["--file", "template.j2"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected, "{name}");
    }
}

#[test]
fn conditional_include_keeps_the_line_break_in_both_branches() {
    let root = temp_root("adr0026-d8-conditional");
    write_file(
        &root.join("template.j2"),
        "A\n@<{{ 'yes.md' if pick else 'no.md' }}>\nB\nZ",
    );
    write_file(&root.join("yes.md"), "Y");
    write_file(&root.join("no.md"), "N");
    for (args, expected) in [
        (&["--var", "pick=chosen"][..], "A\nY\nB\nZ\n"),
        (&[][..], "A\nN\nB\nZ\n"),
    ] {
        let output = sc_compose()
            .args(["render", "--mode", "file", "--root"])
            .arg(&root)
            .args(["--file", "template.j2"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    }
}

#[test]
fn final_crlf_survives_stdout_file_and_json_with_custom_delimiters() {
    let root = temp_root("adr0026-d8-output-modes");
    write_file(&root.join("template.j2"), "A\r\n@<child.md>\r\nB\r\n");
    write_file(&root.join("child.md"), "C");
    for custom in [false, true] {
        for mode in ["stdout", "file", "json"] {
            let mut command = sc_compose();
            command
                .args(["render", "--mode", "file", "--root"])
                .arg(&root)
                .args(["--file", "template.j2"]);
            if custom {
                command.args(["--variable-delimiters", "<<", ">>"]);
            }
            if mode == "file" {
                command.arg("--output").arg(root.join("output.txt"));
            }
            if mode == "json" {
                command.arg("--json");
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{mode} custom={custom}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if mode == "file" {
                assert_eq!(
                    std::fs::read(root.join("output.txt")).unwrap(),
                    b"A\r\nC\r\nB\r\n"
                );
            } else if mode == "json" {
                let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(payload["payload"]["body"], "A\r\nC\r\nB\r\n");
            } else {
                assert_eq!(output.stdout, b"A\r\nC\r\nB\r\n");
            }
        }
    }
}

#[test]
fn unrelated_noninclude_and_variable_crlf_keep_existing_stdout_behavior() {
    let root = temp_root("adr0026-d8-controls");
    write_file(&root.join("vars.json"), "{\"v\": \"end\\r\\n\"}");
    write_file(&root.join("child.md"), "child\n");
    for (name, body, expected) in [
        ("no-include", "A\r\nB\r\n", "A\r\nB\n"),
        ("variable", "@<child.md>\n{{ v }}", "child\nend\n"),
    ] {
        write_file(&root.join("template.j2"), body);
        let output = sc_compose()
            .args(["render", "--mode", "file", "--root"])
            .arg(&root)
            .args(["--file", "template.j2", "--var-file"])
            .arg(root.join("vars.json"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected, "{name}");
    }
}

#[test]
fn final_blank_lines_keep_existing_trimming_and_crlf_kind() {
    let root = temp_root("adr0026-d8-blank-lines");
    write_file(&root.join("template.j2"), "A\r\n@<child.md>\r\nB  \r\n\r\n");
    write_file(&root.join("child.md"), "C");
    for custom in [false, true] {
        let mut command = sc_compose();
        command
            .args(["render", "--mode", "file", "--root"])
            .arg(&root)
            .args(["--file", "template.j2"]);
        if custom {
            command.args(["--variable-delimiters", "<<", ">>"]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"A\r\nC\r\nB\r\n");
    }
}

#[test]
fn custom_delimiter_include_error_retains_source_location() {
    let root = temp_root("adr0026-d8-error-location");
    write_file(&root.join("template.j2"), "A\r\n@<child.md>\r\nB\r\n");
    write_file(&root.join("child.md"), "{% if %}\n");
    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args([
            "--file",
            "template.j2",
            "--variable-delimiters",
            "<<",
            ">>",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("child.md:1"));
}

#[test]
fn crlf_receipt_case_materializes_git_stable_fixture() {
    let root = temp_root("adr0026-d8-fixture");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/render-regressions/adr0026-d8/line-endings");
    let hex = std::fs::read_to_string(fixture.join("template.hex")).unwrap();
    let bytes = hex
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    std::fs::write(root.join("template.j2"), bytes).unwrap();
    std::fs::copy(fixture.join("child.md"), root.join("child.md")).unwrap();
    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args(["--file", "template.j2"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"A\r\nC\nB\r\n");
}

/// Render `template` with an empty `e.md` beside it and return stdout bytes.
fn render_with_empty_child(label: &str, template: &[u8]) -> Vec<u8> {
    let root = temp_root(label);
    std::fs::write(root.join("template.j2"), template).unwrap();
    std::fs::write(root.join("e.md"), b"").unwrap();
    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args(["--file", "template.j2"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn empty_child_directive_line_becomes_an_empty_line() {
    // ADR-0026 D8: empty content has no final line break, so the directive's
    // own terminator follows it and the directive line is an empty line.
    // Trailing blank lines keep the existing final trimming.
    for (name, template, expected) in [
        ("lf-nonfinal", &b"A\n@<e.md>\nB\n"[..], &b"A\n\nB\n"[..]),
        ("crlf-nonfinal", b"A\r\n@<e.md>\r\nB\r\n", b"A\r\n\r\nB\r\n"),
        ("lf-final", b"A\n@<e.md>\n", b"A\n"),
        ("lf-final-no-eol", b"A\n@<e.md>", b"A\n"),
        ("crlf-final", b"A\r\n@<e.md>\r\n", b"A\r\n"),
    ] {
        assert_eq!(
            render_with_empty_child(&format!("adr0026-d8-empty-{name}"), template),
            expected,
            "{name}"
        );
    }
}

#[test]
fn empty_child_receipt_case_renders_an_empty_directive_line() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/render-regressions/adr0026-d8/empty-child");
    assert_eq!(std::fs::read(fixture.join("e.md")).unwrap(), b"");
    let template = std::fs::read(fixture.join("template.j2")).unwrap();
    assert_eq!(template, b"A\n@<e.md>\nB\n");
    assert_eq!(
        render_with_empty_child("adr0026-d8-empty-receipt", &template),
        b"A\n\nB\n"
    );
}
