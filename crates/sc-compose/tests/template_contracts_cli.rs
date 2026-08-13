mod support;

use support::{TempFixture, parse_stdout, sc_compose, write_file};

#[test]
fn validate_lint_auto_mode_quoted_placeholder_is_nonzero_with_location() {
    let fixture = TempFixture::new("template-contracts-validate");
    write_file(
        &fixture.path.join("payload.json.j2"),
        "{\"value\": \"{{ value }}\"}\n",
    );
    let output = sc_compose()
        .args([
            "validate",
            "--lint",
            "--mode",
            "file",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--file",
            "payload.json.j2",
            "--var",
            "value=hello",
            "--json",
        ])
        .output()
        .expect("run validate --lint");

    assert_eq!(output.status.code(), Some(2));
    let envelope = parse_stdout(&output);
    assert_eq!(envelope["payload"]["valid"], true);
    let diagnostic = envelope["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .find(|diagnostic| {
            diagnostic["code"] == "WARN_JSON_LEGACY_ESCAPE_MODE"
                && diagnostic["severity"] == "error"
        })
        .expect("quoted placeholder diagnostic");
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(diagnostic["line"], 1);
    assert_eq!(diagnostic["column"], 12);
    assert!(
        diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains("docs/migration/json-escape-mode.md"))
    );
}
