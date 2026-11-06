//! Complex YAML keys must be identified in duplicate diagnostics.
#[path = "support/mod.rs"]
mod support;
use support::{sc_compose, temp_root, write_file};

#[test]
fn duplicate_complex_keys_name_the_key_and_block() {
    let root = temp_root("d3-complex-keys");
    for (name, input, key, block_line) in [
        (
            "sequence",
            "---\n? [alpha, beta]\n: 1\n? [alpha, beta]\n: 2\n---\nbody\n",
            "alpha",
            "line 2",
        ),
        (
            "mapping",
            "---\nmetadata:\n  ? {alpha: beta}\n  : 1\n  ? {alpha: beta}\n  : 2\n---\nbody\n",
            "alpha",
            "line 2",
        ),
        (
            "nested",
            "---\nmetadata:\n  entries:\n    - ? [alpha, beta]\n      : 1\n      ? [alpha, beta]\n      : 2\n---\nbody\n",
            "alpha",
            "line 2",
        ),
        (
            "stacked",
            "---\npass: 1\n---\n---\npass: 2\nmetadata:\n  ? [alpha, beta]\n  : 1\n  ? [alpha, beta]\n  : 2\n---\nbody\n",
            "alpha",
            "line 5",
        ),
        (
            "tagged",
            "---\n? !example alpha\n: 1\n? !example alpha\n: 2\n---\nbody\n",
            "alpha",
            "line 2",
        ),
    ] {
        write_file(&root.join("t.j2"), input);
        for json in [false, true] {
            let mut cmd = sc_compose();
            cmd.args(["render", "--mode", "file", "--root"])
                .arg(&root)
                .args(["--file", "t.j2"]);
            if json {
                cmd.arg("--json");
            }
            let output = cmd.output().unwrap();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.status.code(), Some(3), "{name}, json={json}: {text}");
            assert!(
                text.contains("ERR_CONFIG_PARSE")
                    && text.contains(key)
                    && text.contains(block_line),
                "{name}, json={json}: {text}"
            );
        }
    }
}

#[test]
fn duplicate_keys_in_unrecognized_yaml_body_are_not_frontmatter() {
    let root = temp_root("d3-yaml-body");
    let body = "---\nordinary: 1\nordinary: 2\n---\nbody\n";
    write_file(&root.join("t.j2"), &format!("---\npass: 1\n---\n{body}"));
    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args(["--file", "t.j2"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), body);
}
