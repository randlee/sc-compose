use sc_composer_beads::{
    BeadComposeError as Error, BeadId, GraphConflictReason, GraphDependencyType,
    GraphFormulaUnsupportedReason, GraphIdField, GraphRelationInvalidReason, MissingEdge,
};
use serde_json::json;

#[test]
fn graph_errors_preserve_typed_details_and_recovery() {
    let from = BeadId::new("parent.build").unwrap();
    let to = BeadId::new("parent.test").unwrap();
    let blocks = GraphDependencyType::try_from("blocks".to_owned()).unwrap();
    let related = GraphDependencyType::try_from("related".to_owned()).unwrap();
    let cases = [
        (
            Error::GraphParentNotFound {
                parent: from.clone(),
            },
            json!({"parent":"parent.build"}),
        ),
        (
            Error::GraphIdInvalid {
                field: GraphIdField::Ref,
                value: "bad ref".into(),
            },
            json!({"field":"ref","value":"bad ref"}),
        ),
        (
            Error::GraphScopeMismatch {
                field: GraphIdField::Parent,
                value: "other".into(),
            },
            json!({"field":"parent","value":"other"}),
        ),
        (
            Error::GraphFormulaUnsupported {
                reason: GraphFormulaUnsupportedReason::VarsDeclared,
            },
            json!({"reason":"vars_declared"}),
        ),
        (
            Error::GraphRelationInvalid {
                index: 2,
                reason: GraphRelationInvalidReason::UnknownStep,
            },
            json!({"index":2,"reason":"unknown_step"}),
        ),
        (
            Error::GraphConflict {
                id: from.clone(),
                reason: GraphConflictReason::NotOwned,
            },
            json!({"id":"parent.build","reason":"not_owned"}),
        ),
        (
            Error::GraphEdgeConflict {
                from: from.clone(),
                to: to.clone(),
                existing: related,
                requested: blocks.clone(),
            },
            json!({"from":"parent.build","to":"parent.test","existing":"related","requested":"blocks"}),
        ),
        (
            Error::GraphEdgeMissing {
                edges: vec![
                    MissingEdge {
                        from: from.clone(),
                        to: to.clone(),
                        kind: blocks.clone(),
                    },
                    MissingEdge {
                        from: to,
                        to: from,
                        kind: blocks,
                    },
                ],
            },
            json!({"edges":[{"from":"parent.build","to":"parent.test","type":"blocks"},{"from":"parent.test","to":"parent.build","type":"blocks"}]}),
        ),
        (
            Error::GraphReadFailed {
                command: vec!["bd".into(), "show".into(), "parent.build".into()],
                status: Some(7),
                cause: "malformed JSON".into(),
            },
            json!({"command":["bd","show","parent.build"],"status":7,"cause":"malformed JSON"}),
        ),
        (
            Error::GraphApplyFailed {
                command: vec!["bd".into(), "create".into(), "--graph".into()],
                status: None,
                cause: "bd rejected the graph".into(),
            },
            json!({"command":["bd","create","--graph"],"status":null,"cause":"bd rejected the graph"}),
        ),
    ];
    for (error, details) in cases {
        let serialized = serde_json::to_value(&error).unwrap();
        assert_eq!(serialized["code"], error.code());
        assert_eq!(serialized["message"], error.to_string());
        assert_eq!(serialized["details"], details);
        assert!(!serialized["recovery"].as_str().unwrap().is_empty());
    }
}

#[test]
fn existing_request_error_wire_shape_is_unchanged() {
    let error = Error::UnknownSchema {
        actual: "future".into(),
    };
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        json!({
            "code": "BEADS_UNKNOWN_SCHEMA",
            "message": "unsupported Beads composition schema `future`"
        })
    );
}

#[test]
fn request_read_errors_preserve_native_sources_and_recovery() {
    for kind in [
        std::io::ErrorKind::NotFound,
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::InvalidData,
    ] {
        let error = Error::RequestReadFailed {
            path: "requests/input.json".into(),
            source: std::io::Error::new(kind, "read diagnostic"),
        };
        let document = serde_json::to_value(&error).unwrap();
        assert_eq!(document["code"], "BEADS_REQUEST_READ_FAILED");
        assert_eq!(
            document["details"],
            json!({"path": "requests/input.json", "kind": format!("{kind:?}")})
        );
        assert!(
            document["recovery"]
                .as_str()
                .unwrap()
                .contains("permission")
        );
        assert!(error.to_string().contains("read diagnostic"));
        let source = std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap();
        assert_eq!(source.kind(), kind);
    }
}

#[cfg(unix)]
#[test]
fn output_path_invalid_serializes_non_utf8_path_lossily() {
    use std::os::unix::ffi::OsStringExt;
    let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(
        b"out-\xff.formula.toml".to_vec(),
    ));
    let error = Error::OutputPathInvalid {
        path,
        rule: "parent must exist".into(),
    };
    let document = serde_json::to_value(&error).expect("lossy envelope");
    assert_eq!(document["code"], "BEADS_OUTPUT_PATH_INVALID");
    assert!(
        document["details"]["value"]
            .as_str()
            .unwrap()
            .contains("out-")
    );
}
