//! Engine failures retain typed diagnostics under custom variable delimiters.
#[path = "support/mod.rs"]
mod support;
use support::{sc_compose, temp_root, write_file};

#[test]
fn custom_and_default_engine_errors_keep_template_diagnostics() {
    let root = temp_root("adr0026-d2-custom");
    for (option, syntax, evaluation) in [
        ("brace", "{{ 1 + }}\n", "{{ range(3) | int }}\n"),
        ("custom", "<< 1 + >>\n", "<< range(3) | int >>\n"),
        ("default", "{{ 1 + }}\n", "{{ range(3) | int }}\n"),
    ] {
        for (kind, source) in [("syntax", syntax), ("evaluation", evaluation)] {
            write_file(&root.join("template.j2"), source);
            for json in [false, true] {
                let mut cmd = sc_compose();
                cmd.args(["render", "--mode", "file", "--root"])
                    .arg(&root)
                    .args(["--file", "template.j2"]);
                match option {
                    "brace" => {
                        cmd.args(["--brace-count", "2"]);
                    }
                    "custom" => {
                        cmd.args(["--variable-delimiters", "<<", ">>"]);
                    }
                    _ => {}
                }
                if json {
                    cmd.arg("--json");
                }
                let output = cmd.output().unwrap();
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    output.status.code(),
                    Some(2),
                    "{option} {kind} json={json}: {text}"
                );
                assert!(
                    text.contains("ERR_RENDER_TEMPLATE") && text.contains("template.j2"),
                    "{option} {kind} json={json}: {text}"
                );
                if json {
                    let envelope: serde_json::Value =
                        serde_json::from_slice(&output.stdout).unwrap();
                    let diagnostic = &envelope["diagnostics"][0];
                    assert_eq!(diagnostic["line"], 1);
                    assert!(
                        diagnostic["path"]
                            .as_str()
                            .unwrap()
                            .ends_with("template.j2")
                    );
                }
            }
        }
    }
}

#[test]
fn invalid_delimiters_remain_configuration_errors() {
    let root = temp_root("adr0026-d2-invalid-delimiters");
    write_file(&root.join("template.j2"), "hello\n");
    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args([
            "--file",
            "template.j2",
            "--variable-delimiters",
            "",
            "",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stdout).contains("ERR_CONFIG_PARSE"));
}

#[test]
fn custom_include_errors_preserve_source_chain_and_unbound_code() {
    let root = temp_root("adr0026-d2-custom-include");
    write_file(&root.join("template.j2"), "before\n@<child.md>\nafter\n");
    for (source, code) in [
        ("<< 1 + >>", "ERR_RENDER_TEMPLATE"),
        ("<< missing.field >>", "ERR_VAL_UNBOUND_VARIABLE"),
    ] {
        write_file(&root.join("child.md"), &format!("first\n{source}\n"));
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
        assert_eq!(output.status.code(), Some(2));
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let diagnostic = &value["diagnostics"][0];
        assert_eq!(diagnostic["code"], code);
        assert_eq!(diagnostic["line"], 2);
        assert!(diagnostic["path"].as_str().unwrap().ends_with("child.md"));
        let chain = diagnostic["include_chain"].as_array().unwrap();
        assert_eq!(chain.len(), 2);
        assert!(chain[0].as_str().unwrap().ends_with("template.j2"));
        assert!(chain[1].as_str().unwrap().ends_with("child.md"));
    }
}

#[test]
fn custom_delimiter_output_write_failure_remains_a_write_error() {
    let root = temp_root("adr0026-d2-custom-write");
    write_file(&root.join("template.j2"), "hello\n");
    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args([
            "--file",
            "template.j2",
            "--variable-delimiters",
            "<<",
            ">>",
            "--output",
        ])
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).contains("ERR_RENDER_WRITE"));
}
