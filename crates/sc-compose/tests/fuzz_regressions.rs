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

/// The default text path should expose only sc-compose's stable parse message,
/// not `serde_yaml`'s raw source-chain text. The existing formatter backtrace
/// remains outside this narrow parser-scope fix.
#[test]
fn malformed_frontmatter_text_output_hides_raw_serde_yaml_error_details() {
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
    assert!(!stderr.contains("caused by"), "stderr: {stderr}");
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
