//! File resolution distinguishes root escapes from missing in-root files.
#[path = "support/mod.rs"]
mod support;
use support::{sc_compose, temp_root, write_file};

#[cfg(unix)]
#[test]
fn escapes_report_the_same_cause_for_render_and_validate() {
    use std::os::unix::fs::symlink;
    let base = temp_root("adr0026-d1-resolve");
    let root = base.join("root");
    std::fs::create_dir_all(&root).unwrap();
    write_file(&base.join("outside.j2"), "hello");
    symlink(base.join("outside.j2"), root.join("existing.j2")).unwrap();
    symlink(base.join("missing.j2"), root.join("dangling.j2")).unwrap();
    for name in ["../outside.j2", "existing.j2", "dangling.j2"] {
        for command in ["render", "validate"] {
            for json in [false, true] {
                let mut cli = sc_compose();
                cli.args([command, "--mode", "file", "--root"])
                    .arg(&root)
                    .args(["--file", name]);
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
                    Some(3),
                    "{name} {command} json={json}: {text}"
                );
                assert!(
                    text.contains("ERR_RESOLVE_ESCAPE")
                        && text.contains("template path escapes configured roots"),
                    "{name} {command} json={json}: {text}"
                );
            }
        }
    }
    for command in ["render", "validate"] {
        let output = sc_compose()
            .args([command, "--mode", "file", "--root"])
            .arg(&root)
            .args(["--file", "missing.j2", "--json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&output.stdout).contains("ERR_RESOLVE_NOT_FOUND"));
    }
}

#[test]
fn relative_escape_and_in_root_missing_are_distinct_on_every_platform() {
    let base = temp_root("adr0026-d1-portable");
    let root = base.join("root");
    std::fs::create_dir_all(&root).unwrap();
    write_file(&base.join("outside.j2"), "hello");
    for (file, code) in [
        ("../outside.j2", "ERR_RESOLVE_ESCAPE"),
        ("missing.j2", "ERR_RESOLVE_NOT_FOUND"),
    ] {
        let output = sc_compose()
            .args(["render", "--mode", "file", "--root"])
            .arg(&root)
            .args(["--file", file, "--json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&output.stdout).contains(code));
    }
}

#[cfg(unix)]
#[test]
fn profile_symlink_escapes_fail_in_all_supported_commands() {
    use std::os::unix::fs::symlink;
    let base = temp_root("adr0026-d1-profile");
    let root = base.join("root");
    let agents = root.join(".claude/agents");
    std::fs::create_dir_all(&agents).unwrap();
    write_file(&base.join("outside.md"), "outside");
    symlink(base.join("outside.md"), agents.join("existing.md")).unwrap();
    symlink(base.join("missing.md"), agents.join("dangling.md")).unwrap();
    for name in ["existing", "dangling"] {
        for cmd in ["resolve", "render", "validate"] {
            for json in [false, true] {
                let mut cli = sc_compose();
                cli.args([cmd, "--mode", "profile", "--root"])
                    .arg(&root)
                    .args(["--kind", "agent", "--agent", name, "--runtime", "claude"]);
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
                    Some(3),
                    "{cmd} {name} json={json}: {text}"
                );
                assert!(
                    text.contains("ERR_RESOLVE_ESCAPE")
                        && text.contains("template path escapes configured roots"),
                    "{cmd} {name} json={json}: {text}"
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn profile_lookup_refuses_a_missing_name_beneath_an_escaping_directory_symlink() {
    use std::os::unix::fs::symlink;
    let base = temp_root("adr0026-d1-profile-directory");
    let root = base.join("root");
    std::fs::create_dir_all(root.join(".claude")).unwrap();
    std::fs::create_dir_all(base.join("outside")).unwrap();
    symlink(base.join("outside"), root.join(".claude/agents")).unwrap();
    for cmd in ["resolve", "render", "validate"] {
        let output = sc_compose()
            .args([cmd, "--mode", "profile", "--root"])
            .arg(&root)
            .args([
                "--kind",
                "agent",
                "--agent",
                "missing",
                "--runtime",
                "claude",
                "--json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&output.stdout).contains("ERR_RESOLVE_ESCAPE"));
    }
}
