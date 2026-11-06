use std::fs;

use sc_composer_beads::{
    BeadOperation, StdProcessRunner, execute_bead_request_with_runner, parse_request,
};
use serde_json::json;

fn temp_root(label: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
    let nonce = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "adr0026-d6-{label}-{}-{timestamp}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("temporary root");
    root
}

#[test]
fn unknown_top_level_annotations_are_ignored_for_every_operation() {
    for operation in [
        BeadOperation::Render,
        BeadOperation::Validate,
        BeadOperation::PreviewPour,
        BeadOperation::Pour,
        BeadOperation::PreviewAttach,
        BeadOperation::Attach,
    ] {
        let request = json!({
            "schema": "sc-compose/beads/v1",
            "operation": operation,
            "working_directory": "/work",
            "template": "caller-selected-template.source",
            "rendered_formula": "/work/formula.toml",
            "compose_variables": {},
            "bead_variables": {},
            "caller_annotation": {"owner": "another-tool"}
        });

        let parsed = parse_request(&request.to_string()).unwrap_or_else(|error| {
            panic!("{operation:?} rejected an unknown annotation: {error}")
        });
        assert_eq!(parsed.operation, operation);
        assert_eq!(
            parsed.template,
            std::path::PathBuf::from("caller-selected-template.source")
        );
    }
}

#[test]
fn request_template_suffix_is_not_enforced_when_rendered_extension_is_supported() {
    let root = temp_root("template-suffix");
    let template = root.join("caller-template.source");
    let rendered_formula = root.join("result.formula.toml");
    fs::write(&template, "title = \"suffix is caller-owned\"\n").expect("template");
    let request = json!({
        "schema": "sc-compose/beads/v1",
        "operation": "render",
        "working_directory": root,
        "template": template,
        "rendered_formula": rendered_formula,
        "compose_variables": {},
        "bead_variables": {},
        "caller_annotation": "preserved by the caller"
    });
    let parsed =
        parse_request(&request.to_string()).expect("request with unconventional template suffix");

    let receipt = execute_bead_request_with_runner(&parsed, &StdProcessRunner::default())
        .expect("render with a caller-selected template suffix");

    assert_eq!(receipt.outcome, sc_composer_beads::BeadOutcome::Succeeded);
    assert_eq!(
        fs::read_to_string(rendered_formula).expect("rendered formula"),
        "title = \"suffix is caller-owned\""
    );
}
