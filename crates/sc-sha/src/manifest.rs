//! Caller-supplied resolved manifest types.

use std::fmt::{Display, Formatter};

/// Error returned when a canonical source representation is malformed.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CanonicalSourceError {
    /// The value is empty, contains control data, or uses a non-canonical
    /// backslash separator.
    InvalidRepresentation,
}

impl Display for CanonicalSourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRepresentation => f.write_str("invalid canonical source representation"),
        }
    }
}

impl std::error::Error for CanonicalSourceError {}

/// An already-canonical, host-independent template path.
#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalTemplatePath(String);

impl TryFrom<String> for CanonicalTemplatePath {
    type Error = CanonicalSourceError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_representation(&value)?;
        Ok(Self(value))
    }
}

impl CanonicalTemplatePath {
    /// Read the canonical path representation without exposing its storage.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An already-canonical source URL.
#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalSourceUrl(String);

impl TryFrom<String> for CanonicalSourceUrl {
    type Error = CanonicalSourceError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_representation(&value)?;
        Ok(Self(value))
    }
}

impl CanonicalSourceUrl {
    /// Read the canonical URL representation without exposing its storage.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_representation(value: &str) -> Result<(), CanonicalSourceError> {
    if value.is_empty() || value.contains('\\') || value.chars().any(char::is_control) {
        return Err(CanonicalSourceError::InvalidRepresentation);
    }
    Ok(())
}

/// A tagged source key in a resolved manifest.
#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CanonicalSource {
    /// A canonical local template path.
    LocalPath(CanonicalTemplatePath),
    /// A canonical URL source.
    Url(CanonicalSourceUrl),
}

/// Version of the resolved manifest framing contract.
#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ManifestSchemaVersion {
    /// The first stable manifest contract.
    V1,
    /// A schema identifier not supported by this crate version.
    Unsupported(u16),
}

#[cfg(test)]
mod tests {
    use super::{CanonicalSourceError, CanonicalSourceUrl, CanonicalTemplatePath};

    #[test]
    fn constructors_reject_noncanonical_representations() {
        assert_eq!(
            CanonicalTemplatePath::try_from(String::new()),
            Err(CanonicalSourceError::InvalidRepresentation)
        );
        assert_eq!(
            CanonicalTemplatePath::try_from("nested\\template.md".to_owned()),
            Err(CanonicalSourceError::InvalidRepresentation)
        );
        assert_eq!(
            CanonicalSourceUrl::try_from("https://example.test/\n".to_owned()),
            Err(CanonicalSourceError::InvalidRepresentation)
        );
    }
}

/// One unique source node and its already-calculated file identity.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResolvedTemplateNode {
    /// Tagged canonical source identity.
    pub source: CanonicalSource,
    /// Per-file normalized text identity.
    pub content_hash: crate::TemplateSha256,
}

/// One ordered include occurrence in a resolved manifest.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResolvedIncludeEdge {
    /// The including source.
    pub parent: CanonicalSource,
    /// The included source.
    pub child: CanonicalSource,
    /// Stable occurrence ordinal supplied by the resolver.
    pub occurrence: u32,
}

/// A caller-supplied, ordered, de-duplicated resolved template manifest.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResolvedTemplateManifest {
    /// Manifest framing schema.
    pub schema: ManifestSchemaVersion,
    /// Unique nodes in resolver-defined canonical order.
    pub nodes: Vec<ResolvedTemplateNode>,
    /// Include occurrences in resolver-defined order.
    pub edges: Vec<ResolvedIncludeEdge>,
}
