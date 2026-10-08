//! Regression tests from the adversarial fuzz campaign.

#[path = "support/mod.rs"]
mod support;

use support::{
    assert_envelope, assert_first_code, parse_stdout, sc_compose, temp_root, write_file,
};

/// A literal default-delimiter expression is inert when custom variable
/// delimiters are active and must not trigger strict undeclared-token errors.
#[test]
fn strict_validation_with_custom_delimiters_does_not_flag_literal_default_delimiter_text() {
    let root = temp_root("fuzz-strict-custom-delim-false-positive");
    write_file(
        &root.join("t.j2"),
        "---\nname: t\nversion: 1.0.0\nformat: markdown\nrequired_variables:\n  - name\n---\n<<name>>{{x}}",
    );

    let output = sc_compose()
        .args([
            "render",
            "--file",
            "t.j2",
            "--var",
            "name=World",
            "--variable-delimiters",
            "<<",
            ">>",
            "--strict",
            "--json",
            "--root",
        ])
        .arg(&root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "literal default-delimiter text must not fail strict validation: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// A variable referenced through the active custom delimiters must be caught
/// by strict validation before the renderer substitutes it with an empty
/// string.
#[test]
fn strict_validation_with_custom_delimiters_catches_undeclared_custom_delimiter_reference() {
    let root = temp_root("fuzz-strict-custom-delim-false-negative");
    write_file(
        &root.join("t.j2"),
        "---\nname: t\nversion: 1.0.0\nformat: markdown\nrequired_variables:\n  - name\n---\n<<name>><<undeclared>>",
    );

    let output = sc_compose()
        .args([
            "render",
            "--file",
            "t.j2",
            "--var",
            "name=World",
            "--variable-delimiters",
            "<<",
            ">>",
            "--strict",
            "--json",
            "--root",
        ])
        .arg(&root)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_VAL_UNDECLARED_TOKEN");
}

#[test]
fn adjacent_plain_yaml_frontmatter_block_is_not_silently_consumed_as_a_second_pass() {
    let root = temp_root("fuzz-adjacent-plain-yaml-frontmatter");
    write_file(&root.join("t.j2"), "---\n{}\n---\n---\na: b\n---\nBODY\n");

    let output = sc_compose()
        .args(["render", "--file", "t.j2", "--root"])
        .arg(&root)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let rendered = String::from_utf8(output.stdout).unwrap();
    assert!(
        rendered.contains("---") && rendered.contains("a: b"),
        "{rendered:?}"
    );
}

#[test]
fn whitespace_control_tag_markers_do_not_produce_a_phantom_dash_variable_under_strict() {
    let root = temp_root("fuzz-whitespace-control-phantom-dash");
    write_file(&root.join("t.j2"), "{%- if true %}Hi{% endif %}");

    let output = sc_compose()
        .args(["render", "--file", "t.j2", "--strict", "--json", "--root"])
        .arg(&root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(value["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn opening_delimiter_with_trailing_whitespace_does_not_silently_bypass_required_variables() {
    let root = temp_root("fuzz-opening-delimiter-trailing-whitespace");
    write_file(
        &root.join("t.j2"),
        "---   \nrequired_variables:\n  - name\n---\nHi {{ name }}\n",
    );

    let output = sc_compose()
        .args(["render", "--file", "t.j2", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    assert!(
        stderr.contains("ERR_VAL_MISSING_REQUIRED"),
        "stderr: {stderr}"
    );
}

/// The default text path should retain sc-compose's stable parse message while
/// exposing the preserved `serde_yaml` source chain for diagnosis.
#[test]
fn malformed_frontmatter_text_output_preserves_serde_yaml_error_details() {
    let root = temp_root("fuzz-config-parse-raw-yaml");
    write_file(&root.join("t.j2"), "---\ndefaults: [\n---\nbody\n");

    let output = sc_compose()
        .args(["render", "--file", "t.j2", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(3), "stderr: {stderr}");
    assert!(
        stderr.contains("failed to parse YAML frontmatter"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("caused by:"), "stderr: {stderr}");
    assert!(
        stderr.contains("invalid type: sequence, expected a map"),
        "stderr: {stderr}"
    );
}

#[test]
fn malformed_frontmatter_json_output_remains_structured_and_stable() {
    let root = temp_root("fuzz-config-parse-json");
    write_file(&root.join("t.j2"), "---\ndefaults: [\n---\nbody\n");

    let output = sc_compose()
        .args(["render", "--file", "t.j2", "--json", "--root"])
        .arg(&root)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_PARSE");
    assert_eq!(
        value["diagnostics"][0]["message"],
        "failed to parse YAML frontmatter"
    );
}

/// FUZZ-002 (adversarial fuzz campaign 20260811-3, boundary-probe): a
/// malformed `--var` value (missing the `key=value` separator) is rejected
/// by clap's own value-parser error path before sc-compose's application
/// layer ever runs, so the tool prints plain-text usage text on stderr and
/// leaves stdout empty even though `--json` was explicitly requested. Every
/// diagnostic emitted while `--json` is set, including CLI-usage errors,
/// must stay inside the tool's stable JSON envelope.
#[test]
fn malformed_var_argument_does_not_bypass_the_json_output_contract() {
    let output = sc_compose()
        .args(["validate", "--json", "--var", "novalue"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3));
    assert!(
        !output.stdout.is_empty(),
        "expected a JSON envelope on stdout, got empty stdout; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
}

/// FUZZ-003 (adversarial fuzz campaign 20260811-3, template-probe): `--all`
/// is declared `conflicts_with_all` against `--brace-count` and
/// `--variable-delimiters`, and clap enforces that conflict before
/// sc-compose's application layer runs, so the resulting argument-conflict
/// error is plain clap usage text on stderr rather than the tool's stable
/// JSON envelope, even though `--json` was explicitly requested.
#[test]
fn all_and_brace_count_conflict_does_not_bypass_the_json_output_contract() {
    let root = temp_root("fuzz-all-brace-count-json-contract");
    write_file(&root.join("t.j2"), "Hello {{ name }}\n");

    let output = sc_compose()
        .args([
            "render",
            "--json",
            "--all",
            "--brace-count",
            "3",
            "--file",
            "t.j2",
            "--root",
        ])
        .arg(&root)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3));
    assert!(
        !output.stdout.is_empty(),
        "expected a JSON envelope on stdout, got empty stdout; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
}

#[test]
fn clap_usage_errors_exit_with_usage_fail_in_plain_text_mode() {
    for args in [
        &["validate", "--var", "novalue"][..],
        &["render", "--all", "--brace-count", "3", "--file", "t.j2"][..],
        &["validate", "--unknown-flag"][..],
    ] {
        let output = sc_compose().args(args).output().unwrap();

        assert_eq!(
            output.status.code(),
            Some(3),
            "expected usage failure for {args:?}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.stderr.is_empty(),
            "expected usage text for {args:?}"
        );
    }
}

#[test]
fn help_and_version_preserve_clap_display_output_in_both_modes() {
    for (args, expected_text) in [
        (&["--version", "--json"][..], "sc-compose"),
        (&["render", "--help", "--json"][..], "render [OPTIONS]"),
        (&["--version"][..], "sc-compose"),
        (&["render", "--help"][..], "render [OPTIONS]"),
    ] {
        let output = sc_compose().args(args).output().unwrap();

        assert!(output.status.success(), "args={args:?}: {output:?}");
        assert!(
            output.stderr.is_empty(),
            "args={args:?}: unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected_text),
            "args={args:?}: expected {expected_text:?} in stdout: {stdout}"
        );
        assert!(
            !stdout.trim_start().starts_with('{'),
            "args={args:?}: display output must not be a JSON envelope: {stdout}"
        );
    }
}

/// Writes a `bead render` request for `template` (relative to the workspace)
/// and returns its absolute path.
fn write_bead_render_request(root: &std::path::Path, template: &str) -> std::path::PathBuf {
    let request = root.join("request.json");
    write_file(
        &request,
        &serde_json::json!({
            "schema": "sc-compose/beads/v1",
            "operation": "render",
            "working_directory": root,
            "template": template,
            "rendered_formula": root.join("build/out.formula.toml"),
            "compose_variables": {},
            "bead_variables": {}
        })
        .to_string(),
    );
    std::fs::create_dir_all(root.join("build")).unwrap();
    request
}

// FUZZ-012: a relative `template` resolves against `working_directory`
// (ADR-0021 "All paths resolve relative to working_directory"), not against
// the caller's current directory.
#[test]
#[ignore = "FUZZ-012"]
fn fuzz_012_bead_request_template_is_relative_to_working_directory() {
    let root = std::fs::canonicalize(temp_root("fuzz-012-bead-relative-template")).unwrap();
    write_file(
        &root.join("wf/m.formula.toml.j2"),
        "formula = \"m\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"a\"\ntitle = \"A\"\n",
    );
    let request = write_bead_render_request(&root, "wf/m.formula.toml.j2");
    let elsewhere = root.join("other");
    std::fs::create_dir_all(&elsewhere).unwrap();

    let output = sc_compose()
        .current_dir(&elsewhere)
        .args(["bead", "render", "--json", "--request"])
        .arg(&request)
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(root.join("build/out.formula.toml").is_file());
}

// FUZZ-017: an appended record keeps the rendered object's values; only key
// order may change (`sc-compose help render`, "Appending JSON records").
#[test]
#[ignore = "FUZZ-017"]
fn fuzz_017_render_append_keeps_exact_number_values() {
    let root = temp_root("fuzz-017-append-number-fidelity");
    write_file(
        &root.join("rec.json.j2"),
        "{\"id\": 12345678901234567890123, \"amount\": 0.10000000000000000001}",
    );
    let destination = root.join("log.jsonl");

    let output = sc_compose()
        .args(["render", "--file", "rec.json.j2", "--root"])
        .arg(&root)
        .arg("--append")
        .arg(&destination)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let line = std::fs::read_to_string(&destination).unwrap();
    assert!(
        line.contains("\"id\":12345678901234567890123")
            && line.contains("\"amount\":0.10000000000000000001"),
        "appended record changed a value: {line}"
    );
}

// FUZZ-018: `bead render` expands sc-compose `@<path>` includes, as the bead
// manual's "Shared step blocks" section and FR-23.1 promise.
#[test]
#[ignore = "FUZZ-018"]
fn fuzz_018_bead_render_expands_at_path_includes() {
    let root = std::fs::canonicalize(temp_root("fuzz-018-bead-include")).unwrap();
    write_file(
        &root.join("main.formula.toml.j2"),
        "formula = \"inc\"\nversion = 1\ntype = \"workflow\"\n@<frag/common.toml.j2>\n[[steps]]\nid = \"local\"\ntitle = \"Local\"\n",
    );
    write_file(
        &root.join("frag/common.toml.j2"),
        "[[steps]]\nid = \"shared\"\ntitle = \"Shared step\"\n",
    );
    let request = write_bead_render_request(&root, "main.formula.toml.j2");

    let output = sc_compose()
        .current_dir(&root)
        .args(["bead", "render", "--json", "--request"])
        .arg(&request)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let rendered = std::fs::read_to_string(root.join("build/out.formula.toml")).unwrap();
    assert!(
        rendered.contains("id = \"shared\"") && !rendered.contains("@<"),
        "include was not expanded:\n{rendered}"
    );
}

// FUZZ-019: a failed bead render explains why (FR-8): the render stage carries
// the template error, not an empty excerpt.
#[test]
#[ignore = "FUZZ-019"]
fn fuzz_019_bead_render_failure_reports_the_template_error() {
    let root = std::fs::canonicalize(temp_root("fuzz-019-bead-render-message")).unwrap();
    write_file(&root.join("bad.formula.toml.j2"), "{% if %}\n");
    let request = write_bead_render_request(&root, "bad.formula.toml.j2");

    let output = sc_compose()
        .current_dir(&root)
        .args(["bead", "render", "--json", "--request"])
        .arg(&request)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let receipt = parse_stdout(&output);
    let stage = &receipt["payload"]["stages"][0];
    assert_eq!(stage["outcome"]["failed"]["code"], "BEADS_RENDER_FAILED");
    assert!(
        !stage["stderr_excerpt"]
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "render stage has no diagnostic: {stage}"
    );
}
