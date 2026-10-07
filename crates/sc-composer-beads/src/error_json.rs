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
        let graph = match self {
            Self::GraphParentNotFound { parent } => Some((
                json!({ "parent": parent }),
                "Create the parent or name an existing bead.",
            )),
            Self::GraphIdInvalid { field, value } => Some((
                json!({ "field": field, "value": value }),
                "Correct the identifier according to the named field's grammar.",
            )),
            Self::GraphScopeMismatch { field, value } => Some((
                json!({ "field": field, "value": value }),
                "Make compose_variables agree with the top-level parent and ref.",
            )),
            Self::GraphFormulaUnsupported { reason } => Some((
                json!({ "reason": reason }),
                "Express the unsupported construct in the template or use registry pour.",
            )),
            Self::GraphRelationInvalid { index, reason } => Some((
                json!({ "index": index, "reason": reason }),
                "Correct the relation at the reported zero-based index.",
            )),
            Self::GraphConflict { id, reason } => Some((
                json!({ "id": id, "reason": reason }),
                "Inspect the named bead or use a new ref; existing beads are never edited.",
            )),
            Self::GraphEdgeConflict {
                from,
                to,
                existing,
                requested,
            } => Some((
                json!({ "from": from, "to": to, "existing": existing, "requested": requested }),
                "Inspect the existing edge or change the relation.",
            )),
            Self::GraphEdgeMissing { edges } => Some((
                json!({ "edges": edges }),
                "For each missing edge, run bd dep add <from> <to> --type <type>, then retry.",
            )),
            Self::GraphReadFailed {
                command,
                status,
                cause,
            }
            | Self::GraphApplyFailed {
                command,
                status,
                cause,
            } => Some((
                json!({ "command": command, "status": status, "cause": cause }),
                "Fix the bd failure and retry; nothing was written.",
            )),
            _ => None,
        };
        let (details, recovery) = match graph {
            Some((details, recovery)) => (Some(details), Some(recovery)),
            None => (None, None),
        };
        ErrorEnvelope {
            code: self.code(),
            message: self.to_string(),
            details,
            recovery,
        }
        .serialize(serializer)
    }
}
