use sc_composer_beads::{
    BeadComposeError as Error, BeadId, GraphConflictReason, GraphDependencyType,
    GraphFormulaUnsupportedReason, GraphRelationInvalidReason, MissingEdge,
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
                field: "ref".into(),
                value: "bad ref".into(),
            },
            json!({"field":"ref","value":"bad ref"}),
        ),
        (
            Error::GraphScopeMismatch {
                field: "parent".into(),
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
