//! Stable `sc-compose/beads/v1` contract coverage.

use std::path::PathBuf;

use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeError, BeadComposeRequest, BeadOperation, BeadOutcome, BeadStage,
    BeadStageOutcome,
};

#[test]
fn protocol_types_serialize_with_stable_names() {
    assert_eq!(
        serde_json::to_string(&BeadOperation::PreviewPour).expect("serialize"),
        "\"preview_pour\""
    );
    assert_eq!(
        serde_json::to_string(&BeadStage::ResolveActiveRegistry).expect("serialize"),
        "\"resolve_active_registry\""
    );
    assert_eq!(
        serde_json::to_string(&BeadOutcome::Refused {
            code: "BEADS_POUR_AUTH_REQUIRED".to_owned()
        })
        .expect("serialize"),
        r#"{"refused":{"code":"BEADS_POUR_AUTH_REQUIRED"}}"#
    );
    assert_eq!(
        serde_json::to_string(&BeadStageOutcome::Failed {
            code: "BEADS_COOK_FAILED".to_owned()
        })
        .expect("serialize"),
        r#"{"failed":{"code":"BEADS_COOK_FAILED"}}"#
    );
    assert_eq!(BEADS_SCHEMA_V1, "sc-compose/beads/v1");
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "The complete ADR-0021 error-code table is intentionally asserted in one auditable test."
)]
fn every_advertised_error_has_its_stable_code() {
    let path = PathBuf::from("formula.formula.toml");
    let examples = [
        (
            BeadComposeError::UnknownSchema {
                actual: "v0".to_owned(),
            },
            "BEADS_UNKNOWN_SCHEMA",
        ),
        (
            BeadComposeError::FormulaPathNotFile { path: path.clone() },
            "BEADS_FORMULA_NOT_FILE",
        ),
        (
            BeadComposeError::FormulaExtensionUnsupported { path: path.clone() },
            "BEADS_FORMULA_EXTENSION_UNSUPPORTED",
        ),
        (
            BeadComposeError::TemplatePathInvalid { path: path.clone() },
            "BEADS_TEMPLATE_PATH_INVALID",
        ),
        (
            BeadComposeError::TemplateOutsideWorkingDirectory { path: path.clone() },
            "BEADS_TEMPLATE_OUTSIDE_WORKING_DIR",
        ),
        (
            BeadComposeError::OutputOutsideWorkingDirectory { path: path.clone() },
            "BEADS_OUTPUT_OUTSIDE_WORKING_DIR",
        ),
        (
            BeadComposeError::BeadVariableKeyInvalid {
                key: "?".to_owned(),
            },
            "BEADS_VARIABLE_KEY_INVALID",
        ),
        (
            BeadComposeError::BeadVariableKeyDuplicate {
                key: "name".to_owned(),
            },
            "BEADS_VARIABLE_KEY_DUPLICATE",
        ),
        (
            BeadComposeError::FormulaNameRequired,
            "BEADS_FORMULA_NAME_REQUIRED",
        ),
        (
            BeadComposeError::PourAuthorizationRequired,
            "BEADS_POUR_AUTH_REQUIRED",
        ),
        (
            BeadComposeError::PourAuthorizationInvalid,
            "BEADS_POUR_AUTH_INVALID",
        ),
        (
            BeadComposeError::BdUnavailable {
                executable: path.clone(),
            },
            "BEADS_BD_UNAVAILABLE",
        ),
        (
            BeadComposeError::RenderFailed {
                message: "bad template".to_owned(),
            },
            "BEADS_RENDER_FAILED",
        ),
        (
            BeadComposeError::CookFailed {
                exit_status: Some(1),
            },
            "BEADS_COOK_FAILED",
        ),
        (
            BeadComposeError::ActiveRegistryResolutionFailed {
                exit_status: Some(1),
            },
            "BEADS_WHERE_FAILED",
        ),
        (
            BeadComposeError::FormulaOutsideActiveRegistry { path: path.clone() },
            "BEADS_FORMULA_OUTSIDE_ACTIVE_REGISTRY",
        ),
        (
            BeadComposeError::FormulaRegistryAmbiguous {
                formula_name: "sample".to_owned(),
            },
            "BEADS_FORMULA_REGISTRY_AMBIGUOUS",
        ),
        (
            BeadComposeError::PreviewPourFailed {
                exit_status: Some(1),
            },
            "BEADS_PREVIEW_POUR_FAILED",
        ),
        (
            BeadComposeError::PourFailed {
                exit_status: Some(1),
            },
            "BEADS_POUR_FAILED",
        ),
    ];

    for (error, expected_code) in examples {
        assert_eq!(error.code(), expected_code);
    }
}

#[test]
fn duplicate_bead_variables_are_rejected_during_request_deserialization() {
    let request = r#"{
        "schema":"sc-compose/beads/v1",
        "operation":"validate",
        "working_directory":".",
        "template":"example.formula.toml.j2",
        "rendered_formula":"example.formula.toml",
        "compose_variables":{},
        "formula_name":null,
        "bead_variables":{"release":"one","release":"two"},
        "bd_executable":null,
        "pour_authorization":null
    }"#;

    let error = serde_json::from_str::<BeadComposeRequest>(request)
        .expect_err("duplicate Beads variables must not collapse before execution");
    assert!(
        error
            .to_string()
            .contains("duplicate Beads variable key `release`")
    );
}
