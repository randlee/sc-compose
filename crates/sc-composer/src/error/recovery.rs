use std::path::PathBuf;

use crate::types::VariableName;

/// Structured recovery hint attached to configuration or validation failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryHint {
    /// Stable kind describing the hint payload.
    pub kind: RecoveryHintKind,
}

impl RecoveryHint {
    /// Create a structured recovery hint.
    #[must_use]
    pub const fn new(kind: RecoveryHintKind) -> Self {
        Self { kind }
    }
}

/// Structured recovery-hint payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryHintKind {
    /// Suggest a follow-up command.
    RunCommand {
        /// Command to execute.
        command: String,
    },
    /// Suggest reviewing a path.
    InspectPath {
        /// Path to inspect.
        path: PathBuf,
    },
    /// Suggest supplying a missing variable.
    ProvideVariable {
        /// Variable to provide.
        variable: VariableName,
    },
    /// Suggest correcting a configuration key.
    ReviewConfiguration {
        /// Configuration key to revisit.
        key: String,
    },
    /// Suggest inspecting an input payload or source document.
    InspectInput {
        /// Description of the input to inspect.
        description: String,
    },
    /// Suggest reviewing occurrence paths and selection rules.
    DisambiguateOccurrences {
        /// Description of the ambiguity to resolve.
        description: String,
    },
    /// Suggest replacing a construct outside the supported contract.
    UnsupportedConstruct {
        /// Description of the unsupported construct and its supported alternative.
        description: String,
    },
}
