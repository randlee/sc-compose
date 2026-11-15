//! Public graph revision options and change descriptions.
use crate::{BeadId, GraphDependencyType, GraphRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Policy for a database with uncommitted or unknown state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UncommittedPolicy {
    /// Refuse unless the database is known clean.
    Refuse,
    /// Permit a revision even when cleanliness cannot be established.
    Allow,
}

/// Identity of the graph to compare or revise.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct GraphRevisionOptions {
    /// The existing graph to revise.
    pub target: ExistingGraphTarget,
    /// How to handle a dirty or unreadable database.
    pub uncommitted: UncommittedPolicy,
}

impl GraphRevisionOptions {
    /// Select an existing graph and its database safety policy.
    #[must_use]
    pub fn new(target: ExistingGraphTarget, uncommitted: UncommittedPolicy) -> Self {
        Self {
            target,
            uncommitted,
        }
    }
}

/// Select revision routing, target, and database policy for a request.
///
/// An explicit pour root overrides the request target. Non-revision operations
/// and untargeted pours return `None` and use the ordinary request executor.
#[must_use]
pub fn revision_options_for_request(
    request: &mut crate::BeadComposeRequest,
    root: Option<BeadId>,
    allow_uncommitted: bool,
) -> Option<GraphRevisionOptions> {
    if !matches!(
        request.operation,
        crate::BeadOperation::Pour
            | crate::BeadOperation::PreviewPour
            | crate::BeadOperation::Repour
            | crate::BeadOperation::PreviewRepour
    ) {
        return None;
    }
    if let Some(root) = root {
        request.parent = Some(root);
        request.ref_ = None;
    }
    let parent = request.parent.clone()?;
    let target = match &request.ref_ {
        Some(reference) => ExistingGraphTarget::Attach {
            parent,
            reference: reference.clone(),
        },
        None => ExistingGraphTarget::PourRoot(parent),
    };
    let policy = if allow_uncommitted {
        UncommittedPolicy::Allow
    } else {
        UncommittedPolicy::Refuse
    };
    Some(GraphRevisionOptions::new(target, policy))
}

/// A plan-owned bead field, with its stable wire name and bd update flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum GraphField {
    /// Bead title.
    Title,
    /// Work description.
    Description,
    /// Design notes.
    Design,
    /// Acceptance criteria.
    AcceptanceCriteria,
    /// Numeric priority.
    Priority,
    /// Additional labels.
    Labels,
    /// Plan metadata, merged with unrelated keys.
    Metadata,
}

impl GraphField {
    pub(crate) const ALL: [Self; 7] = [
        Self::Title,
        Self::Description,
        Self::Design,
        Self::AcceptanceCriteria,
        Self::Priority,
        Self::Labels,
        Self::Metadata,
    ];

    /// Return the stable serialized field name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Description => "description",
            Self::Design => "design",
            Self::AcceptanceCriteria => "acceptance_criteria",
            Self::Priority => "priority",
            Self::Labels => "labels",
            Self::Metadata => "metadata",
        }
    }

    /// Return the corresponding bd update flag, including its leading dashes.
    #[must_use]
    pub const fn as_flag(self) -> &'static str {
        match self {
            Self::AcceptanceCriteria => "--acceptance",
            Self::Labels => "--add-label",
            Self::Title => "--title",
            Self::Description => "--description",
            Self::Design => "--design",
            Self::Priority => "--priority",
            Self::Metadata => "--metadata",
        }
    }
}

/// One user-visible graph revision change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case")]
#[non_exhaustive]
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
        field: GraphField,
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
#[non_exhaustive]
pub struct GraphApplyPartial {
    /// Changes confirmed applied.
    pub applied: Vec<GraphChange>,
    /// Changes still pending or whose outcome is uncertain.
    pub pending: Vec<GraphChange>,
    /// Whether the failed operation may have written unconfirmed changes.
    #[serde(default)]
    pub outcome_uncertain: bool,
    /// Stable code of the underlying error.
    pub cause_code: String,
    /// Failed operation.
    pub failed_operation: String,
    /// Underlying failure.
    pub cause: String,
}

impl GraphApplyPartial {
    /// Describe confirmed work, pending work and the failed operation.
    #[must_use]
    pub fn new(
        applied: Vec<GraphChange>,
        pending: Vec<GraphChange>,
        failed_operation: String,
        cause: String,
        cause_code: String,
        outcome_uncertain: bool,
    ) -> Self {
        Self {
            applied,
            pending,
            outcome_uncertain,
            cause_code,
            failed_operation,
            cause,
        }
    }
}

impl std::fmt::Display for GraphApplyPartial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} changes confirmed applied ({}), {} pending ({}); {} failed ({}): {}",
            self.applied.len(),
            change_summary(&self.applied),
            self.pending.len(),
            change_summary(&self.pending),
            self.failed_operation,
            self.cause_code,
            self.cause
        )?;
        if self.outcome_uncertain {
            f.write_str("; write outcome uncertain, reconcile before retry")?;
        }
        Ok(())
    }
}

pub(crate) fn change_summary(changes: &[GraphChange]) -> String {
    let mut counts = std::collections::BTreeMap::new();
    for change in changes {
        let kind = match change {
            GraphChange::AddNode { .. } => "new nodes",
            GraphChange::AddEdge { .. } => "new edges",
            GraphChange::FieldChange { .. } => "field changes",
            GraphChange::EdgeTypeChange { .. } => "edge type changes",
            GraphChange::RemoveInternalEdge { .. } => "removed internal edges",
            GraphChange::RetainExternalEdge { .. } => "retained external edges",
            GraphChange::MarkRemovedNode { .. } => "removed nodes retained",
        };
        *counts.entry(kind).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(kind, count)| format!("{count} {kind}"))
        .collect::<Vec<_>>()
        .join(", ")
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
