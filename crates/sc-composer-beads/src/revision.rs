//! Public graph revision options and change descriptions.
use crate::{BeadId, GraphDependencyType, GraphRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Policy for a database with uncommitted or unknown state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum UncommittedPolicy {
    /// Refuse unless the database is known clean.
    Refuse,
    /// Permit a revision even when cleanliness cannot be established.
    Allow,
}

/// Identity of the graph to compare or revise.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExistingGraphTarget {
    /// A graph created by pour.
    PourRoot(BeadId),
    /// An attachment under an existing parent.
    Attach {
        /// The existing parent.
        parent: BeadId,
        /// The attachment's stable reference.
        reference: GraphRef,
    },
}

/// Target and safety policy for a graph revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphRevisionOptions {
    /// The existing graph to revise.
    pub target: ExistingGraphTarget,
    /// How to handle a dirty or unreadable database.
    pub uncommitted: UncommittedPolicy,
}

/// One user-visible graph revision change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case")]
pub enum GraphChange {
    /// A missing plan step will be created.
    AddNode {
        /// Formula step identifier.
        step: crate::StepId,
    },
    /// A planned dependency will be added.
    AddEdge {
        /// Dependent endpoint.
        from: crate::GraphEndpoint,
        /// Dependency endpoint.
        to: crate::GraphEndpoint,
        /// Dependency type.
        kind: GraphDependencyType,
    },
    /// A plan-owned field differs from the stored value.
    FieldChange {
        /// Changed bead.
        bead: BeadId,
        /// Existing runtime status.
        status: String,
        /// Changed field name.
        field: String,
        /// Stored field value.
        before: Value,
        /// Desired field value.
        after: Value,
    },
    /// A dependency exists with a different type than requested.
    EdgeTypeChange {
        /// Dependent bead.
        from: BeadId,
        /// Dependency target.
        to: BeadId,
        /// Stored dependency type.
        before: GraphDependencyType,
        /// Requested dependency type.
        after: GraphDependencyType,
    },
    /// An obsolete dependency between graph-owned beads.
    RemoveInternalEdge {
        /// Dependent bead.
        from: BeadId,
        /// Dependency target.
        to: BeadId,
        /// Dependency type.
        kind: GraphDependencyType,
    },
    /// A dependency crossing the graph boundary which remains intact.
    RetainExternalEdge {
        /// Dependent bead.
        from: BeadId,
        /// Dependency target.
        to: BeadId,
        /// Dependency type.
        kind: GraphDependencyType,
    },
    /// An omitted bead retained for separate cleanup.
    MarkRemovedNode {
        /// Retained bead.
        bead: BeadId,
        /// Existing runtime status.
        status: String,
        /// Recommended explicit cleanup command.
        remove_command: String,
    },
}

/// Details of a revision interrupted after one or more writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphApplyPartial {
    /// Changes confirmed applied.
    pub applied: Vec<GraphChange>,
    /// Changes still pending or whose outcome is uncertain.
    pub pending: Vec<GraphChange>,
    /// Failed operation, including an uncertainty indication where applicable.
    pub failed_operation: String,
    /// Underlying failure.
    pub cause: String,
}

/// Closed wire vocabulary for `GraphConflictReason` (ADR-0023).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum GraphConflictReason {
    /// The `not_owned` refusal reason.
    NotOwned,
    /// The `provenance_differs` refusal reason.
    ProvenanceDiffers,
    /// A v1 node lacks verifiable creation history for migration.
    MigrationProofMissing,
    /// The requested plan differs from the existing graph.
    PlanChanged,
}

impl std::fmt::Display for GraphConflictReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotOwned => "not_owned",
            Self::ProvenanceDiffers => "provenance_differs",
            Self::MigrationProofMissing => "migration_proof_missing",
            Self::PlanChanged => "plan_changed",
        })
    }
}
