//! Structured diagnostics and stable `ERR_*` codes.

mod envelope;
mod filesystem;
mod record;
mod schema;

/// Stable schema version for diagnostics and machine-readable result payloads.
pub const DIAGNOSTIC_SCHEMA_VERSION: &str = "1";

pub use envelope::DiagnosticEnvelope;
pub(crate) use filesystem::{FilesystemErrorClass, classify_filesystem_error};
pub use record::Diagnostic;
pub use schema::{DiagnosticCode, DiagnosticSeverity};
