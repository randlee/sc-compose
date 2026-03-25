use serde::Serialize;

use sc_composer::{Diagnostic, DiagnosticEnvelope};

pub fn envelope<T: Serialize>(payload: T, diagnostics: Vec<Diagnostic>) -> serde_json::Value {
    serde_json::to_value(DiagnosticEnvelope::new(payload, diagnostics)).unwrap_or_else(|_| {
        serde_json::json!({
            "schema_version": sc_composer::DIAGNOSTIC_SCHEMA_VERSION,
            "payload": null,
            "diagnostics": []
        })
    })
}
