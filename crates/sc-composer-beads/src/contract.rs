//! Versioned public contract types for Beads composition.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::de::{Error as _, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use crate::error::BeadComposeError;

/// Stable schema identifier for the Beads composition protocol.
pub const BEADS_SCHEMA_V1: &str = "sc-compose/beads/v1";

const DUPLICATE_BEAD_VARIABLE_PREFIX: &str = "duplicate Beads variable key \u{1f}";

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
    #[serde(deserialize_with = "deserialize_unique_bead_variables")]
    pub bead_variables: BTreeMap<String, String>,
    /// Optional direct path to the `bd` executable; defaults to `bd`.
    pub bd_executable: Option<PathBuf>,
    /// Required sentinel for [`BeadOperation::Pour`].
    pub pour_authorization: Option<PourAuthorization>,
}

fn deserialize_unique_bead_variables<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct UniqueBeadVariables;

    impl<'de> Visitor<'de> for UniqueBeadVariables {
        type Value = BTreeMap<String, String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON object with unique Beads variable keys")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut variables = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if variables.insert(key.clone(), value).is_some() {
                    return Err(A::Error::custom(format!(
                        "{DUPLICATE_BEAD_VARIABLE_PREFIX}{key}"
                    )));
                }
            }
            Ok(variables)
        }
    }

    deserializer.deserialize_map(UniqueBeadVariables)
}

/// Parse a JSON request into the stable Beads composition contract.
///
/// This boundary preserves duplicate runtime-variable keys as
/// [`BeadComposeError::BeadVariableKeyDuplicate`] instead of exposing a
/// serializer-specific error to adapters.
///
/// # Errors
///
/// Returns a stable [`BeadComposeError`] when the input is malformed or does
/// not deserialize into the v1 request contract.
pub fn parse_request(input: &str) -> Result<BeadComposeRequest, BeadComposeError> {
    serde_json::from_str(input).map_err(|error| {
        let message = error.to_string();
        duplicate_bead_variable_key(&message).map_or_else(
            || BeadComposeError::RequestDeserializationFailed { message },
            |key| BeadComposeError::BeadVariableKeyDuplicate { key },
        )
    })
}

fn duplicate_bead_variable_key(message: &str) -> Option<String> {
    message
        .strip_prefix(DUPLICATE_BEAD_VARIABLE_PREFIX)
        .map(|key_with_location| {
            key_with_location
                .split_once(" at line ")
                .map_or(key_with_location, |(key, _)| key)
                .to_owned()
        })
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
