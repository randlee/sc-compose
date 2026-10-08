#![deny(missing_docs)]
//! Host-neutral Beads formula composition and validation.
//!
//! This crate owns the versioned `sc-compose/beads/v1` contract and invokes
//! the authoritative `bd` executable without depending on a CLI or adapter.

/// Versioned Beads request and receipt contract types.
pub mod contract;
/// Stable Beads composition error types and codes.
pub mod error;
mod error_json;
/// Render-to-`bd` operation staging.
pub mod execute;
mod graph;
mod pour;
/// Fixed-delimiter formula rendering.
pub mod render;
mod request;
/// Injectable direct-process runner abstraction.
pub mod runner;
mod snapshot;

#[doc(inline)]
pub use contract::{
    BEADS_SCHEMA_V1, BeadComposeReceipt, BeadComposeRequest, BeadDependencyType, BeadDiagnostic,
    BeadEdgeAction, BeadEndpoint, BeadGraph, BeadGraphEdge, BeadGraphMode, BeadGraphNode,
    BeadGraphProvenance, BeadId, BeadNodeAction, BeadOperation, BeadOutcome, BeadPourMode,
    BeadRelation, BeadStage, BeadStageOutcome, BeadStageReceipt, DependencyName, FormulaName,
    GraphDependencyType, GraphEndpoint, GraphRef, MissingEdge, PROVENANCE_KEY, PourAuthorization,
    RefusedBeadComposeReceipt, RequestParseOutcome, Sha256Digest, StepId, parse_relations,
    parse_request, parse_request_for_operation, parse_request_with_outcome,
};
#[doc(inline)]
pub use error::{
    BeadComposeError, GraphConflictReason, GraphFormulaUnsupportedReason, GraphIdField,
    GraphRelationInvalidReason,
};
#[doc(inline)]
pub use execute::{
    execute_bead_request, execute_bead_request_with_diagnostics, execute_bead_request_with_runner,
};
#[doc(inline)]
pub use runner::{CommandSpec, ProcessOutput, ProcessRunner, StdProcessRunner};
