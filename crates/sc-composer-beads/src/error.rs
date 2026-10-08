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
///
/// Adapters must not match every variant: the enum is `#[non_exhaustive]`,
/// so new conditions are additive. Use [`BeadComposeError::code`],
/// [`BeadComposeError::stage`] and [`BeadComposeError::class`] instead.
///
/// ```compile_fail,E0004
/// # use sc_composer_beads::BeadComposeError;
/// # fn exit(error: &BeadComposeError) -> u8 {
/// // Listing every variant without a wildcard arm does not compile.
/// # match error {
/// #     BeadComposeError::RequestReadFailed { .. } => 0,
/// #     BeadComposeError::RequestDeserializationFailed { .. } => 0,
/// #     BeadComposeError::RelationEndpointInvalid { .. } => 0,
/// #     BeadComposeError::UnknownSchema { .. } => 0,
/// #     BeadComposeError::FormulaPathNotFile { .. } => 0,
/// #     BeadComposeError::FormulaExtensionUnsupported { .. } => 0,
/// #     BeadComposeError::TemplatePathInvalid { .. } => 0,
/// #     BeadComposeError::OutputPathInvalid { .. } => 0,
/// #     BeadComposeError::TemplateOutsideWorkingDirectory { .. } => 0,
/// #     BeadComposeError::OutputOutsideWorkingDirectory { .. } => 0,
/// #     BeadComposeError::OutputPathSymlink { .. } => 0,
/// #     BeadComposeError::PathNotUtf8 { .. } => 0,
/// #     BeadComposeError::BeadVariableKeyInvalid { .. } => 0,
/// #     BeadComposeError::BeadVariableKeyDuplicate { .. } => 0,
/// #     BeadComposeError::BeadVariableValueInvalid { .. } => 0,
/// #     BeadComposeError::FormulaNameRequired => 0,
/// #     BeadComposeError::PourAuthorizationRequired => 0,
/// #     BeadComposeError::PourAuthorizationInvalid => 0,
/// #     BeadComposeError::BdUnavailable { .. } => 0,
/// #     BeadComposeError::ProcessArgumentInvalid { .. } => 0,
/// #     BeadComposeError::ProcessOutputLimitExceeded { .. } => 0,
/// #     BeadComposeError::RenderFailed { .. } => 0,
/// #     BeadComposeError::CookFailed { .. } => 0,
/// #     BeadComposeError::ActiveRegistryResolutionFailed { .. } => 0,
/// #     BeadComposeError::FormulaOutsideActiveRegistry { .. } => 0,
/// #     BeadComposeError::FormulaRegistryAmbiguous { .. } => 0,
/// #     BeadComposeError::PreviewPourFailed { .. } => 0,
/// #     BeadComposeError::PourFailed { .. } => 0,
/// #     BeadComposeError::GraphParentNotFound { .. } => 0,
/// #     BeadComposeError::GraphIdInvalid { .. } => 0,
/// #     BeadComposeError::GraphScopeMismatch { .. } => 0,
/// #     BeadComposeError::GraphFormulaUnsupported { .. } => 0,
/// #     BeadComposeError::GraphRelationInvalid { .. } => 0,
/// #     BeadComposeError::GraphConflict { .. } => 0,
/// #     BeadComposeError::GraphEdgeConflict { .. } => 0,
/// #     BeadComposeError::GraphEdgeMissing { .. } => 0,
/// #     BeadComposeError::GraphReadFailed { .. } => 0,
/// #     BeadComposeError::GraphApplyFailed { .. } => 0,
/// #     BeadComposeError::GraphApplyUnconfirmed { .. } => 0,
/// # }
/// # }
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
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
    /// `ApplyUnconfirmed` condition from ADR-0023: `bd create --graph` exited
    /// 0, so its transaction may have committed, but its response could not
    /// be consumed.
    #[error(
        "graph apply unconfirmed: {cause}; {command:?}; bd exited 0, so beads may have been created; {}",
        unconfirmed_apply_reconcile(.ids)
    )]
    GraphApplyUnconfirmed {
        /// Attempted bd argv.
        command: Vec<String>,
        /// Short diagnostic for the response-shape or id-mapping failure.
        cause: String,
        /// Known bead ids to inspect: the attach parent and planned step ids.
        /// Empty for a by-path pour, whose root id only bd assigns.
        ids: Vec<BeadId>,
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

    /// Stable code for [`BeadComposeError::RelationEndpointInvalid`].
    pub const RELATION_ENDPOINT_INVALID_CODE: &'static str = "BEADS_RELATION_ENDPOINT_INVALID";

    /// Stable code for [`BeadComposeError::UnknownSchema`].
    pub const UNKNOWN_SCHEMA_CODE: &'static str = "BEADS_UNKNOWN_SCHEMA";

    /// Stable code for [`BeadComposeError::FormulaPathNotFile`].
    pub const FORMULA_NOT_FILE_CODE: &'static str = "BEADS_FORMULA_NOT_FILE";

    /// Stable code for [`BeadComposeError::FormulaExtensionUnsupported`].
    pub const FORMULA_EXTENSION_UNSUPPORTED_CODE: &'static str =
        "BEADS_FORMULA_EXTENSION_UNSUPPORTED";

    /// Stable code for [`BeadComposeError::TemplatePathInvalid`].
    pub const TEMPLATE_PATH_INVALID_CODE: &'static str = "BEADS_TEMPLATE_PATH_INVALID";

    /// Stable code for [`BeadComposeError::OutputPathInvalid`].
    pub const OUTPUT_PATH_INVALID_CODE: &'static str = "BEADS_OUTPUT_PATH_INVALID";

    /// Stable code for [`BeadComposeError::TemplateOutsideWorkingDirectory`].
    pub const TEMPLATE_OUTSIDE_WORKING_DIR_CODE: &'static str =
        "BEADS_TEMPLATE_OUTSIDE_WORKING_DIR";

    /// Stable code for [`BeadComposeError::OutputOutsideWorkingDirectory`].
    pub const OUTPUT_OUTSIDE_WORKING_DIR_CODE: &'static str = "BEADS_OUTPUT_OUTSIDE_WORKING_DIR";

    /// Stable code for [`BeadComposeError::OutputPathSymlink`].
    pub const OUTPUT_PATH_SYMLINK_CODE: &'static str = "BEADS_OUTPUT_PATH_SYMLINK";

    /// Stable code for [`BeadComposeError::PathNotUtf8`].
    pub const PATH_NOT_UTF8_CODE: &'static str = "BEADS_PATH_NOT_UTF8";

    /// Stable code for [`BeadComposeError::BeadVariableKeyInvalid`].
    pub const VARIABLE_KEY_INVALID_CODE: &'static str = "BEADS_VARIABLE_KEY_INVALID";

    /// Stable code for [`BeadComposeError::BeadVariableKeyDuplicate`].
    pub const VARIABLE_KEY_DUPLICATE_CODE: &'static str = "BEADS_VARIABLE_KEY_DUPLICATE";

    /// Stable code for [`BeadComposeError::BeadVariableValueInvalid`].
    pub const VARIABLE_VALUE_INVALID_CODE: &'static str = "BEADS_VARIABLE_VALUE_INVALID";

    /// Stable code for [`BeadComposeError::FormulaNameRequired`].
    pub const FORMULA_NAME_REQUIRED_CODE: &'static str = "BEADS_FORMULA_NAME_REQUIRED";

    /// Stable code for [`BeadComposeError::PourAuthorizationRequired`].
    pub const POUR_AUTH_REQUIRED_CODE: &'static str = "BEADS_POUR_AUTH_REQUIRED";

    /// Stable code for [`BeadComposeError::PourAuthorizationInvalid`].
    pub const POUR_AUTH_INVALID_CODE: &'static str = "BEADS_POUR_AUTH_INVALID";

    /// Stable code for [`BeadComposeError::BdUnavailable`].
    pub const BD_UNAVAILABLE_CODE: &'static str = "BEADS_BD_UNAVAILABLE";

    /// Stable code for [`BeadComposeError::ProcessArgumentInvalid`].
    pub const PROCESS_ARGUMENT_INVALID_CODE: &'static str = "BEADS_PROCESS_ARGUMENT_INVALID";

    /// Stable code for [`BeadComposeError::ProcessOutputLimitExceeded`].
    pub const PROCESS_OUTPUT_LIMIT_CODE: &'static str = "BEADS_PROCESS_OUTPUT_LIMIT";

    /// Stable code for [`BeadComposeError::RenderFailed`].
    pub const RENDER_FAILED_CODE: &'static str = "BEADS_RENDER_FAILED";

    /// Stable code for [`BeadComposeError::CookFailed`].
    pub const COOK_FAILED_CODE: &'static str = "BEADS_COOK_FAILED";

    /// Stable code for [`BeadComposeError::ActiveRegistryResolutionFailed`].
    pub const WHERE_FAILED_CODE: &'static str = "BEADS_WHERE_FAILED";

    /// Stable code for [`BeadComposeError::FormulaOutsideActiveRegistry`].
    pub const FORMULA_OUTSIDE_ACTIVE_REGISTRY_CODE: &'static str =
        "BEADS_FORMULA_OUTSIDE_ACTIVE_REGISTRY";

    /// Stable code for [`BeadComposeError::FormulaRegistryAmbiguous`].
    pub const FORMULA_REGISTRY_AMBIGUOUS_CODE: &'static str = "BEADS_FORMULA_REGISTRY_AMBIGUOUS";

    /// Stable code for [`BeadComposeError::PreviewPourFailed`].
    pub const PREVIEW_POUR_FAILED_CODE: &'static str = "BEADS_PREVIEW_POUR_FAILED";

    /// Stable code for [`BeadComposeError::PourFailed`].
    pub const POUR_FAILED_CODE: &'static str = "BEADS_POUR_FAILED";

    /// Stable code for [`BeadComposeError::GraphParentNotFound`].
    pub const GRAPH_PARENT_NOT_FOUND_CODE: &'static str = "BEADS_GRAPH_PARENT_NOT_FOUND";

    /// Stable code for [`BeadComposeError::GraphIdInvalid`].
    pub const GRAPH_ID_INVALID_CODE: &'static str = "BEADS_GRAPH_ID_INVALID";

    /// Stable code for [`BeadComposeError::GraphScopeMismatch`].
    pub const GRAPH_SCOPE_MISMATCH_CODE: &'static str = "BEADS_GRAPH_SCOPE_MISMATCH";

    /// Stable code for [`BeadComposeError::GraphFormulaUnsupported`].
    pub const GRAPH_FORMULA_UNSUPPORTED_CODE: &'static str = "BEADS_GRAPH_FORMULA_UNSUPPORTED";

    /// Stable code for [`BeadComposeError::GraphRelationInvalid`].
    pub const GRAPH_RELATION_INVALID_CODE: &'static str = "BEADS_GRAPH_RELATION_INVALID";

    /// Stable code for [`BeadComposeError::GraphConflict`].
    pub const GRAPH_CONFLICT_CODE: &'static str = "BEADS_GRAPH_CONFLICT";

    /// Stable code for [`BeadComposeError::GraphEdgeConflict`].
    pub const GRAPH_EDGE_CONFLICT_CODE: &'static str = "BEADS_GRAPH_EDGE_CONFLICT";

    /// Stable code for [`BeadComposeError::GraphEdgeMissing`].
    pub const GRAPH_EDGE_MISSING_CODE: &'static str = "BEADS_GRAPH_EDGE_MISSING";

    /// Stable code for [`BeadComposeError::GraphReadFailed`].
    pub const GRAPH_READ_FAILED_CODE: &'static str = "BEADS_GRAPH_READ_FAILED";

    /// Stable code for [`BeadComposeError::GraphApplyFailed`].
    pub const GRAPH_APPLY_FAILED_CODE: &'static str = "BEADS_GRAPH_APPLY_FAILED";

    /// Stable code for [`BeadComposeError::GraphApplyUnconfirmed`].
    pub const GRAPH_APPLY_UNCONFIRMED_CODE: &'static str = "BEADS_GRAPH_APPLY_UNCONFIRMED";

    /// Stable code an adapter reports when it cannot deserialize a receipt
    /// returned by this library.
    pub const RECEIPT_DESERIALIZATION_FAILED_CODE: &'static str =
        "BEADS_RECEIPT_DESERIALIZATION_FAILED";

    /// Every `BEADS_GRAPH_*` code, in ADR-0023 table order.
    pub const GRAPH_CODES: &'static [&'static str] = &[
        Self::GRAPH_PARENT_NOT_FOUND_CODE,
        Self::GRAPH_ID_INVALID_CODE,
        Self::GRAPH_SCOPE_MISMATCH_CODE,
        Self::GRAPH_FORMULA_UNSUPPORTED_CODE,
        Self::GRAPH_RELATION_INVALID_CODE,
        Self::GRAPH_CONFLICT_CODE,
        Self::GRAPH_EDGE_CONFLICT_CODE,
        Self::GRAPH_EDGE_MISSING_CODE,
        Self::GRAPH_READ_FAILED_CODE,
        Self::GRAPH_APPLY_FAILED_CODE,
        Self::GRAPH_APPLY_UNCONFIRMED_CODE,
    ];

    /// Return the stage this error belongs to, or `None` for a request error
    /// found before any stage ran.
    #[must_use]
    pub const fn stage(&self) -> Option<BeadStage> {
        match self {
            Self::RenderFailed { .. } => Some(BeadStage::Render),
            Self::ProcessOutputLimitExceeded { stage, .. } => Some(*stage),
            Self::GraphIdInvalid { .. }
            | Self::CookFailed { .. }
            | Self::BdUnavailable { .. }
            | Self::ProcessArgumentInvalid { .. } => Some(BeadStage::Validate),
            Self::ActiveRegistryResolutionFailed { .. }
            | Self::FormulaOutsideActiveRegistry { .. }
            | Self::FormulaRegistryAmbiguous { .. } => Some(BeadStage::ResolveActiveRegistry),
            Self::PreviewPourFailed { .. } => Some(BeadStage::PreviewPour),
            Self::PourFailed { .. } => Some(BeadStage::Pour),
            // Graph-stage failures are reported in receipts, whose stage
            // receipts carry the stage; as a bare error they have none.
            Self::GraphParentNotFound { .. }
            | Self::GraphScopeMismatch { .. }
            | Self::GraphFormulaUnsupported { .. }
            | Self::GraphRelationInvalid { .. }
            | Self::GraphConflict { .. }
            | Self::GraphEdgeConflict { .. }
            | Self::GraphEdgeMissing { .. }
            | Self::GraphReadFailed { .. }
            | Self::GraphApplyFailed { .. }
            | Self::GraphApplyUnconfirmed { .. }
            | Self::RelationEndpointInvalid { .. }
            | Self::RequestReadFailed { .. }
            | Self::RequestDeserializationFailed { .. }
            | Self::UnknownSchema { .. }
            | Self::FormulaPathNotFile { .. }
            | Self::FormulaExtensionUnsupported { .. }
            | Self::TemplatePathInvalid { .. }
            | Self::OutputPathInvalid { .. }
            | Self::TemplateOutsideWorkingDirectory { .. }
            | Self::OutputOutsideWorkingDirectory { .. }
            | Self::OutputPathSymlink { .. }
            | Self::PathNotUtf8 { .. }
            | Self::BeadVariableKeyInvalid { .. }
            | Self::BeadVariableKeyDuplicate { .. }
            | Self::BeadVariableValueInvalid { .. }
            | Self::FormulaNameRequired
            | Self::PourAuthorizationRequired
            | Self::PourAuthorizationInvalid => None,
        }
    }

    /// Return whether this error rejects the request itself or reports a
    /// failure while executing it. The CLI maps these to exit 3 and 2.
    #[must_use]
    pub const fn class(&self) -> BeadErrorClass {
        match self {
            Self::RequestReadFailed { .. }
            | Self::RequestDeserializationFailed { .. }
            | Self::RelationEndpointInvalid { .. }
            | Self::UnknownSchema { .. }
            | Self::FormulaPathNotFile { .. }
            | Self::FormulaExtensionUnsupported { .. }
            | Self::TemplatePathInvalid { .. }
            | Self::OutputPathInvalid { .. }
            | Self::TemplateOutsideWorkingDirectory { .. }
            | Self::OutputOutsideWorkingDirectory { .. }
            | Self::OutputPathSymlink { .. }
            | Self::PathNotUtf8 { .. }
            | Self::BeadVariableKeyInvalid { .. }
            | Self::BeadVariableKeyDuplicate { .. }
            | Self::BeadVariableValueInvalid { .. }
            | Self::FormulaNameRequired
            | Self::PourAuthorizationRequired
            | Self::PourAuthorizationInvalid => BeadErrorClass::Request,
            Self::BdUnavailable { .. }
            | Self::ProcessArgumentInvalid { .. }
            | Self::ProcessOutputLimitExceeded { .. }
            | Self::RenderFailed { .. }
            | Self::CookFailed { .. }
            | Self::ActiveRegistryResolutionFailed { .. }
            | Self::FormulaOutsideActiveRegistry { .. }
            | Self::FormulaRegistryAmbiguous { .. }
            | Self::PreviewPourFailed { .. }
            | Self::PourFailed { .. }
            | Self::GraphParentNotFound { .. }
            | Self::GraphIdInvalid { .. }
            | Self::GraphScopeMismatch { .. }
            | Self::GraphFormulaUnsupported { .. }
            | Self::GraphRelationInvalid { .. }
            | Self::GraphConflict { .. }
            | Self::GraphEdgeConflict { .. }
            | Self::GraphEdgeMissing { .. }
            | Self::GraphReadFailed { .. }
            | Self::GraphApplyFailed { .. }
            | Self::GraphApplyUnconfirmed { .. } => BeadErrorClass::Execution,
        }
    }

    /// Return the stable protocol code for this condition.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RequestReadFailed { .. } => Self::REQUEST_READ_FAILED_CODE,
            Self::RequestDeserializationFailed { .. } => Self::REQUEST_DESERIALIZATION_FAILED_CODE,
            Self::RelationEndpointInvalid { .. } => Self::RELATION_ENDPOINT_INVALID_CODE,
            Self::UnknownSchema { .. } => Self::UNKNOWN_SCHEMA_CODE,
            Self::FormulaPathNotFile { .. } => Self::FORMULA_NOT_FILE_CODE,
            Self::FormulaExtensionUnsupported { .. } => Self::FORMULA_EXTENSION_UNSUPPORTED_CODE,
            Self::TemplatePathInvalid { .. } => Self::TEMPLATE_PATH_INVALID_CODE,
            Self::OutputPathInvalid { .. } => Self::OUTPUT_PATH_INVALID_CODE,
            Self::TemplateOutsideWorkingDirectory { .. } => Self::TEMPLATE_OUTSIDE_WORKING_DIR_CODE,
            Self::OutputOutsideWorkingDirectory { .. } => Self::OUTPUT_OUTSIDE_WORKING_DIR_CODE,
            Self::OutputPathSymlink { .. } => Self::OUTPUT_PATH_SYMLINK_CODE,
            Self::PathNotUtf8 { .. } => Self::PATH_NOT_UTF8_CODE,
            Self::BeadVariableKeyInvalid { .. } => Self::VARIABLE_KEY_INVALID_CODE,
            Self::BeadVariableKeyDuplicate { .. } => Self::VARIABLE_KEY_DUPLICATE_CODE,
            Self::BeadVariableValueInvalid { .. } => Self::VARIABLE_VALUE_INVALID_CODE,
            Self::FormulaNameRequired => Self::FORMULA_NAME_REQUIRED_CODE,
            Self::PourAuthorizationRequired => Self::POUR_AUTH_REQUIRED_CODE,
            Self::PourAuthorizationInvalid => Self::POUR_AUTH_INVALID_CODE,
            Self::BdUnavailable { .. } => Self::BD_UNAVAILABLE_CODE,
            Self::ProcessArgumentInvalid { .. } => Self::PROCESS_ARGUMENT_INVALID_CODE,
            Self::ProcessOutputLimitExceeded { .. } => Self::PROCESS_OUTPUT_LIMIT_CODE,
            Self::RenderFailed { .. } => Self::RENDER_FAILED_CODE,
            Self::CookFailed { .. } => Self::COOK_FAILED_CODE,
            Self::ActiveRegistryResolutionFailed { .. } => Self::WHERE_FAILED_CODE,
            Self::FormulaOutsideActiveRegistry { .. } => Self::FORMULA_OUTSIDE_ACTIVE_REGISTRY_CODE,
            Self::FormulaRegistryAmbiguous { .. } => Self::FORMULA_REGISTRY_AMBIGUOUS_CODE,
            Self::PreviewPourFailed { .. } => Self::PREVIEW_POUR_FAILED_CODE,
            Self::PourFailed { .. } => Self::POUR_FAILED_CODE,
            Self::GraphParentNotFound { .. } => Self::GRAPH_PARENT_NOT_FOUND_CODE,
            Self::GraphIdInvalid { .. } => Self::GRAPH_ID_INVALID_CODE,
            Self::GraphScopeMismatch { .. } => Self::GRAPH_SCOPE_MISMATCH_CODE,
            Self::GraphFormulaUnsupported { .. } => Self::GRAPH_FORMULA_UNSUPPORTED_CODE,
            Self::GraphRelationInvalid { .. } => Self::GRAPH_RELATION_INVALID_CODE,
            Self::GraphConflict { .. } => Self::GRAPH_CONFLICT_CODE,
            Self::GraphEdgeConflict { .. } => Self::GRAPH_EDGE_CONFLICT_CODE,
            Self::GraphEdgeMissing { .. } => Self::GRAPH_EDGE_MISSING_CODE,
            Self::GraphReadFailed { .. } => Self::GRAPH_READ_FAILED_CODE,
            Self::GraphApplyFailed { .. } => Self::GRAPH_APPLY_FAILED_CODE,
            Self::GraphApplyUnconfirmed { .. } => Self::GRAPH_APPLY_UNCONFIRMED_CODE,
        }
    }
}

/// Whether a [`BeadComposeError`] rejects the request or reports an execution
/// failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BeadErrorClass {
    /// The request is malformed or not permitted; nothing ran (CLI exit 3).
    Request,
    /// A stage failed while executing a valid request (CLI exit 2).
    Execution,
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
///
/// An argument without control or formatting characters is emitted as a POSIX
/// single-quoted word (`'` becomes `'"'"'`), which every POSIX shell and
/// `PowerShell` accept for these values. An argument that contains a control or
/// formatting character is emitted as an ANSI-C `$'...'` word with `\xNN`
/// byte escapes so no raw control character reaches the terminal; that form is
/// understood by Bash, Zsh, ksh93 and mksh but not by every POSIX `sh`, and is
/// not valid in `cmd.exe`. Windows callers should run recovery commands from
/// `PowerShell` or Git Bash.
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
                let mut encoded = [0; 4];
                for byte in character.encode_utf8(&mut encoded).bytes() {
                    // String formatting is infallible.
                    let _ = write!(quoted, "\\x{byte:02X}");
                }
            }
            character => quoted.push(character),
        }
    }
    quoted.push('\'');
    quoted
}

fn needs_shell_escape(character: char) -> bool {
    character.is_control()
        || matches!(character, '\u{2028}' | '\u{2029}')
        || is_format_character(character)
}

fn is_format_character(character: char) -> bool {
    matches!(u32::from(character),
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890..=0x0891 | 0x08E2
        | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F
        | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD | 0x13430..=0x1343F
        | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0001 | 0xE0020..=0xE007F)
}

/// Render terminal control and formatting characters as visible Unicode escapes.
#[must_use]
pub fn escape_human_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if needs_shell_escape(character) {
            let _ = write!(escaped, "\\u{{{:04X}}}", u32::from(character));
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn unconfirmed_apply_reconcile(ids: &[BeadId]) -> String {
    if ids.is_empty() {
        return String::from("find the molecule with bd list and reconcile before pouring again");
    }
    let ids = ids
        .iter()
        .map(|id| shell_quote(id.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    format!("reconcile with bd show {ids} before pouring again")
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

impl GraphIdField {
    /// Return the grammar rule an identifier of this field must satisfy.
    #[must_use]
    pub fn rule(self) -> &'static str {
        graph_id_rule(self)
    }
}
