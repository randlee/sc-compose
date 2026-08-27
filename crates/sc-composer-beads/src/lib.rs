#![deny(missing_docs)]
//! Host-neutral Beads formula composition and validation.
//!
//! This crate owns the versioned `sc-compose/beads/v1` contract and invokes
//! the authoritative `bd` executable without depending on a CLI or adapter.

/// Versioned Beads request and receipt contract types.
pub mod contract;
/// Stable Beads composition error types and codes.
pub mod error;
/// Render-to-`bd` operation staging.
pub mod execute;
/// Fixed-delimiter formula rendering.
pub mod render;
/// Injectable direct-process runner abstraction.
pub mod runner;

#[doc(inline)]
pub use contract::{
    BEADS_SCHEMA_V1, BeadComposeReceipt, BeadComposeRequest, BeadOperation, BeadOutcome, BeadStage,
    BeadStageOutcome, BeadStageReceipt, PourAuthorization, parse_request,
};
#[doc(inline)]
pub use error::BeadComposeError;
#[doc(inline)]
pub use execute::{execute_bead_request, execute_bead_request_with_runner};
#[doc(inline)]
pub use runner::{CommandSpec, ProcessOutput, ProcessRunner, StdProcessRunner};
