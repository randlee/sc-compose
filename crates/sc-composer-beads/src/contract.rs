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
    /// Existing parent for an attach operation.
    #[serde(default)]
    pub parent: Option<BeadId>,
    /// Attachment identity within the parent.
    #[serde(default, rename = "ref")]
    pub ref_: Option<GraphRef>,
    /// Additional dependencies between steps and existing beads.
    #[serde(default)]
    pub relations: Vec<BeadRelation>,
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
    /// Selected pour mode, absent for operations without a pour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pour_mode: Option<BeadPourMode>,
    /// Graph plan and resulting identities, when using graph construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<BeadGraph>,
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

/// Metadata key identifying graphs constructed by sc-compose (ADR-0023).
pub const PROVENANCE_KEY: &str = "sc_compose_graph";

// The macro keeps validation identical at the Rust and serde boundaries.
macro_rules! graph_string {
    ($name:ident, $field:literal, $rule:literal, $valid:expr) => {
        #[doc = $rule]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Validate a value according to the ADR-0023 identifier rule.
            ///
            /// # Errors
            /// Returns `GraphIdInvalid` when the value violates this type's rule.
            pub fn new(value: impl Into<String>) -> Result<Self, BeadComposeError> {
                let value = value.into();
                if ($valid)(&value) {
                    Ok(Self(value))
                } else {
                    Err(BeadComposeError::GraphIdInvalid {
                        field: $field.to_owned(),
                        value,
                    })
                }
            }

            /// Borrow the validated string.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = BeadComposeError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

graph_string!(
    BeadId,
    "bead",
    "A non-empty bead id containing no whitespace.",
    |s: &str| !s.is_empty() && !s.chars().any(char::is_whitespace)
);
graph_string!(
    GraphRef,
    "ref",
    "An attachment reference matching `[A-Za-z0-9_-]{1,32}`.",
    |s: &str| (1..=32).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
);
graph_string!(
    StepId,
    "step",
    "A step id matching `[A-Za-z0-9_]{1,64}`; hyphens are forbidden.",
    |s: &str| (1..=64).contains(&s.len())
        && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
);
graph_string!(
    Sha256Digest,
    "digest",
    "A `sha256:` prefix followed by exactly 64 lowercase hexadecimal digits.",
    |s: &str| s.strip_prefix("sha256:").is_some_and(|hex| hex.len() == 64
        && hex
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)))
);

/// How a formula is poured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum BeadPourMode {
    /// Existing registry-based pour.
    Registry,
    /// Atomic graph construction.
    Graph,
}

/// Graph construction mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum BeadGraphMode {
    /// Create a new molecule root.
    Pour,
    /// Attach children under an existing parent.
    Attach,
}

/// Well-known bd dependency types, excluding hierarchy-owned parent-child.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum BeadDependencyType {
    /// The `Blocks` dependency type.
    Blocks,
    /// The `ConditionalBlocks` dependency type.
    ConditionalBlocks,
    /// The `WaitsFor` dependency type.
    WaitsFor,
    /// The `Related` dependency type.
    Related,
    /// The `DiscoveredFrom` dependency type.
    DiscoveredFrom,
    /// The `RepliesTo` dependency type.
    RepliesTo,
    /// The `RelatesTo` dependency type.
    RelatesTo,
    /// The `Duplicates` dependency type.
    Duplicates,
    /// The `Supersedes` dependency type.
    Supersedes,
    /// The `AuthoredBy` dependency type.
    AuthoredBy,
    /// The `AssignedTo` dependency type.
    AssignedTo,
    /// The `ApprovedBy` dependency type.
    ApprovedBy,
    /// The `Attests` dependency type.
    Attests,
    /// The `Tracks` dependency type.
    Tracks,
    /// The `Until` dependency type.
    Until,
    /// The `CausedBy` dependency type.
    CausedBy,
    /// The `Validates` dependency type.
    Validates,
    /// The `DelegatedFrom` dependency type.
    DelegatedFrom,
}

/// A step or existing bead endpoint, encoded as `step:<id>` or `bead:<id>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(try_from = "String", into = "String")]
pub enum BeadEndpoint {
    /// A step of the rendered formula.
    Step(StepId),
    /// A bead that already exists.
    Bead(BeadId),
}

impl TryFrom<String> for BeadEndpoint {
    type Error = BeadComposeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Some(id) = value.strip_prefix("step:") {
            return StepId::new(id).map(Self::Step);
        }
        if let Some(id) = value.strip_prefix("bead:") {
            return BeadId::new(id).map(Self::Bead);
        }
        Err(BeadComposeError::RequestDeserializationFailed {
            message: format!("endpoint `{value}` must have a step: or bead: prefix (ADR-0023)"),
        })
    }
}

impl From<BeadEndpoint> for String {
    fn from(endpoint: BeadEndpoint) -> Self {
        match endpoint {
            BeadEndpoint::Step(id) => format!("step:{id}"),
            BeadEndpoint::Bead(id) => format!("bead:{id}"),
        }
    }
}

/// A directed relation requested in addition to formula dependencies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadRelation {
    /// Dependent endpoint.
    pub from: BeadEndpoint,
    /// Dependency endpoint.
    pub to: BeadEndpoint,
    /// Dependency type.
    #[serde(rename = "type")]
    pub kind: BeadDependencyType,
}

/// The planned or applied graph and its resolved bead identities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadGraph {
    /// Construction mode.
    pub mode: BeadGraphMode,
    /// Attach parent, or created pour root.
    pub parent: Option<BeadId>,
    /// Attachment reference, absent for pour.
    #[serde(rename = "ref")]
    pub ref_: Option<GraphRef>,
    /// Formula name.
    pub formula: String,
    /// Hash of normalized rendered formula text.
    pub revision: Sha256Digest,
    /// Graph plan path, absent when nothing must be created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_path: Option<PathBuf>,
    /// Step-to-bead mapping; empty for pour preview.
    pub ids: BTreeMap<StepId, BeadId>,
    /// Nodes in plan order.
    pub nodes: Vec<BeadGraphNode>,
    /// Edges in plan order.
    pub edges: Vec<BeadGraphEdge>,
}

/// One planned or applied graph node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadGraphNode {
    /// Formula step, or None for the pour root.
    pub step: Option<StepId>,
    /// Known bead id, absent until bd assigns one.
    pub id: Option<BeadId>,
    /// Node action.
    pub action: BeadNodeAction,
}

/// One planned or applied graph edge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadGraphEdge {
    /// Dependent bead id or unresolved step:<id>.
    pub from: String,
    /// Dependency bead id or unresolved step:<id>.
    pub to: String,
    /// Dependency type, including generated parent-child edges.
    #[serde(rename = "type")]
    pub kind: String,
    /// Edge action.
    pub action: BeadEdgeAction,
}

/// Outcome for a graph node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum BeadNodeAction {
    /// Will be created.
    Create,
    /// Was created.
    Created,
    /// Already exists and remains untouched.
    Existing,
}

/// Outcome for a graph edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum BeadEdgeAction {
    /// Will be added.
    Add,
    /// Was added.
    Added,
    /// Already exists and remains untouched.
    Existing,
}

/// An absent dependency between existing beads requiring manual repair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingEdge {
    /// Dependent bead id.
    pub from: BeadId,
    /// Dependency bead id.
    pub to: BeadId,
    /// Required dependency type.
    #[serde(rename = "type")]
    pub kind: String,
}

/// Identity metadata carried by every bead the graph engine creates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadGraphProvenance {
    /// Provenance format version; always 1.
    pub v: u32,
    /// Construction mode.
    pub mode: BeadGraphMode,
    /// Formula name.
    pub formula: String,
    /// Normalized rendered formula hash.
    pub revision: Sha256Digest,
    /// Hash of canonical mode and sorted relations.
    pub inputs: Sha256Digest,
    /// Attach parent, absent on pour.
    pub parent: Option<BeadId>,
    /// Attachment reference, absent on pour.
    #[serde(rename = "ref")]
    pub ref_: Option<GraphRef>,
    /// Formula step, absent on a pour root.
    pub step: Option<StepId>,
}
