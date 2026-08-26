//! Versioned public contract types for Beads composition.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Stable schema identifier for the Beads composition protocol.
pub const BEADS_SCHEMA_V1: &str = "sc-compose/beads/v1";

/// Requested Beads composition operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadOperation {
    /// Render a template without invoking Beads.
    Render,
    /// Render and validate the formula with `bd cook --dry-run`.
    Validate,
    /// Render, validate, and preview `bd mol pour --dry-run`.
    PreviewPour,
    /// Render, validate, and create persistent Beads state when authorized.
    Pour,
}

/// Explicit authorization required for a persistent pour.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PourAuthorization {
    /// Permit exactly one persistent Beads creation operation.
    CreatePersistentBeads,
}

/// Request for one host-neutral Beads composition operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeadComposeRequest {
    /// Must equal [`BEADS_SCHEMA_V1`].
    pub schema: String,
    /// Requested operation.
    pub operation: BeadOperation,
    /// Root that confines template and ordinary output paths.
    pub working_directory: PathBuf,
    /// Input `.formula.toml.j2` or `.formula.json.j2` template path.
    pub template: PathBuf,
    /// Explicit destination `.formula.toml` or `.formula.json` path.
    pub rendered_formula: PathBuf,
    /// Structured values supplied to fixed triple-brace composition expressions.
    pub compose_variables: Map<String, Value>,
    /// Required active-registry formula name for preview and persistent pour.
    pub formula_name: Option<String>,
    /// Sorted scalar variables supplied to Beads as `--var key=value`.
    pub bead_variables: BTreeMap<String, String>,
    /// Optional direct path to the `bd` executable; defaults to `bd`.
    pub bd_executable: Option<PathBuf>,
    /// Required sentinel for [`BeadOperation::Pour`].
    pub pour_authorization: Option<PourAuthorization>,
}

/// Completed host-neutral Beads composition operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeadComposeReceipt {
    /// Always [`BEADS_SCHEMA_V1`].
    pub schema: String,
    /// Requested operation.
    pub operation: BeadOperation,
    /// Normalized absolute rendered formula path.
    pub rendered_formula: PathBuf,
    /// Attempted stage evidence in execution order.
    pub stages: Vec<BeadStageReceipt>,
    /// Final operation result.
    pub outcome: BeadOutcome,
}

/// One discrete execution stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadStage {
    /// Template rendering.
    Render,
    /// `bd cook --dry-run` validation.
    Validate,
    /// `bd where --json` active-registry resolution.
    ResolveActiveRegistry,
    /// `bd mol pour --dry-run` preview.
    PreviewPour,
    /// Authorized persistent `bd mol pour`.
    Pour,
}

/// Outcome of a stage.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadStageOutcome {
    /// Stage completed successfully.
    Succeeded,
    /// Stage was intentionally not needed for the requested operation.
    Skipped,
    /// Stage failed with a stable error code.
    Failed {
        /// Stable `BEADS_*` error code.
        code: String,
    },
}

/// Bounded diagnostic evidence for an attempted stage.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeadStageReceipt {
    /// Stage represented by this evidence.
    pub stage: BeadStage,
    /// Executable and arguments passed directly to the process runner.
    pub argv: Vec<String>,
    /// Exit status when a process ran.
    pub exit_status: Option<i32>,
    /// Wall-clock duration rounded down to milliseconds.
    pub elapsed_ms: u64,
    /// Bounded standard-output evidence.
    pub stdout_excerpt: String,
    /// Bounded standard-error evidence.
    pub stderr_excerpt: String,
    /// Final stage classification.
    pub outcome: BeadStageOutcome,
}

/// Final operation outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeadOutcome {
    /// All requested stages succeeded.
    Succeeded,
    /// A safe precondition refused the operation.
    Refused {
        /// Stable `BEADS_*` error code.
        code: String,
    },
    /// A rendering or external-process stage failed.
    Failed {
        /// Stable `BEADS_*` error code.
        code: String,
    },
}
