//! Stable Beads composition failures.

use std::fmt::Write as _;
use std::path::PathBuf;

use thiserror::Error;

use crate::contract::{BeadId, BeadStage, GraphDependencyType, MissingEdge};
use serde::{Deserialize, Serialize};

/// Closed wire vocabulary for graph identifier and scope fields (ADR-0023).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphIdField {
    /// A bead identifier.
    Bead,
    /// An attachment reference.
    Ref,
    /// A formula step identifier.
    Step,
    /// A content digest.
    Digest,
    /// A formula name.
    Formula,
    /// A dependency type token.
    DependencyType,
    /// The top-level attachment parent.
    Parent,
}

impl GraphIdField {
    /// Return the field's stable serde wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bead => "bead",
            Self::Ref => "ref",
            Self::Step => "step",
            Self::Digest => "digest",
            Self::Formula => "formula",
            Self::DependencyType => "dependency_type",
            Self::Parent => "parent",
        }
    }
}

impl std::fmt::Display for GraphIdField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub(crate) fn graph_id_rule(field: GraphIdField) -> &'static str {
    match field {
        GraphIdField::Bead => "bead ids are non-empty without whitespace",
        GraphIdField::Ref => "ref is [A-Za-z0-9_-]{1,32}",
        GraphIdField::Step => "step is [A-Za-z0-9_]{1,64}; hyphens are forbidden",
        GraphIdField::Digest => "digest is sha256: plus exactly 64 lowercase hex digits",
        GraphIdField::Formula => {
            "formula contains ASCII letters, digits, underscores, dots and hyphens; no leading dot/hyphen or consecutive dots"
        }
        GraphIdField::DependencyType => {
            "dependency_type starts with an ASCII letter followed by ASCII letters, digits, underscores or hyphens"
        }
        GraphIdField::Parent => "use a valid identifier for the named field",
    }
}

/// Stable errors returned before or during Beads composition.
///
/// Serializes as `code` and `message`, with additive `details` and `recovery`
/// fields for graph failures and request-file read failures. Other errors
/// preserve their original two-field shape.
#[derive(Debug, Error)]
pub enum BeadComposeError {
    /// The request file could not be read as UTF-8.
    #[error(
        "could not read Beads request `{path}`: {source}; check that the request path exists and is a readable UTF-8 file"
    )]
    RequestReadFailed {
        /// Request path supplied by the caller.
        path: PathBuf,
        /// Native I/O error, retaining its kind and operating-system context.
        source: std::io::Error,
    },
    /// The JSON request did not deserialize into the versioned contract.
    #[error("invalid Beads composition request: {message}")]
    RequestDeserializationFailed {
        /// Serializer diagnostic retained for callers and logs.
        message: String,
    },
    /// A relation endpoint omitted its recognized step or bead prefix.
    #[error("endpoint `{value}` must have a step: or bead: prefix (ADR-0023)")]
    RelationEndpointInvalid {
        /// Endpoint string with an unrecognized or missing prefix.
        value: String,
    },
    /// The request selected an unsupported protocol schema.
    #[error("unsupported Beads composition schema `{actual}`")]
    UnknownSchema {
        /// Unsupported schema value supplied by the caller.
        actual: String,
    },
    /// A formula path was not a regular file.
    #[error("formula path is not a file: {path}")]
    FormulaPathNotFile {
        /// Path that did not resolve to a regular file.
        path: PathBuf,
    },
    /// A formula path has an unsupported extension.
    #[error("unsupported formula extension: {path}")]
    FormulaExtensionUnsupported {
        /// Path whose formula extension is unsupported.
        path: PathBuf,
    },
    /// A template path could not be normalized safely.
    #[error("invalid template path: {path}")]
    TemplatePathInvalid {
        /// Template path that could not be normalized safely.
        path: PathBuf,
    },
    /// The rendered-formula output path could not be normalized.
    #[error("invalid rendered_formula output path `{path}`: {rule}")]
    OutputPathInvalid {
        /// Output path supplied by the request.
        path: PathBuf,
        /// Rule that the output path failed.
        rule: String,
    },
    /// A template path escaped the working directory.
    #[error("template path escapes working directory: {path}")]
    TemplateOutsideWorkingDirectory {
        /// Canonical template path outside the configured working directory.
        path: PathBuf,
    },
    /// A rendered output path escaped the working directory.
    #[error("output path escapes working directory: {path}")]
    OutputOutsideWorkingDirectory {
        /// Canonical ordinary output path outside the configured working directory.
        path: PathBuf,
    },
    /// The final output path is a symbolic link and cannot be written safely.
    #[error("rendered output path is a symbolic link: {path}")]
    OutputPathSymlink {
        /// Final output path that resolved to a symbolic link.
        path: PathBuf,
    },
    /// A direct-Rust request supplied a path that cannot be represented in a
    /// direct UTF-8 `bd` argument.
    #[error("Beads composition paths must be valid UTF-8: {path}")]
    PathNotUtf8 {
        /// Non-UTF-8 request path rejected before rendering or process launch.
        path: PathBuf,
    },
    /// A Beads variable key is malformed.
    #[error("invalid Beads variable key `{key}`")]
    BeadVariableKeyInvalid {
        /// Malformed runtime-variable key.
        key: String,
    },
    /// A Beads variable key was supplied more than once.
    #[error("duplicate Beads variable key `{key}`")]
    BeadVariableKeyDuplicate {
        /// Duplicate runtime-variable key.
        key: String,
    },
    /// A Beads variable value contains a byte that cannot be passed in argv.
    #[error("invalid Beads variable value for `{key}`: {value:?}")]
    BeadVariableValueInvalid {
        /// Runtime-variable key whose value was rejected.
        key: String,
        /// Rejected runtime-variable value, rendered with escapes for control bytes.
        value: String,
    },
    /// Preview or persistent pour omitted the formula name.
    #[error("formula name is required for pour operations")]
    FormulaNameRequired,
    /// Persistent pour omitted the required authorization sentinel.
    #[error("persistent Beads creation requires explicit authorization")]
    PourAuthorizationRequired,
    /// Persistent pour supplied an unsupported authorization value.
    #[error("persistent Beads creation authorization is invalid")]
    PourAuthorizationInvalid,
    /// The configured `bd` executable could not be started.
    #[error("Beads executable is unavailable: {executable}")]
    BdUnavailable {
        /// Configured executable that could not be started.
        executable: PathBuf,
    },
    /// A process argument could not be represented by the operating system.
    #[error("invalid argument for Beads executable `{executable}`: {message}")]
    ProcessArgumentInvalid {
        /// Configured executable receiving the invalid argument.
        executable: PathBuf,
        /// Operating-system diagnostic describing the invalid argument.
        message: String,
    },
    /// A `bd` stage exceeded the per-stream output capture policy.
    #[error("Beads {stage:?} output exceeded the {limit_bytes}-byte capture limit")]
    ProcessOutputLimitExceeded {
        /// Stage whose child process exceeded the capture bound.
        stage: BeadStage,
        /// Per-stream hard output capture limit in bytes.
        limit_bytes: usize,
    },
    /// Formula rendering failed before Beads validation.
    #[error("formula rendering failed: {message}")]
    RenderFailed {
        /// Rendering failure details retained for diagnostics.
        message: String,
    },
    /// `bd cook --dry-run` failed.
    #[error("Beads formula validation failed: {cause}")]
    CookFailed {
        /// Exit status returned by `bd cook`, if it started.
        exit_status: Option<i32>,
        /// Short diagnostic returned by `bd cook` or its output parser.
        cause: String,
    },
    /// `bd where --json` failed or returned unusable output.
    #[error("active Beads registry resolution failed")]
    ActiveRegistryResolutionFailed {
        /// Exit status returned by `bd where`, if it started.
        exit_status: Option<i32>,
    },
    /// The rendered formula did not belong to the active registry.
    #[error("formula is outside the active Beads registry: {path}")]
    FormulaOutsideActiveRegistry {
        /// Rendered formula path outside the active Beads registry.
        path: PathBuf,
    },
    /// Both TOML and JSON formulas exist for the requested name.
    #[error("active Beads registry has ambiguous formula `{formula_name}`")]
    FormulaRegistryAmbiguous {
        /// Formula name with both TOML and JSON entries in the active registry.
        formula_name: String,
    },
    /// `bd mol pour --dry-run` failed.
    #[error("Beads pour preview failed")]
    PreviewPourFailed {
        /// Exit status returned by preview `bd mol pour`, if it started.
        exit_status: Option<i32>,
    },
    /// Authorized persistent `bd mol pour` failed.
    #[error("persistent Beads pour failed")]
    PourFailed {
        /// Exit status returned by persistent `bd mol pour`, if it started.
        exit_status: Option<i32>,
    },
    /// `ParentNotFound` condition from ADR-0023.
    #[error("graph parent `{parent}` does not exist; create it or name an existing bead")]
    GraphParentNotFound {
        /// Missing parent.
        parent: BeadId,
    },
    /// `IdInvalid` condition from ADR-0023.
    #[error(
        "invalid graph {field} `{value}`; follow ADR-0023: {rule}",
        rule = graph_id_rule(*.field)
    )]
    GraphIdInvalid {
        /// Invalid identifier field.
        field: GraphIdField,
        /// Rejected value.
        value: String,
    },
    /// `ScopeMismatch` condition from ADR-0023.
    #[error(
        "graph scope {field} disagrees with `{value}`; make compose_variables agree with the top-level parent/ref"
    )]
    GraphScopeMismatch {
        /// Mismatched scope field.
        field: GraphIdField,
        /// Conflicting value.
        value: String,
    },
    /// `FormulaUnsupported` condition from ADR-0023.
    #[error(
        "unsupported graph formula: {reason}; express the construct in the template or use registry pour"
    )]
    GraphFormulaUnsupported {
        /// Unsupported construct.
        reason: GraphFormulaUnsupportedReason,
    },
    /// `RelationInvalid` condition from ADR-0023.
    #[error("invalid graph relation {index}: {reason}; correct the relation")]
    GraphRelationInvalid {
        /// Zero-based relation index.
        index: usize,
        /// Relation rejection reason.
        reason: GraphRelationInvalidReason,
    },
    /// `Conflict` condition from ADR-0023.
    #[error(
        "graph bead `{id}` conflicts: {reason}; inspect it or use a new ref (existing beads are never edited)"
    )]
    GraphConflict {
        /// Conflicting planned bead.
        id: BeadId,
        /// Ownership conflict reason.
        reason: GraphConflictReason,
    },
    /// `EdgeConflict` condition from ADR-0023.
    #[error(
        "graph edge {from} -> {to} has type `{existing}`, requested `{requested}`; inspect the edge or change the relation"
    )]
    GraphEdgeConflict {
        /// Dependent bead.
        from: BeadId,
        /// Dependency bead.
        to: BeadId,
        /// Existing edge type.
        existing: GraphDependencyType,
        /// Requested edge type.
        requested: GraphDependencyType,
    },
    /// `EdgeMissing` condition from ADR-0023.
    #[error("graph edges missing; repair then retry: {}", missing_edge_commands(.edges))]
    GraphEdgeMissing {
        /// Non-empty missing edges in plan order; repair each before retrying.
        edges: Vec<MissingEdge>,
    },
    /// `ReadFailed` condition from ADR-0023.
    #[error(
        "graph read failed ({status:?}): {cause}; {command:?}; fix bd and retry, nothing was written"
    )]
    GraphReadFailed {
        /// Attempted bd argv.
        command: Vec<String>,
        /// Exit status, or None when killed by a signal.
        status: Option<i32>,
        /// Short diagnostic for the process, parse, or response-shape failure.
        cause: String,
    },
    /// `ApplyFailed` condition from ADR-0023.
    #[error(
        "graph apply failed ({status:?}): {cause}; {command:?}; fix bd and retry, nothing was written"
    )]
    GraphApplyFailed {
        /// Attempted bd argv.
        command: Vec<String>,
        /// Exit status, or None when killed by a signal.
        status: Option<i32>,
        /// Short diagnostic for the process, parse, or response-shape failure.
        cause: String,
    },
}

pub(crate) fn short_cause(message: &str) -> String {
    let first_line = message
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(message);
    let cause = first_line.trim().chars().take(256).collect::<String>();
    if cause.is_empty() {
        "no diagnostic provided".to_owned()
    } else {
        cause
    }
}

impl BeadComposeError {
    /// Stable code for request-file read failures.
    pub const REQUEST_READ_FAILED_CODE: &'static str = "BEADS_REQUEST_READ_FAILED";

    /// Stable code for malformed requests, available without constructing an error.
    pub const REQUEST_DESERIALIZATION_FAILED_CODE: &'static str =
        "BEADS_REQUEST_DESERIALIZATION_FAILED";

    /// Return the stable protocol code for this condition.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RequestReadFailed { .. } => Self::REQUEST_READ_FAILED_CODE,
            Self::RequestDeserializationFailed { .. } => Self::REQUEST_DESERIALIZATION_FAILED_CODE,
            Self::RelationEndpointInvalid { .. } => "BEADS_RELATION_ENDPOINT_INVALID",
            Self::UnknownSchema { .. } => "BEADS_UNKNOWN_SCHEMA",
            Self::FormulaPathNotFile { .. } => "BEADS_FORMULA_NOT_FILE",
            Self::FormulaExtensionUnsupported { .. } => "BEADS_FORMULA_EXTENSION_UNSUPPORTED",
            Self::TemplatePathInvalid { .. } => "BEADS_TEMPLATE_PATH_INVALID",
            Self::OutputPathInvalid { .. } => "BEADS_OUTPUT_PATH_INVALID",
            Self::TemplateOutsideWorkingDirectory { .. } => "BEADS_TEMPLATE_OUTSIDE_WORKING_DIR",
            Self::OutputOutsideWorkingDirectory { .. } => "BEADS_OUTPUT_OUTSIDE_WORKING_DIR",
            Self::OutputPathSymlink { .. } => "BEADS_OUTPUT_PATH_SYMLINK",
            Self::PathNotUtf8 { .. } => "BEADS_PATH_NOT_UTF8",
            Self::BeadVariableKeyInvalid { .. } => "BEADS_VARIABLE_KEY_INVALID",
            Self::BeadVariableKeyDuplicate { .. } => "BEADS_VARIABLE_KEY_DUPLICATE",
            Self::BeadVariableValueInvalid { .. } => "BEADS_VARIABLE_VALUE_INVALID",
            Self::FormulaNameRequired => "BEADS_FORMULA_NAME_REQUIRED",
            Self::PourAuthorizationRequired => "BEADS_POUR_AUTH_REQUIRED",
            Self::PourAuthorizationInvalid => "BEADS_POUR_AUTH_INVALID",
            Self::BdUnavailable { .. } => "BEADS_BD_UNAVAILABLE",
            Self::ProcessArgumentInvalid { .. } => "BEADS_PROCESS_ARGUMENT_INVALID",
            Self::ProcessOutputLimitExceeded { .. } => "BEADS_PROCESS_OUTPUT_LIMIT",
            Self::RenderFailed { .. } => "BEADS_RENDER_FAILED",
            Self::CookFailed { .. } => "BEADS_COOK_FAILED",
            Self::ActiveRegistryResolutionFailed { .. } => "BEADS_WHERE_FAILED",
            Self::FormulaOutsideActiveRegistry { .. } => "BEADS_FORMULA_OUTSIDE_ACTIVE_REGISTRY",
            Self::FormulaRegistryAmbiguous { .. } => "BEADS_FORMULA_REGISTRY_AMBIGUOUS",
            Self::PreviewPourFailed { .. } => "BEADS_PREVIEW_POUR_FAILED",
            Self::PourFailed { .. } => "BEADS_POUR_FAILED",
            Self::GraphParentNotFound { .. } => "BEADS_GRAPH_PARENT_NOT_FOUND",
            Self::GraphIdInvalid { .. } => "BEADS_GRAPH_ID_INVALID",
            Self::GraphScopeMismatch { .. } => "BEADS_GRAPH_SCOPE_MISMATCH",
            Self::GraphFormulaUnsupported { .. } => "BEADS_GRAPH_FORMULA_UNSUPPORTED",
            Self::GraphRelationInvalid { .. } => "BEADS_GRAPH_RELATION_INVALID",
            Self::GraphConflict { .. } => "BEADS_GRAPH_CONFLICT",
            Self::GraphEdgeConflict { .. } => "BEADS_GRAPH_EDGE_CONFLICT",
            Self::GraphEdgeMissing { .. } => "BEADS_GRAPH_EDGE_MISSING",
            Self::GraphReadFailed { .. } => "BEADS_GRAPH_READ_FAILED",
            Self::GraphApplyFailed { .. } => "BEADS_GRAPH_APPLY_FAILED",
        }
    }
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
}

impl std::fmt::Display for GraphConflictReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotOwned => "not_owned",
            Self::ProvenanceDiffers => "provenance_differs",
        })
    }
}

/// Closed wire vocabulary for `GraphRelationInvalidReason` (ADR-0023).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum GraphRelationInvalidReason {
    /// The `unknown_step` refusal reason.
    UnknownStep,
    /// The `self_edge` refusal reason.
    SelfEdge,
    /// The `no_step` refusal reason.
    NoStep,
    /// The `duplicate` refusal reason.
    Duplicate,
    /// The `parent_pair` refusal reason.
    ParentPair,
    /// The `bead_not_found` refusal reason.
    BeadNotFound,
    /// The `registry_pour` refusal reason.
    RegistryPour,
}

impl std::fmt::Display for GraphRelationInvalidReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnknownStep => "unknown_step",
            Self::SelfEdge => "self_edge",
            Self::NoStep => "no_step",
            Self::Duplicate => "duplicate",
            Self::ParentPair => "parent_pair",
            Self::BeadNotFound => "bead_not_found",
            Self::RegistryPour => "registry_pour",
        })
    }
}

/// Closed wire vocabulary for `GraphFormulaUnsupportedReason` (ADR-0023).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum GraphFormulaUnsupportedReason {
    /// The `not_utf8` refusal reason.
    NotUtf8,
    /// The `vars_declared` refusal reason.
    VarsDeclared,
    /// The `bead_variables_set` refusal reason.
    BeadVariablesSet,
    /// The `composition` refusal reason.
    Composition,
    /// The `unknown_key` refusal reason.
    UnknownKey,
    /// The `step_construct` refusal reason.
    StepConstruct,
    /// The `reserved_metadata` refusal reason.
    ReservedMetadata,
    /// The `label_comma` refusal reason.
    LabelComma,
    /// The `step_graph` refusal reason.
    StepGraph,
}

impl std::fmt::Display for GraphFormulaUnsupportedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotUtf8 => "not_utf8",
            Self::VarsDeclared => "vars_declared",
            Self::BeadVariablesSet => "bead_variables_set",
            Self::Composition => "composition",
            Self::UnknownKey => "unknown_key",
            Self::StepConstruct => "step_construct",
            Self::ReservedMetadata => "reserved_metadata",
            Self::LabelComma => "label_comma",
            Self::StepGraph => "step_graph",
        })
    }
}

/// Quote one argument for a shell-copyable recovery command.
#[must_use]
pub fn shell_quote(argument: &str) -> String {
    if !argument.chars().any(needs_shell_escape) {
        return format!("'{}'", argument.replace('\'', "'\"'\"'"));
    }
    let mut quoted = String::from("$'");
    for character in argument.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '\'' => quoted.push_str("\\'"),
            character if needs_shell_escape(character) => {
                let code = u32::from(character);
                // String formatting is infallible.
                let _ = if character.is_ascii() {
                    write!(quoted, "\\x{code:02X}")
                } else if code <= 0xFFFF {
                    write!(quoted, "\\u{code:04X}")
                } else {
                    write!(quoted, "\\U{code:08X}")
                };
            }
            character => quoted.push(character),
        }
    }
    quoted.push('\'');
    quoted
}

fn needs_shell_escape(character: char) -> bool {
    character.is_control()
        || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}')
}

fn missing_edge_commands(edges: &[MissingEdge]) -> String {
    edges
        .iter()
        .map(|edge| {
            format!(
                "bd dep add {} {} --type {}",
                shell_quote(edge.from.as_str()),
                shell_quote(edge.to.as_str()),
                shell_quote(&edge.kind.to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
