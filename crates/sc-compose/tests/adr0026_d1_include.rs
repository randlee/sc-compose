//! Boundary-first include diagnostics for ADR-0026 D1.
#![cfg(unix)]

#[path = "support/mod.rs"]
mod support;

use std::fs;
use support::{sc_compose, temp_root, write_file};

#[test]
fn include_dangling_symlink_boundary_precedes_not_found_across_commands() {
    use std::os::unix::fs::symlink;

    let namespace = temp_root("include-dangling-containment");
    let root = namespace.join("repo");
    let outside = namespace.join("outside");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    write_file(&outside.join("exists.md"), "outside");
    symlink(outside.join("missing.md"), root.join("outside-link.md")).unwrap();
    symlink("../outside/missing.md", root.join("relative-link.md")).unwrap();
    symlink("relative-link.md", root.join("chain.md")).unwrap();
    symlink("missing.md", root.join("inside-link.md")).unwrap();
    symlink(outside.join("exists.md"), root.join("existing-link.md")).unwrap();
    symlink(&outside, root.join("outside-dir")).unwrap();
    fs::create_dir_all(root.join("inside-dir")).unwrap();
    symlink("inside-dir", root.join("inside-dir-link")).unwrap();
    write_file(&root.join("nested/parent.md"), "@<../outside/missing.md>\n");
    write_file(&root.join("deployed.md"), "existing");

    for (target, expected) in [
        ("outside-link.md", "ERR_INCLUDE_ESCAPE"),
        ("chain.md", "ERR_INCLUDE_ESCAPE"),
        ("inside-link.md", "ERR_INCLUDE_NOT_FOUND"),
        ("inside-dir-link/missing.md", "ERR_INCLUDE_NOT_FOUND"),
        ("outside-dir/missing.md", "ERR_INCLUDE_ESCAPE"),
        ("nested/parent.md", "ERR_INCLUDE_ESCAPE"),
        ("existing-link.md", "ERR_INCLUDE_ESCAPE"),
        ("../outside/missing.md", "ERR_INCLUDE_ESCAPE"),
    ] {
        write_file(&root.join("template.md.j2"), &format!("@<{target}>\n"));
        for (command, json) in ["render", "validate", "verify"]
            .into_iter()
            .flat_map(|command| [(command, false), (command, true)])
        {
            let mut cli = sc_compose();
            cli.args([command, "--mode", "file", "--root"]).arg(&root);
            if command == "verify" {
                cli.args(["--against", "template.md.j2"])
                    .arg(root.join("deployed.md"));
            } else {
                cli.args(["--file", "template.md.j2"]);
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
                "{command} {target} json={json}: {text}"
            );
            assert!(
                text.contains(expected),
                "{command} {target} json={json}: {text}"
            );
        }
    }
}
