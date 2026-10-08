//! Stable `sc-compose/beads/v1` contract coverage.

use std::path::PathBuf;

use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeError, BeadComposeReceipt, BeadDependencyType, BeadEdgeAction,
    BeadEndpoint, BeadGraph, BeadGraphMode, BeadGraphProvenance, BeadId, BeadNodeAction,
    BeadOperation, BeadOutcome, BeadPourMode, BeadRelation, BeadStage, BeadStageOutcome,
    DependencyName, FormulaName, GraphConflictReason, GraphFormulaUnsupportedReason, GraphIdField,
    GraphRef, GraphRelationInvalidReason, MissingEdge, PROVENANCE_KEY, Sha256Digest, StepId,
    parse_request,
};

#[test]
fn shared_enum_names_match_serde_wire_names() {
    fn wire_name(value: &serde_json::Value) -> &str {
        value.as_str().unwrap_or_else(|| {
            value
                .as_object()
                .expect("tagged outcome")
                .keys()
                .next()
                .expect("variant")
        })
    }
    for operation in [
        BeadOperation::Render,
        BeadOperation::Validate,
        BeadOperation::PreviewPour,
        BeadOperation::Pour,
        BeadOperation::PreviewAttach,
        BeadOperation::Attach,
    ] {
        let wire = serde_json::to_value(operation).expect("operation serializes");
        assert_eq!(operation.as_str(), wire_name(&wire));
        assert_eq!(
            serde_json::from_value::<BeadOperation>(wire).expect("parse operation"),
            operation
        );
    }
    for stage in [
        BeadStage::Render,
        BeadStage::Validate,
        BeadStage::ResolveActiveRegistry,
        BeadStage::PreviewPour,
        BeadStage::Pour,
        BeadStage::PreviewAttach,
        BeadStage::Attach,
    ] {
        assert_eq!(
            stage.as_str(),
            wire_name(&serde_json::to_value(stage).expect("stage serializes"))
        );
    }
    for outcome in [
        BeadStageOutcome::Succeeded,
        BeadStageOutcome::Skipped,
        BeadStageOutcome::Failed {
            code: "test-code".into(),
        },
    ] {
        assert_eq!(
            outcome.as_str(),
            wire_name(&serde_json::to_value(&outcome).expect("stage outcome serializes"))
        );
    }
    for outcome in [
        BeadOutcome::Succeeded,
        BeadOutcome::Refused {
            code: "test-code".into(),
        },
        BeadOutcome::Failed {
            code: "test-code".into(),
        },
    ] {
        assert_eq!(
            outcome.as_str(),
            wire_name(&serde_json::to_value(&outcome).expect("outcome serializes"))
        );
    }
}

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
            BeadComposeError::RequestReadFailed {
                path: path.clone(),
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            },
            "BEADS_REQUEST_READ_FAILED",
        ),
        (
            BeadComposeError::RequestDeserializationFailed {
                message: "invalid JSON".to_owned(),
            },
            "BEADS_REQUEST_DESERIALIZATION_FAILED",
        ),
        (
            BeadComposeError::RelationEndpointInvalid {
                value: "build".into(),
            },
            "BEADS_RELATION_ENDPOINT_INVALID",
        ),
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
            BeadComposeError::OutputPathSymlink { path: path.clone() },
            "BEADS_OUTPUT_PATH_SYMLINK",
        ),
        (
            BeadComposeError::PathNotUtf8 { path: path.clone() },
            "BEADS_PATH_NOT_UTF8",
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
            BeadComposeError::BeadVariableValueInvalid {
                key: "name".to_owned(),
                value: "bad\0value".to_owned(),
            },
            "BEADS_VARIABLE_VALUE_INVALID",
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
            BeadComposeError::ProcessArgumentInvalid {
                executable: path.clone(),
                message: "invalid argument".to_owned(),
            },
            "BEADS_PROCESS_ARGUMENT_INVALID",
        ),
        (
            BeadComposeError::ProcessOutputLimitExceeded {
                stage: BeadStage::Validate,
                limit_bytes: 64 * 1024,
            },
            "BEADS_PROCESS_OUTPUT_LIMIT",
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
                cause: "invalid formula".to_owned(),
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
        (
            BeadComposeError::GraphParentNotFound {
                parent: bead("proj-42"),
            },
            "BEADS_GRAPH_PARENT_NOT_FOUND",
        ),
        (
            BeadComposeError::GraphIdInvalid {
                field: GraphIdField::Step,
                value: "bad-id".into(),
            },
            "BEADS_GRAPH_ID_INVALID",
        ),
        (
            BeadComposeError::GraphScopeMismatch {
                field: GraphIdField::Ref,
                value: "other".into(),
            },
            "BEADS_GRAPH_SCOPE_MISMATCH",
        ),
        (
            BeadComposeError::GraphFormulaUnsupported {
                reason: GraphFormulaUnsupportedReason::VarsDeclared,
            },
            "BEADS_GRAPH_FORMULA_UNSUPPORTED",
        ),
        (
            BeadComposeError::GraphRelationInvalid {
                index: 0,
                reason: GraphRelationInvalidReason::SelfEdge,
            },
            "BEADS_GRAPH_RELATION_INVALID",
        ),
        (
            BeadComposeError::GraphConflict {
                id: bead("proj-42.release-build"),
                reason: GraphConflictReason::NotOwned,
            },
            "BEADS_GRAPH_CONFLICT",
        ),
        (
            BeadComposeError::GraphEdgeConflict {
                from: bead("proj-42"),
                to: bead("proj-3"),
                existing: sc_composer_beads::BeadDependencyType::Related.into(),
                requested: sc_composer_beads::BeadDependencyType::Blocks.into(),
            },
            "BEADS_GRAPH_EDGE_CONFLICT",
        ),
        (
            BeadComposeError::GraphEdgeMissing {
                edges: vec![MissingEdge {
                    from: bead("proj-42"),
                    to: bead("proj-3"),
                    kind: sc_composer_beads::BeadDependencyType::Blocks.into(),
                }],
            },
            "BEADS_GRAPH_EDGE_MISSING",
        ),
        (
            BeadComposeError::GraphReadFailed {
                command: vec!["bd".into(), "show".into(), "proj-42".into()],
                status: Some(1),
                cause: "bad JSON".to_owned(),
            },
            "BEADS_GRAPH_READ_FAILED",
        ),
        (
            BeadComposeError::GraphApplyFailed {
                command: vec![
                    "bd".into(),
                    "create".into(),
                    "--graph".into(),
                    "plan.json".into(),
                ],
                status: None,
                cause: "missing ids".to_owned(),
            },
            "BEADS_GRAPH_APPLY_FAILED",
        ),
    ];

    for (error, expected_code) in examples {
        assert_eq!(error.code(), expected_code);
    }
}

#[test]
fn duplicate_bead_variables_are_rejected_with_a_stable_contract_error() {
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

    let error = parse_request(request)
        .expect_err("duplicate Beads variables must not collapse before execution");
    assert!(matches!(
        error,
        BeadComposeError::BeadVariableKeyDuplicate { ref key } if key == "release"
    ));
    assert_eq!(error.code(), "BEADS_VARIABLE_KEY_DUPLICATE");
}

#[test]
fn malformed_request_json_has_a_stable_contract_error() {
    let error = parse_request("{").expect_err("malformed request JSON must be rejected");
    assert_eq!(error.code(), "BEADS_REQUEST_DESERIALIZATION_FAILED");
}

fn bead(id: &str) -> BeadId {
    BeadId::new(id).expect("bead id")
}

fn round_trip<T>(wire: serde_json::Value) -> T
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let expected = wire.clone();
    let value: T = serde_json::from_value(wire).expect("parse normative shape");
    assert_eq!(serde_json::to_value(&value).expect("serialize"), expected);
    value
}

#[test]
fn graph_contract_serializes_adr_0023_shapes() {
    use serde_json::json;
    round_trip::<BeadId>(json!("proj-42"));
    round_trip::<GraphRef>(json!("qa1-f1-r1"));
    round_trip::<StepId>(json!("build_1"));
    let digest = format!("sha256:{}", "a".repeat(64));
    round_trip::<Sha256Digest>(json!(digest));
    round_trip::<BeadEndpoint>(json!("step:build_1"));
    round_trip::<BeadEndpoint>(json!("bead:proj-42"));
    round_trip::<BeadRelation>(json!({"from":"step:build_1","to":"bead:proj-42","type":"blocks"}));
    round_trip::<BeadGraph>(json!({
        "mode":"attach", "parent":"proj-42", "ref":"qa1-f1-r1",
        "formula":"release", "revision":digest,
        "plan_path":"release.formula.toml.graph.json",
        "ids":{"build_1":"proj-42.qa1-f1-r1-build_1"},
        "nodes":[{"step":"build_1","id":"proj-42.qa1-f1-r1-build_1","action":"existing"}],
        "edges":[{"from":"proj-42.qa1-f1-r1-build_1","to":"proj-3","type":"blocks","action":"existing"}]
    }));
    round_trip::<MissingEdge>(json!({"from":"proj-42","to":"proj-3","type":"blocks"}));
    round_trip::<BeadGraphProvenance>(json!({
        "v":1,"mode":"attach","formula":"release","revision":digest,"inputs":digest,
        "parent":"proj-42","ref":"qa1-f1-r1","step":"build_1"
    }));
    assert_eq!(PROVENANCE_KEY, "sc_compose_graph");
    for mode in ["registry", "graph"] {
        round_trip::<BeadPourMode>(json!(mode));
    }
    for mode in ["pour", "attach"] {
        round_trip::<BeadGraphMode>(json!(mode));
    }
    for action in ["create", "created", "existing"] {
        round_trip::<BeadNodeAction>(json!(action));
    }
    for action in ["add", "added", "existing"] {
        round_trip::<BeadEdgeAction>(json!(action));
    }
    for kind in [
        "blocks",
        "conditional-blocks",
        "waits-for",
        "related",
        "discovered-from",
        "replies-to",
        "relates-to",
        "duplicates",
        "supersedes",
        "authored-by",
        "assigned-to",
        "approved-by",
        "attests",
        "tracks",
        "until",
        "caused-by",
        "validates",
        "delegated-from",
    ] {
        round_trip::<BeadDependencyType>(json!(kind));
    }
    serde_json::from_value::<BeadDependencyType>(json!("parent-child"))
        .expect_err("invalid contract value");
    serde_json::from_value::<BeadDependencyType>(json!("invented"))
        .expect_err("invalid contract value");
}

#[test]
fn identifier_validation_rejects_bad_values_at_rust_and_json_boundaries() {
    use serde_json::json;
    for value in ["", "a b", "a\nb", "a\tb", "a\u{2003}b"] {
        BeadId::new(value).expect_err("invalid contract value");
        serde_json::from_value::<BeadId>(json!(value)).expect_err("invalid contract value");
    }
    for value in ["", ".", "é", "a b", &"a".repeat(33)] {
        let error = GraphRef::new(value).expect_err("invalid ref");
        assert!(error.to_string().contains("ref"));
        assert!(error.to_string().contains("ADR-0023"));
        serde_json::from_value::<GraphRef>(json!(value)).expect_err("invalid contract value");
    }
    for value in ["", "a-b", "a.b", "é", &"a".repeat(65)] {
        StepId::new(value).expect_err("invalid contract value");
        serde_json::from_value::<StepId>(json!(value)).expect_err("invalid contract value");
    }
    GraphRef::new("a".repeat(32)).expect("valid boundary value");
    StepId::new("a".repeat(64)).expect("valid boundary value");
    for value in [
        "a".repeat(64),
        format!("sha256:{}", "A".repeat(64)),
        format!("sha256:{}", "g".repeat(64)),
        format!("sha256:{}", "a".repeat(63)),
    ] {
        Sha256Digest::new(&value).expect_err("invalid contract value");
        serde_json::from_value::<Sha256Digest>(json!(value)).expect_err("invalid contract value");
    }
    for endpoint in ["step:", "step:bad-id", "bead:", "other:proj-42", "proj-42"] {
        serde_json::from_value::<BeadEndpoint>(json!(endpoint))
            .expect_err("invalid contract value");
    }
}

#[test]
fn invalid_identifier_messages_name_only_the_failing_fields_rule() {
    let cases = [
        (
            GraphIdField::Bead,
            "a b",
            BeadId::new("a b").expect_err("invalid bead"),
            "bead ids are non-empty without whitespace",
        ),
        (
            GraphIdField::Ref,
            ".",
            GraphRef::new(".").expect_err("invalid ref"),
            "ref is [A-Za-z0-9_-]{1,32}",
        ),
        (
            GraphIdField::Step,
            "a-b",
            StepId::new("a-b").expect_err("invalid step"),
            "step is [A-Za-z0-9_]{1,64}; hyphens are forbidden",
        ),
        (
            GraphIdField::Digest,
            "bad",
            Sha256Digest::new("bad").expect_err("invalid digest"),
            "digest is sha256: plus exactly 64 lowercase hex digits",
        ),
        (
            GraphIdField::Formula,
            "../bad",
            FormulaName::new("../bad").expect_err("invalid formula"),
            "formula contains ASCII letters, digits, underscores, dots and hyphens; no leading dot/hyphen or consecutive dots",
        ),
        (
            GraphIdField::DependencyType,
            "1bad",
            DependencyName::new("1bad").expect_err("invalid dependency type"),
            "dependency_type starts with an ASCII letter followed by ASCII letters, digits, underscores or hyphens",
        ),
    ];
    for (field, value, error, rule) in cases {
        let expected = format!("invalid graph {field} `{value}`; follow ADR-0023: {rule}");
        assert!(
            matches!(&error, BeadComposeError::GraphIdInvalid { field: actual, .. } if *actual == field)
        );
        assert_eq!(
            serde_json::to_value(&error).expect("error JSON")["details"]["field"],
            field.as_str()
        );
        assert_eq!(error.to_string(), expected);
        assert_eq!(
            serde_json::to_value(&error).expect("error JSON")["message"],
            expected
        );
    }
}

#[test]
fn digests_use_the_composer_hash_contract() {
    use sc_composer::{HashInput, calculate_hash};
    let hash = |bytes| {
        calculate_hash(HashInput::TextFileBytes {
            utf8_file_bytes: bytes,
        })
        .expect("UTF-8 hash")
    };
    let lf = hash(b"line one\nline two\n");
    assert_eq!(lf, hash(b"line one\r\nline two\r"));
    let digest = Sha256Digest::new(format!("sha256:{}", lf.template())).expect("canonical digest");
    assert_eq!(digest.as_str(), format!("sha256:{}", lf.template()));
}

#[test]
fn graph_identifier_field_vocabulary_is_closed_and_preserves_wire_strings() {
    for (field, name) in [
        (GraphIdField::Bead, "bead"),
        (GraphIdField::Ref, "ref"),
        (GraphIdField::Step, "step"),
        (GraphIdField::Digest, "digest"),
        (GraphIdField::Formula, "formula"),
        (GraphIdField::DependencyType, "dependency_type"),
        (GraphIdField::Parent, "parent"),
    ] {
        assert_eq!(field.to_string(), name);
        assert_eq!(field.as_str(), name);
        assert_eq!(
            serde_json::to_value(field).expect("field serializes"),
            serde_json::json!(name)
        );
        assert_eq!(
            serde_json::from_value::<GraphIdField>(serde_json::json!(name)).expect("known field"),
            field
        );
    }
    for invalid in ["unknown", "dependency-type", "Ref", ""] {
        serde_json::from_value::<GraphIdField>(serde_json::json!(invalid))
            .expect_err("unknown field");
    }
    for field in [GraphIdField::Parent, GraphIdField::Ref] {
        let error = BeadComposeError::GraphScopeMismatch {
            field,
            value: "other".into(),
        };
        assert_eq!(
            error.to_string(),
            format!(
                "graph scope {field} disagrees with `other`; make compose_variables agree with the top-level parent/ref"
            )
        );
        assert_eq!(
            serde_json::to_value(&error).expect("error JSON")["details"]["field"],
            field.as_str()
        );
    }
}

#[test]
fn graph_reason_vocabulary_is_closed_and_prints_the_wire_value() {
    use serde_json::json;
    for value in ["not_owned", "provenance_differs"] {
        assert_eq!(
            round_trip::<GraphConflictReason>(json!(value)).to_string(),
            value
        );
    }
    for value in [
        "unknown_step",
        "self_edge",
        "no_step",
        "duplicate",
        "parent_pair",
        "bead_not_found",
        "registry_pour",
    ] {
        assert_eq!(
            round_trip::<GraphRelationInvalidReason>(json!(value)).to_string(),
            value
        );
    }
    for value in [
        "not_utf8",
        "vars_declared",
        "bead_variables_set",
        "composition",
        "unknown_key",
        "step_construct",
        "reserved_metadata",
        "label_comma",
        "step_graph",
    ] {
        assert_eq!(
            round_trip::<GraphFormulaUnsupportedReason>(json!(value)).to_string(),
            value
        );
    }
    serde_json::from_value::<GraphConflictReason>(json!("unknown"))
        .expect_err("invalid contract value");
    serde_json::from_value::<GraphRelationInvalidReason>(json!("unknown"))
        .expect_err("invalid contract value");
    serde_json::from_value::<GraphFormulaUnsupportedReason>(json!("unknown"))
        .expect_err("invalid contract value");
}

#[test]
fn missing_edge_error_lists_each_repair_in_plan_order() {
    let error = BeadComposeError::GraphEdgeMissing {
        edges: vec![
            MissingEdge {
                from: bead("proj-42"),
                to: bead("proj-3"),
                kind: sc_composer_beads::BeadDependencyType::Blocks.into(),
            },
            MissingEdge {
                from: bead("proj-9"),
                to: bead("proj-42"),
                kind: sc_composer_beads::BeadDependencyType::Validates.into(),
            },
        ],
    };
    assert_eq!(
        error.to_string(),
        "graph edges missing; repair then retry: bd dep add proj-42 proj-3 --type blocks; bd dep add proj-9 proj-42 --type validates"
    );
}

#[test]
fn phase_r_request_defaults_and_receipt_bytes_are_unchanged() {
    let request = include_str!("fixtures/beads/request.json");
    let request = parse_request(request).expect("Phase R request");
    assert!(request.parent.is_none());
    assert!(request.ref_.is_none());
    assert!(request.relations.is_empty());
    let old = r#"{"schema":"sc-compose/beads/v1","operation":"render","rendered_formula":"sample.formula.toml","stages":[],"outcome":"succeeded"}"#;
    let receipt: BeadComposeReceipt = serde_json::from_str(old).expect("Phase R receipt");
    assert!(receipt.pour_mode.is_none());
    assert!(receipt.graph.is_none());
    assert!(receipt.missing_edges.is_empty());
    assert_eq!(serde_json::to_string(&receipt).expect("serialize"), old);
}

#[test]
fn missing_edges_receipt_field_round_trips_as_typed_edges() {
    use serde_json::json;

    let wire = json!({
        "schema": BEADS_SCHEMA_V1,
        "operation": "attach",
        "rendered_formula": "release.formula.toml",
        "stages": [],
        "outcome": {"refused": {"code": "BEADS_GRAPH_EDGE_MISSING"}},
        "missing_edges": [
            {"from":"proj-1.release-verify", "to":"proj-1.release-build", "type":"blocks"},
            {"from":"proj-1.release-publish", "to":"proj-1.release-verify", "type":"validates"}
        ]
    });
    let receipt: BeadComposeReceipt = serde_json::from_value(wire.clone()).expect("receipt");
    assert_eq!(receipt.missing_edges.len(), 2);
    assert_eq!(
        receipt.missing_edges[0].from.as_str(),
        "proj-1.release-verify"
    );
    assert_eq!(serde_json::to_value(&receipt).expect("receipt JSON"), wire);
}

#[test]
fn captured_parser_fixtures_and_cross_surface_receipts_deserialize() {
    use std::collections::BTreeSet;

    use serde_json::Value;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/beads/graph");
    let read = |name: &str| std::fs::read_to_string(root.join(name)).expect("fixture file");
    let index: Value = serde_json::from_str(&read("captures.json")).expect("capture index");
    let cases = index["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 27);
    let mut covered = BTreeSet::from(["captures.json".to_owned()]);
    for case in cases {
        let input_name = case["input"].as_str().expect("input");
        covered.insert(input_name.to_owned());
        let input: Value = serde_json::from_str(&read(input_name)).expect("formula JSON");
        assert!(input["formula"].is_string());
        let capture: Value =
            serde_json::from_str(&read(case["capture"].as_str().expect("capture")))
                .expect("capture JSON");
        covered.insert(case["capture"].as_str().expect("capture").to_owned());
        if let Some(cooked) = case["cooked"].as_str() {
            covered.insert(cooked.to_owned());
            assert_eq!(capture["exit_status"], 0);
            let raw = read(cooked);
            assert_eq!(raw, capture["stdout"].as_str().expect("captured stdout"));
            let parsed: Value = serde_json::from_str(&raw).expect("bd parser JSON");
            assert_eq!(parsed["schema_version"], 1);
        } else {
            assert_eq!(capture["exit_status"], 1);
            assert!(!capture["stderr"].as_str().expect("stderr").is_empty());
        }
    }
    for (file, code) in [
        ("receipt-graph-pour.json", None),
        ("receipt-registry.json", None),
        ("receipt-conflict.json", Some("BEADS_GRAPH_CONFLICT")),
        (
            "receipt-edge-missing.json",
            Some("BEADS_GRAPH_EDGE_MISSING"),
        ),
    ] {
        covered.insert(file.to_owned());
        let wire: Value = serde_json::from_str(&read(file)).expect("receipt JSON");
        let receipt: BeadComposeReceipt =
            serde_json::from_value(wire.clone()).expect("receipt contract");
        assert_eq!(
            serde_json::to_value(&receipt).expect("serialize receipt"),
            wire
        );
        assert_eq!(
            receipt.outcome,
            code.map_or(BeadOutcome::Succeeded, |code| BeadOutcome::Refused {
                code: code.into()
            })
        );
    }
    for support in ["base.formula.json", "exp.formula.json"] {
        let formula: Value = serde_json::from_str(&read(support)).expect("support formula JSON");
        assert!(formula["formula"].is_string());
        covered.insert(support.to_owned());
    }
    let actual = std::fs::read_dir(&root)
        .expect("graph fixture directory")
        .map(|entry| {
            entry
                .expect("fixture entry")
                .file_name()
                .into_string()
                .expect("UTF-8 fixture name")
        })
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual, covered,
        "every graph JSON fixture must be indexed, a receipt, or named support"
    );
}

#[test]
fn graph_without_missing_nodes_omits_plan_path() {
    let mut graph: BeadGraph = serde_json::from_value(serde_json::json!({
        "mode":"attach","parent":"proj-42","ref":"chain","formula":"release",
        "revision":format!("sha256:{}", "a".repeat(64)),"ids":{},"nodes":[],"edges":[]
    }))
    .expect("no-op graph");
    assert!(graph.plan_path.is_none());
    assert!(
        serde_json::to_value(&graph)
            .expect("serialize")
            .get("plan_path")
            .is_none()
    );
    graph.plan_path = Some(PathBuf::from("plan.json"));
    assert_eq!(
        serde_json::to_value(&graph).expect("serialize")["plan_path"],
        "plan.json"
    );
}

#[test]
fn graph_edge_types_validate_strings_without_changing_wire_format() {
    use sc_composer_beads::{DependencyName, GraphDependencyType, GraphEndpoint};
    use serde_json::json;
    for endpoint in ["proj-42", "step:build", "_root"] {
        round_trip::<GraphEndpoint>(json!(endpoint));
    }
    for kind in ["blocks", "parent-child", "custom-audit_1"] {
        round_trip::<GraphDependencyType>(json!(kind));
        round_trip::<MissingEdge>(json!({"from":"proj-42","to":"proj-3","type":kind}));
    }
    for endpoint in ["", "two ids", "step:", "step:bad-step", "step:a.b"] {
        serde_json::from_value::<GraphEndpoint>(json!(endpoint)).expect_err("invalid endpoint");
    }
    for kind in [
        "",
        "blocks;echo",
        "blocks --force",
        "$(echo)",
        "--force",
        "related\nclose",
    ] {
        serde_json::from_value::<GraphDependencyType>(json!(kind))
            .expect_err("invalid dependency token");
        DependencyName::new(kind).expect_err("cannot bypass validation in Rust");
        serde_json::from_value::<MissingEdge>(json!({"from":"proj-42","to":"proj-3","type":kind}))
            .expect_err("repair command kind must be validated");
    }
    let custom = GraphDependencyType::try_from("custom-audit_1".to_owned()).expect("custom kind");
    let error = BeadComposeError::GraphEdgeMissing {
        edges: vec![MissingEdge {
            from: bead("proj-42"),
            to: bead("proj-3"),
            kind: custom,
        }],
    };
    assert!(
        error
            .to_string()
            .ends_with("bd dep add proj-42 proj-3 --type custom-audit_1")
    );

    let step = BeadEndpoint::Step(StepId::new("build").expect("valid step"));
    let bead = BeadEndpoint::Bead(BeadId::new("proj-42").expect("valid bead"));
    assert_eq!(step.to_string(), "step:build");
    assert_eq!(bead.to_string(), "bead:proj-42");
    assert_eq!(String::from(step), "step:build");
    assert_eq!(String::from(bead), "bead:proj-42");
    assert_eq!(
        GraphEndpoint::Step(StepId::new("build").unwrap()).to_string(),
        "step:build"
    );
    assert_eq!(
        GraphEndpoint::Bead(BeadId::new("proj-42").unwrap()).to_string(),
        "proj-42"
    );
}

#[test]
fn graph_identifier_types_borrow_as_str() {
    use sc_composer_beads::{BeadId, FormulaName, GraphRef, Sha256Digest, StepId};

    let digest = format!("sha256:{}", "a".repeat(64));
    assert_eq!(
        AsRef::<str>::as_ref(&FormulaName::new("formula").unwrap()),
        "formula"
    );
    assert_eq!(
        AsRef::<str>::as_ref(&BeadId::new("proj-42").unwrap()),
        "proj-42"
    );
    assert_eq!(
        AsRef::<str>::as_ref(&GraphRef::new("ref-1").unwrap()),
        "ref-1"
    );
    assert_eq!(
        AsRef::<str>::as_ref(&StepId::new("build_1").unwrap()),
        "build_1"
    );
    assert_eq!(
        AsRef::<str>::as_ref(&Sha256Digest::new(digest.clone()).unwrap()),
        digest
    );
}

#[test]
fn formula_names_are_validated_in_requests_and_graph_metadata() {
    use sc_composer_beads::FormulaName;
    use serde_json::{Value, json};
    for name in ["release", "release-1_2", "release.v1", "_private"] {
        round_trip::<FormulaName>(json!(name));
        assert_eq!(FormulaName::new(name).expect("name").as_str(), name);
    }
    let request: Value =
        serde_json::from_str(include_str!("fixtures/beads/request.json")).expect("request");
    let digest = format!("sha256:{}", "a".repeat(64));
    for name in [
        "",
        ".",
        "..",
        "a..b",
        "../release",
        "/absolute",
        "nested/name",
        r"nested\name",
        "C:name",
        "--help",
        "two names",
        "nul\0name",
    ] {
        FormulaName::new(name).expect_err("invalid Rust name");
        let mut invalid = request.clone();
        invalid["formula_name"] = json!(name);
        let error = parse_request(&invalid.to_string()).expect_err("invalid wire name");
        assert_eq!(error.code(), "BEADS_REQUEST_DESERIALIZATION_FAILED");
        serde_json::from_value::<BeadGraph>(json!({
            "mode":"pour","parent":null,"ref":null,"formula":name,"revision":digest,
            "ids":{},"nodes":[],"edges":[]
        }))
        .expect_err("graph name validated");
        serde_json::from_value::<BeadGraphProvenance>(json!({
            "v":1,"mode":"pour","formula":name,"revision":digest,"inputs":digest,
            "parent":null,"ref":null,"step":null
        }))
        .expect_err("provenance name validated");
    }
}

#[test]
fn invalid_authorization_emits_the_stable_code_from_real_parsing() {
    use serde_json::{Value, json};
    let original: Value =
        serde_json::from_str(include_str!("fixtures/beads/request.json")).expect("request");
    for value in [
        json!("invalid"),
        json!(""),
        json!("createpersistentbeads"),
        json!(true),
        json!(1),
        json!([]),
        json!({"CreatePersistentBeads":null}),
    ] {
        let mut request = original.clone();
        request["pour_authorization"] = value;
        let error = parse_request(&request.to_string()).expect_err("invalid authorization");
        assert!(matches!(error, BeadComposeError::PourAuthorizationInvalid));
        assert_eq!(error.code(), "BEADS_POUR_AUTH_INVALID");
    }
    let mut request = original;
    request["pour_authorization"] = json!("CreatePersistentBeads");
    assert_eq!(
        parse_request(&request.to_string())
            .expect("authorized")
            .pour_authorization,
        Some(sc_composer_beads::PourAuthorization::CreatePersistentBeads)
    );
    request["pour_authorization"] = Value::Null;
    assert_eq!(
        parse_request(&request.to_string())
            .expect("absent sentinel")
            .pour_authorization,
        None
    );
    request
        .as_object_mut()
        .expect("object")
        .remove("pour_authorization");
    assert_eq!(
        parse_request(&request.to_string())
            .expect("omitted sentinel")
            .pour_authorization,
        None
    );
    assert!(matches!(
        sc_composer_beads::PourAuthorization::try_from("invalid"),
        Err(BeadComposeError::PourAuthorizationInvalid)
    ));
}

#[test]
fn duplicate_keys_retain_exact_decoded_text_without_parsing_diagnostics() {
    let mut request: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/beads/request.json")).unwrap();
    request["bead_variables"] = serde_json::Value::Null;
    for key in [
        "release at line 17 column 9",
        "quote\"newline\n",
        "control\u{1f}",
        "é",
    ] {
        let encoded = serde_json::to_string(key).unwrap();
        let input = request.to_string().replace(
            "\"bead_variables\":null",
            &format!("\"bead_variables\":{{{encoded}:\"one\",{encoded}:\"two\"}}"),
        );
        assert!(matches!(parse_request(&input),
            Err(BeadComposeError::BeadVariableKeyDuplicate { key: actual }) if actual == key));
        serde_json::from_str::<sc_composer_beads::BeadComposeRequest>(&input)
            .expect_err("direct serde rejects duplicates too");
    }
    let input = request.to_string().replace(
        "\"bead_variables\":null",
        r#""bead_variables":{"a":"one","\u0061":"two"}"#,
    );
    assert!(matches!(parse_request(&input),
        Err(BeadComposeError::BeadVariableKeyDuplicate { key }) if key == "a"));
    assert_eq!(
        parse_request(&(input + " trailing")).unwrap_err().code(),
        "BEADS_REQUEST_DESERIALIZATION_FAILED"
    );
}

#[test]
fn malformed_endpoint_prefixes_have_a_stable_request_code() {
    use sc_composer_beads::{BeadEndpoint, parse_relations};
    use serde_json::json;
    let mut request: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/beads/request.json")).unwrap();
    for endpoint in ["", "build", "other:build", "Step:build"] {
        let direct = BeadEndpoint::try_from(endpoint.to_owned()).unwrap_err();
        assert_eq!(direct.code(), "BEADS_RELATION_ENDPOINT_INVALID");
        for field in ["from", "to"] {
            let mut relations = json!([{"from":"step:build","to":"bead:parent","type":"blocks"}]);
            relations[0][field] = json!(endpoint);
            assert_eq!(
                parse_relations(relations.clone()).unwrap_err().code(),
                direct.code()
            );
            request["relations"] = relations;
            assert!(matches!(parse_request(&request.to_string()),
                Err(BeadComposeError::RelationEndpointInvalid { value }) if value == endpoint));
        }
    }
}

// FUZZ-013: Phase R requests parse unchanged (ADR-0023 Decision 1). A render
// request whose formula_name bd accepts, such as `café`, was valid before the
// graph grammar existed and must still parse.
#[test]
#[ignore = "FUZZ-013"]
fn fuzz_013_phase_r_render_request_keeps_accepting_its_formula_name() {
    for name in ["café", "re g0"] {
        let request = serde_json::json!({
            "schema": BEADS_SCHEMA_V1,
            "operation": "render",
            "working_directory": "/work",
            "template": "f.formula.toml.j2",
            "rendered_formula": "/work/build/f.formula.toml",
            "formula_name": name,
            "compose_variables": {},
            "bead_variables": {}
        });
        let parsed = parse_request(&request.to_string());
        assert!(parsed.is_ok(), "{name}: {:?}", parsed.err());
    }
}
