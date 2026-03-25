use serde::Serialize;

use sc_composer::{Diagnostic, DiagnosticEnvelope};

/// Create the standard JSON diagnostics envelope for CLI payloads.
pub fn envelope<T: Serialize>(payload: T, diagnostics: Vec<Diagnostic>) -> serde_json::Value {
    serde_json::to_value(DiagnosticEnvelope::new(payload, diagnostics)).unwrap_or_else(|_| {
        serde_json::json!({
            "schema_version": sc_composer::DIAGNOSTIC_SCHEMA_VERSION,
            "payload": null,
            "diagnostics": []
        })
    })
}

#[cfg(test)]
mod tests {
    use sc_composer::{DIAGNOSTIC_SCHEMA_VERSION, Diagnostic, DiagnosticCode, DiagnosticSeverity};

    use super::envelope;

    #[test]
    fn envelope_success_shape_contains_schema_payload_and_diagnostics() {
        let value = envelope(serde_json::json!({ "ok": true }), Vec::new());

        assert_eq!(value["schema_version"], DIAGNOSTIC_SCHEMA_VERSION);
        assert_eq!(value["payload"]["ok"], true);
        assert!(value["diagnostics"].is_array());
    }

    #[test]
    fn envelope_error_shape_contains_diagnostics_array() {
        let diagnostics = vec![Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticCode::ErrValMissingRequired,
            "missing required variable: name",
        )];
        let value = envelope(serde_json::json!({ "valid": false }), diagnostics);

        assert_eq!(value["schema_version"], DIAGNOSTIC_SCHEMA_VERSION);
        assert_eq!(value["payload"]["valid"], false);
        assert_eq!(value["diagnostics"].as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn envelope_dry_run_shape_contains_payload_and_diagnostics() {
        let value = envelope(
            serde_json::json!({
                "would_write": ".prompts/example.md",
                "template": "template.md.j2"
            }),
            Vec::new(),
        );

        assert_eq!(value["schema_version"], DIAGNOSTIC_SCHEMA_VERSION);
        assert_eq!(value["payload"]["would_write"], ".prompts/example.md");
        assert!(value["diagnostics"].is_array());
    }
}
