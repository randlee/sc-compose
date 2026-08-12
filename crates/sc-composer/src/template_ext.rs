//! Helpers for classifying template filename extensions.

/// Remove one recognized template suffix from a filename.
///
/// The suffix list and order preserve sc-compose's historical handling of
/// `.j2`, `.jinja2`, and `.jinja` template filenames.
pub fn strip_template_suffix(name: &str) -> &str {
    [".j2", ".jinja2", ".jinja"]
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::strip_template_suffix;

    #[test]
    fn strips_known_template_suffixes_without_changing_other_names() {
        for (name, expected) in [
            ("payload.json.j2", "payload.json"),
            ("payload.json.jinja2", "payload.json"),
            ("payload.json.jinja", "payload.json"),
            ("payload.json", "payload.json"),
        ] {
            assert_eq!(strip_template_suffix(name), expected, "name={name}");
        }
    }
}
