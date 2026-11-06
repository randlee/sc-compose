//! Multi-pass rendering keeps variable-line boundaries.
#[path = "support/mod.rs"]
mod support;
use support::{sc_compose, temp_root, write_file};

#[test]
fn pass_two_preserves_variable_line_breaks() {
    let root = temp_root("adr0026-d9-newline");
    for (name, body, expected) in [
        ("end-of-line", "A {{{ name }}}\nB\n", "A value\nB\n"),
        ("mid-line", "A {{{ name }}} B\n", "A value B\n"),
        ("final-line", "A\n{{{ name }}}", "A\nvalue\n"),
        ("pass-one", "A {{ name }}\nB\n", "A value\nB\n"),
    ] {
        write_file(
            &root.join("template.j2"),
            &format!(
                "---\npass: 1\ndefaults: {{name: value}}\n---\n---\npass: 2\ndefaults: {{name: value}}\n---\n{body}"
            ),
        );
        for selection in ["--all", "--pass"] {
            let mut cmd = sc_compose();
            cmd.args(["render", selection]);
            if selection == "--pass" {
                cmd.arg(if name == "pass-one" { "1" } else { "2" });
            } else {
                cmd.args(["--pass", "1", "--pass", "2"]);
            }
            let output = cmd
                .args(["--mode", "file", "--root"])
                .arg(&root)
                .args(["--file", "template.j2"])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{name} {selection}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                expected,
                "{name} {selection}"
            );
        }
    }
}
