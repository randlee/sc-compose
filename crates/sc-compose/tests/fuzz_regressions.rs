//! Regression tests promoted from the adversarial fuzz campaign.

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
