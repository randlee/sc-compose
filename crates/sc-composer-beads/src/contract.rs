//! Versioned public contract types for Beads composition.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use crate::error::{BeadComposeError, GraphIdField};

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
    /// Preview graph children under an existing parent.
    PreviewAttach,
    /// Atomically attach graph children under an existing parent.
    Attach,
}

impl BeadOperation {
    /// Return the operation's stable serde wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Render => "render",
            Self::Validate => "validate",
            Self::PreviewPour => "preview_pour",
            Self::Pour => "pour",
            Self::PreviewAttach => "preview_attach",
            Self::Attach => "attach",
        }
    }
}

/// Explicit authorization required for a persistent pour.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum PourAuthorization {
    /// Permit exactly one persistent Beads creation operation.
    CreatePersistentBeads,
}

impl TryFrom<&str> for PourAuthorization {
    type Error = BeadComposeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "CreatePersistentBeads" => Ok(Self::CreatePersistentBeads),
            _ => Err(BeadComposeError::PourAuthorizationInvalid),
        }
    }
}

impl<'de> Deserialize<'de> for PourAuthorization {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        value
            .as_str()
            .and_then(|value| Self::try_from(value).ok())
            .ok_or_else(|| D::Error::custom(BeadComposeError::PourAuthorizationInvalid))
    }
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
    pub formula_name: Option<FormulaName>,
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
    let parsed = crate::request::BeadVariables::deserialize(deserializer)?;
    match parsed.duplicate {
        Some(key) => Err(D::Error::custom(
            BeadComposeError::BeadVariableKeyDuplicate { key },
        )),
        None => Ok(parsed.values),
    }
}

/// Parse a JSON request into the stable Beads composition contract.
///
/// This boundary preserves duplicate runtime-variable keys as
/// [`BeadComposeError::BeadVariableKeyDuplicate`] instead of exposing a
/// serializer-specific error to adapters. Invalid authorization sentinels return
/// [`BeadComposeError::PourAuthorizationInvalid`]; unrecognized relation endpoint
/// prefixes return [`BeadComposeError::RelationEndpointInvalid`].
///
/// # Errors
///
/// Returns a stable [`BeadComposeError`] when the input is malformed or does
/// not deserialize into the v1 request contract.
pub fn parse_request(input: &str) -> Result<BeadComposeRequest, BeadComposeError> {
    crate::request::parse_request(input)
}

/// Parse a request, retaining attach identifier refusals as receipts.
///
/// # Errors
/// Request-shape and authorization errors retain [`parse_request`]'s errors.
pub fn parse_request_with_outcome(input: &str) -> Result<RequestParseOutcome, BeadComposeError> {
    crate::request::parse_request_with_outcome(input, None)
}

/// Parse a request whose operation is decided by the caller, not the file.
///
/// Every operation-dependent check (authorization precedence, reference
/// grammar, identifier error class) uses `operation`; the returned request
/// carries it. Duplicate-field and source-location handling are unchanged.
///
/// # Errors
/// Request-shape and authorization errors retain [`parse_request`]'s errors.
pub fn parse_request_for_operation(
    input: &str,
    operation: BeadOperation,
) -> Result<RequestParseOutcome, BeadComposeError> {
    crate::request::parse_request_with_outcome(input, Some(operation))
}

/// A valid request or an attach identifier validation refusal.
#[derive(Clone, Debug, PartialEq)]
pub enum RequestParseOutcome {
    /// Fully typed request ready for execution.
    Ready(BeadComposeRequest),
    /// Refused request with its canonical identifier diagnostic.
    Refused(RefusedBeadComposeReceipt),
}

/// Canonical owned diagnostic retained alongside a refused receipt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeadDiagnostic {
    /// Stable protocol error code.
    pub code: String,
    /// Native error message including the applicable rule.
    pub message: String,
    /// Typed wire details, including field, original value, and rule for ids.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    /// Actionable recovery guidance from the core error contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<String>,
}

/// Existing receipt fields plus an additive canonical refusal diagnostic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefusedBeadComposeReceipt {
    /// Ordinary receipt readable by existing receipt consumers.
    #[serde(flatten)]
    pub receipt: BeadComposeReceipt,
    /// Diagnostic for the invalid identifier that caused this refusal.
    pub error: BeadDiagnostic,
}

/// Parse relation JSON using the same stable endpoint errors as requests.
///
/// # Errors
///
/// Returns [`BeadComposeError::RelationEndpointInvalid`] for an unrecognized
/// endpoint prefix, or a request-deserialization error for other malformed data.
pub fn parse_relations(value: Value) -> Result<Vec<BeadRelation>, BeadComposeError> {
    crate::request::parse_relations(value)
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
    /// Edges to repair after a `GraphEdgeMissing` refusal, independent of stage output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_edges: Vec<MissingEdge>,
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
    /// Preview graph children under an existing parent.
    PreviewAttach,
    /// Atomically attach graph children under an existing parent.
    Attach,
}

impl BeadStage {
    /// Return the stage's stable serde wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Render => "render",
            Self::Validate => "validate",
            Self::ResolveActiveRegistry => "resolve_active_registry",
            Self::PreviewPour => "preview_pour",
            Self::Pour => "pour",
            Self::PreviewAttach => "preview_attach",
            Self::Attach => "attach",
        }
    }
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

impl BeadStageOutcome {
    /// Return the outcome variant's stable serde wire name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Skipped => "skipped",
            Self::Failed { .. } => "failed",
        }
    }
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
    /// Bounded standard-error excerpt or diagnostic text for a failed stage.
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

impl BeadOutcome {
    /// Return the outcome variant's stable serde wire name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Refused { .. } => "refused",
            Self::Failed { .. } => "failed",
        }
    }
}

/// Metadata key identifying graphs constructed by sc-compose (ADR-0023).
pub const PROVENANCE_KEY: &str = "sc_compose_graph";

// The macro keeps validation identical at the Rust and serde boundaries.
macro_rules! graph_string {
    ($name:ident, $field:path, $rule:literal, $valid:expr) => {
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
                        field: $field,
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

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
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
    FormulaName,
    GraphIdField::Formula,
    "A portable formula name: ASCII letters, digits, underscores, dots and hyphens; no leading dot/hyphen or consecutive dots.",
    |s: &str| s
        .as_bytes()
        .first()
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        && !s.contains("..")
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
);

impl FormulaName {
    /// Preserve a Phase R registry formula name that predates graph validation.
    pub(crate) fn legacy(value: String) -> Self {
        Self(value)
    }
}

graph_string!(
    BeadId,
    GraphIdField::Bead,
    "A non-empty bead id containing no whitespace.",
    |s: &str| !s.is_empty() && !s.chars().any(char::is_whitespace)
);
graph_string!(
    GraphRef,
    GraphIdField::Ref,
    "An attachment reference matching `[A-Za-z0-9_-]{1,32}`.",
    |s: &str| (1..=32).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
);
graph_string!(
    StepId,
    GraphIdField::Step,
    "A step id matching `[A-Za-z0-9_]{1,64}`; hyphens are forbidden.",
    |s: &str| (1..=64).contains(&s.len())
        && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
);
graph_string!(
    Sha256Digest,
    GraphIdField::Digest,
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
        Err(BeadComposeError::RelationEndpointInvalid { value })
    }
}

impl From<BeadEndpoint> for String {
    fn from(endpoint: BeadEndpoint) -> Self {
        match endpoint {
            BeadEndpoint::Bead(id) => encode_endpoint("bead:", id.as_str()),
            BeadEndpoint::Step(id) => encode_endpoint("step:", id.as_str()),
        }
    }
}

impl std::fmt::Display for BeadEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bead(id) => write_endpoint(f, "bead:", id.as_ref()),
            Self::Step(id) => write_endpoint(f, "step:", id.as_ref()),
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
    pub formula: FormulaName,
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

/// A graph receipt endpoint, encoded as a bead id, `step:<id>`, or `_root`.
///
/// The wire format reserves `step:` and `_root` for unresolved identities.
/// Construct `Bead` explicitly for an existing bead with either spelling.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(try_from = "String", into = "String")]
pub enum GraphEndpoint {
    /// A resolved bead id.
    Bead(BeadId),
    /// A step whose bead id has not yet been assigned.
    Step(StepId),
    /// The unassigned pour root.
    Root,
}

impl TryFrom<String> for GraphEndpoint {
    type Error = BeadComposeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "_root" {
            Ok(Self::Root)
        } else if let Some(step) = value.strip_prefix("step:") {
            StepId::new(step).map(Self::Step)
        } else {
            BeadId::new(value).map(Self::Bead)
        }
    }
}

impl GraphEndpoint {
    pub(crate) fn encoded(&self) -> String {
        match self {
            Self::Bead(id) => encode_endpoint("", id.as_str()),
            Self::Step(id) => encode_endpoint("step:", id.as_str()),
            Self::Root => "_root".to_owned(),
        }
    }
}

fn encode_endpoint(prefix: &str, id: &str) -> String {
    let mut encoded = String::new();
    write_endpoint(&mut encoded, prefix, id).expect("writing to a String cannot fail");
    encoded
}

fn write_endpoint(output: &mut impl std::fmt::Write, prefix: &str, id: &str) -> std::fmt::Result {
    output.write_str(prefix)?;
    output.write_str(id)
}

impl std::fmt::Display for GraphEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.encoded())
    }
}

impl From<GraphEndpoint> for String {
    fn from(value: GraphEndpoint) -> Self {
        value.to_string()
    }
}

graph_string!(
    DependencyName,
    GraphIdField::DependencyType,
    "A custom dependency token: an ASCII letter followed by ASCII letters, digits, underscores or hyphens.",
    |s: &str| s.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
);

/// A validated graph dependency kind, including hierarchy and custom bd types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(try_from = "String", into = "String")]
pub enum GraphDependencyType {
    /// A well-known user relation kind.
    Known(BeadDependencyType),
    /// A generated hierarchy edge, unavailable in request relations.
    ParentChild,
    /// A validated custom type observed in existing bd state.
    Other(DependencyName),
}

impl TryFrom<String> for GraphDependencyType {
    type Error = BeadComposeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "parent-child" {
            return Ok(Self::ParentChild);
        }
        if let Ok(kind) =
            serde_json::from_value::<BeadDependencyType>(serde_json::Value::String(value.clone()))
        {
            return Ok(Self::Known(kind));
        }
        DependencyName::new(value).map(Self::Other)
    }
}

impl From<BeadDependencyType> for GraphDependencyType {
    fn from(kind: BeadDependencyType) -> Self {
        Self::Known(kind)
    }
}

impl std::fmt::Display for GraphDependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Known(kind) => f.write_str(kind.as_str()),
            Self::ParentChild => f.write_str("parent-child"),
            Self::Other(name) => name.fmt(f),
        }
    }
}

impl From<GraphDependencyType> for String {
    fn from(value: GraphDependencyType) -> Self {
        value.to_string()
    }
}

impl BeadDependencyType {
    /// Return the stable bd wire name for this dependency type.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Blocks => "blocks",
            Self::ConditionalBlocks => "conditional-blocks",
            Self::WaitsFor => "waits-for",
            Self::Related => "related",
            Self::DiscoveredFrom => "discovered-from",
            Self::RepliesTo => "replies-to",
            Self::RelatesTo => "relates-to",
            Self::Duplicates => "duplicates",
            Self::Supersedes => "supersedes",
            Self::AuthoredBy => "authored-by",
            Self::AssignedTo => "assigned-to",
            Self::ApprovedBy => "approved-by",
            Self::Attests => "attests",
            Self::Tracks => "tracks",
            Self::Until => "until",
            Self::CausedBy => "caused-by",
            Self::Validates => "validates",
            Self::DelegatedFrom => "delegated-from",
        }
    }
}

/// One planned or applied graph edge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadGraphEdge {
    /// Dependent bead id or unresolved step:<id>.
    pub from: GraphEndpoint,
    /// Dependency bead id or unresolved step:<id>.
    pub to: GraphEndpoint,
    /// Dependency type, including generated parent-child edges.
    #[serde(rename = "type")]
    pub kind: GraphDependencyType,
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
    pub kind: GraphDependencyType,
}

/// Identity metadata carried by every bead the graph engine creates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadGraphProvenance {
    /// Provenance format version; always 1.
    pub v: u32,
    /// Construction mode.
    pub mode: BeadGraphMode,
    /// Formula name.
    pub formula: FormulaName,
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
