use serde::{Deserialize, Serialize};

use super::{DIAGNOSTIC_SCHEMA_VERSION, Diagnostic};

/// Versioned top-level diagnostics envelope used by JSON outputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEnvelope<T> {
    /// Stable schema version string.
    pub schema_version: String,
    /// Envelope payload supplied by the caller.
    pub payload: T,
    /// Diagnostics emitted alongside the payload.
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> DiagnosticEnvelope<T> {
    /// Create a versioned diagnostics envelope for a payload.
    #[must_use]
    pub fn new(payload: T, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            schema_version: DIAGNOSTIC_SCHEMA_VERSION.to_owned(),
            payload,
            diagnostics,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{DIAGNOSTIC_SCHEMA_VERSION, DiagnosticEnvelope};
    use crate::{Diagnostic, DiagnosticCode, DiagnosticSeverity};

    #[test]
    fn serialized_shape_and_schema_version_are_stable() {
        let envelope = DiagnosticEnvelope::new(
            json!({"ok": false}),
            vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrConfigParse,
                "bad config",
            )],
        );
        let value = serde_json::to_value(envelope).unwrap();
        assert_eq!(value["schema_version"], DIAGNOSTIC_SCHEMA_VERSION);
        assert_eq!(value["payload"], json!({"ok": false}));
        assert_eq!(value["diagnostics"][0]["severity"], "error");
        assert_eq!(value["diagnostics"][0]["code"], "ERR_CONFIG_PARSE");
        assert_eq!(value["diagnostics"][0]["path"], serde_json::Value::Null);
        assert_eq!(value["diagnostics"][0]["line"], serde_json::Value::Null);
        assert_eq!(value["diagnostics"][0]["column"], serde_json::Value::Null);
        assert_eq!(value["diagnostics"][0]["include_chain"], json!([]));
    }
}
