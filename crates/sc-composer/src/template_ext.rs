//! Helpers for classifying template filename extensions.

use std::path::Path;

/// Remove one recognized template suffix from a filename.
///
/// The suffix list and order preserve sc-compose's historical handling of
/// `.j2`, `.jinja2`, and `.jinja` template filenames.
#[must_use]
pub fn strip_template_suffix(name: &str) -> &str {
    [".j2", ".jinja2", ".jinja"]
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name)
}

/// Remove every recognized template suffix from a filename.
///
/// Template suffixes are compared case-insensitively so path classification
/// remains consistent on case-sensitive and case-insensitive filesystems.
#[must_use]
pub fn strip_all_template_suffixes(mut name: &str) -> &str {
    while let Some(stripped) = strip_one_template_suffix_case_insensitive(name) {
        name = stripped;
    }
    name
}

/// Return the content extension after removing all template suffixes.
///
/// The returned extension preserves its source casing. Callers that recognize
/// a specific content format should compare it case-insensitively.
#[must_use]
pub fn template_content_extension(name: &str) -> Option<&str> {
    Path::new(strip_all_template_suffixes(name))
        .extension()
        .and_then(|extension| extension.to_str())
}

/// Return whether a template path is a JSON template.
///
/// JSON classification removes stacked template suffixes and compares both
/// template suffixes and the JSON extension case-insensitively.
#[must_use]
pub fn is_json_template_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(template_content_extension)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

fn strip_one_template_suffix_case_insensitive(name: &str) -> Option<&str> {
    [".j2", ".jinja2", ".jinja"].iter().find_map(|suffix| {
        let start = name.len().checked_sub(suffix.len())?;
        let candidate = name.get(start..)?;
        candidate
            .eq_ignore_ascii_case(suffix)
            .then_some(&name[..start])
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{is_json_template_path, strip_all_template_suffixes, strip_template_suffix};

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

    #[test]
    fn generalized_suffix_classification_handles_stacked_and_case_variant_names() {
        for (name, expected) in [
            ("payload.json.j2.j2", "payload.json"),
            ("payload.JSON.J2", "payload.JSON"),
            ("payload.json.JINJA2", "payload.json"),
            ("payload.j2.j2", "payload"),
        ] {
            assert_eq!(strip_all_template_suffixes(name), expected, "name={name}");
        }

        for path in ["payload.json.j2.j2", "payload.JSON.j2", "payload.json.J2"] {
            assert!(is_json_template_path(Path::new(path)), "path={path}");
        }
        assert!(!is_json_template_path(Path::new("payload.j2.j2")));
    }
}
