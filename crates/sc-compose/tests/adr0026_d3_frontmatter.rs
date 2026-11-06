//! Duplicate YAML keys are refused in every frontmatter block.
#[path = "support/mod.rs"]
mod support;

use support::{sc_compose, temp_root, write_file};

#[test]
fn duplicate_frontmatter_keys_are_rejected_at_all_levels() {
    let root = temp_root("adr0026-d3-duplicates");
    for (label, input, key) in [
        ("unknown", "---\na: 1\na: 2\n---\nHello\n", "a"),
        (
            "metadata",
            "---\nmetadata:\n  name: one\n  name: two\n---\nHello\n",
            "name",
        ),
        (
            "known",
            "---\nrequired_variables: []\nrequired_variables: []\n---\nHello\n",
            "required_variables",
        ),
        (
            "stacked",
            "---\npass: 1\n---\n---\npass: 2\nmetadata:\n  item: one\n  item: two\n---\nHello\n",
            "item",
        ),
        (
            "deep",
            "---\nmetadata:\n  nested:\n    item: one\n    item: two\n---\nHello\n",
            "item",
        ),
        ("flow", "---\nmetadata: {a: 1, a: 2}\n---\nHello\n", "a"),
        (
            "sequence",
            "---\nmetadata:\n  list:\n    - {a: 1, a: 2}\n---\nHello\n",
            "a",
        ),
    ] {
        write_file(&root.join("template.j2"), input);
        for json in [false, true] {
            let mut cli = sc_compose();
            cli.args(["render", "--mode", "file", "--root"])
                .arg(&root)
                .args(["--file", "template.j2"]);
            if json {
                cli.arg("--json");
            }
            let output = cli.output().unwrap();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.status.code(), Some(3), "{label} json={json}: {text}");
            assert!(
                text.contains("ERR_CONFIG_PARSE") && text.contains(key) && text.contains("line"),
                "{label} json={json}: {text}"
            );
        }
    }
    write_file(
        &root.join("template.j2"),
        "---\nmetadata:\n  one: value\n  two: other\n---\nHello\n",
    );
    assert!(
        sc_compose()
            .args(["render", "--mode", "file", "--root"])
            .arg(&root)
            .args(["--file", "template.j2"])
            .output()
            .unwrap()
            .status
            .success()
    );
}
