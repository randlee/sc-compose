//! Shared adapter serialization for stable errors and graph recovery details.

use serde::{Serialize, Serializer};
use serde_json::{Value, json};

use crate::BeadComposeError;

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    code: &'a str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery: Option<&'a str>,
}

impl Serialize for BeadComposeError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        error_envelope(self).serialize(serializer)
    }
}

fn error_envelope(error: &BeadComposeError) -> ErrorEnvelope<'static> {
    let context = match error {
        BeadComposeError::RequestReadFailed { path, source } => Some((
            json!({ "path": path.to_string_lossy(), "kind": format!("{:?}", source.kind()) }),
            "Check that the request path names an existing UTF-8 JSON file and that you have permission to read it.",
        )),
        BeadComposeError::OutputPathInvalid { path, rule } => Some((
            json!({ "field": "rendered_formula", "value": path.to_string_lossy(), "rule": rule }),
            "Choose a rendered_formula path whose parent directory exists inside working_directory.",
        )),
        BeadComposeError::GraphParentNotFound { parent } => Some((
            json!({ "parent": parent }),
            "Create the parent or name an existing bead.",
        )),
        BeadComposeError::GraphIdInvalid { field, value } => Some((
            json!({ "field": field, "value": value }),
            "Correct the identifier according to the named field's grammar.",
        )),
        BeadComposeError::GraphScopeMismatch { field, value } => Some((
            json!({ "field": field, "value": value }),
            "Make compose_variables agree with the top-level parent and ref.",
        )),
        BeadComposeError::GraphFormulaUnsupported { reason } => Some((
            json!({ "reason": reason }),
            "Express the unsupported construct in the template or use registry pour.",
        )),
        BeadComposeError::GraphRelationInvalid { index, reason } => Some((
            json!({ "index": index, "reason": reason }),
            "Correct the relation at the reported zero-based index.",
        )),
        BeadComposeError::GraphConflict { id, reason } => Some((
            json!({ "id": id, "reason": reason }),
            "Inspect the named bead or use a new ref; existing beads are never edited.",
        )),
        BeadComposeError::GraphEdgeConflict {
            from,
            to,
            existing,
            requested,
        } => Some((
            json!({ "from": from, "to": to, "existing": existing, "requested": requested }),
            "Inspect the existing edge or change the relation.",
        )),
        BeadComposeError::GraphEdgeMissing { edges } => Some((
            json!({ "edges": edges }),
            "For each missing edge, run bd dep add <from> <to> --type <type>, then retry.",
        )),
        BeadComposeError::GraphReadFailed {
            command,
            status,
            cause,
        }
        | BeadComposeError::GraphApplyFailed {
            command,
            status,
            cause,
        } => Some((
            json!({ "command": command, "status": status, "cause": cause }),
            "Fix the bd failure and retry; nothing was written.",
        )),
        _ => None,
    };
    let (details, recovery) = match context {
        Some((details, recovery)) => (Some(details), Some(recovery)),
        None => (None, None),
    };
    ErrorEnvelope {
        code: error.code(),
        message: error.to_string(),
        details,
        recovery,
    }
}

impl crate::BeadDiagnostic {
    /// Retain a canonical identifier diagnostic, including its native rule.
    /// Returns `None` for errors outside the identifier-refusal contract.
    #[must_use]
    pub fn graph_id_invalid(error: &BeadComposeError) -> Option<Self> {
        let BeadComposeError::GraphIdInvalid { field, .. } = error else {
            return None;
        };
        let mut envelope = error_envelope(error);
        envelope.details.as_mut()?.as_object_mut()?.insert(
            "rule".into(),
            Value::String(crate::error::graph_id_rule(*field).into()),
        );
        Some(Self {
            code: envelope.code.into(),
            message: envelope.message,
            details: envelope.details,
            recovery: envelope.recovery.map(str::to_owned),
        })
    }
}
