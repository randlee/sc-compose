//! Injective hashing of a resolved template manifest.

use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use sha2::{Digest, Sha256};

use crate::manifest::{CanonicalSource, ManifestSchemaVersion, ResolvedTemplateManifest};

const DOMAIN: &[u8] = b"sc-sha/manifest/v1";

/// Errors returned when a caller-supplied manifest is not structurally valid.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CompositionError {
    /// The manifest schema is not supported by this crate.
    UnsupportedManifestSchema,
    /// A source occurs more than once in the node list.
    DuplicateSource,
    /// An edge refers to a source absent from the node list.
    UnknownEdgeEndpoint,
    /// A tagged source fails its canonical representation invariant.
    InvalidTaggedSource,
}

impl CompositionError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedManifestSchema => "SC_SHA_UNSUPPORTED_MANIFEST_SCHEMA",
            Self::DuplicateSource => "SC_SHA_DUPLICATE_SOURCE",
            Self::UnknownEdgeEndpoint => "SC_SHA_UNKNOWN_EDGE_ENDPOINT",
            Self::InvalidTaggedSource => "SC_SHA_INVALID_TAGGED_SOURCE",
        }
    }
}

impl Display for CompositionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnsupportedManifestSchema => "unsupported resolved manifest schema",
            Self::DuplicateSource => "resolved manifest contains a duplicate source",
            Self::UnknownEdgeEndpoint => "resolved manifest edge has an unknown endpoint",
            Self::InvalidTaggedSource => "resolved manifest contains an invalid tagged source",
        })
    }
}

impl std::error::Error for CompositionError {}

/// A SHA-256 digest over the injectively framed resolved manifest.
#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompositionSha256([u8; 32]);

impl CompositionSha256 {
    /// Borrow the raw 32-byte digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Render the digest as lowercase hexadecimal.
    #[must_use]
    pub fn to_hex(self) -> String {
        self.to_string()
    }
}

impl Display for CompositionSha256 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Hash a caller-supplied ordered, de-duplicated resolved manifest.
///
/// This function does not discover, reorder, deduplicate, or cycle-check the
/// graph. Those operations belong to the resolver that creates the manifest.
///
/// # Errors
///
/// Returns a typed [`CompositionError`] when the manifest schema, node set, or
/// edge endpoints violate the framing contract.
pub fn calculate_composition_hash(
    manifest: &ResolvedTemplateManifest,
) -> Result<CompositionSha256, CompositionError> {
    if !matches!(manifest.schema, ManifestSchemaVersion::V1) {
        return Err(CompositionError::UnsupportedManifestSchema);
    }

    let mut known_sources = HashSet::with_capacity(manifest.nodes.len());
    for node in &manifest.nodes {
        node.source
            .validate()
            .map_err(|_error| CompositionError::InvalidTaggedSource)?;
        if !known_sources.insert(&node.source) {
            return Err(CompositionError::DuplicateSource);
        }
    }
    for edge in &manifest.edges {
        edge.parent
            .validate()
            .map_err(|_error| CompositionError::InvalidTaggedSource)?;
        edge.child
            .validate()
            .map_err(|_error| CompositionError::InvalidTaggedSource)?;
        if !known_sources.contains(&edge.parent) || !known_sources.contains(&edge.child) {
            return Err(CompositionError::UnknownEdgeEndpoint);
        }
    }

    let mut encoded = Vec::new();
    write_bytes(&mut encoded, DOMAIN);
    encoded.push(1);
    write_count(&mut encoded, manifest.nodes.len());
    for node in &manifest.nodes {
        write_source(&mut encoded, &node.source);
        write_bytes(&mut encoded, node.content_hash.as_bytes());
    }
    write_count(&mut encoded, manifest.edges.len());
    for edge in &manifest.edges {
        write_source(&mut encoded, &edge.parent);
        write_source(&mut encoded, &edge.child);
        encoded.extend_from_slice(&edge.occurrence.to_be_bytes());
    }

    let digest = Sha256::digest(encoded);
    let mut bytes = [0; 32];
    bytes.copy_from_slice(&digest);
    Ok(CompositionSha256(bytes))
}

fn write_count(output: &mut Vec<u8>, count: usize) {
    output.extend_from_slice(&(count as u64).to_be_bytes());
}

fn write_bytes(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u64).to_be_bytes());
    output.extend_from_slice(value);
}

fn write_source(output: &mut Vec<u8>, source: &CanonicalSource) {
    match source {
        CanonicalSource::LocalPath(path) => {
            output.push(0);
            write_bytes(output, path.as_str().as_bytes());
        }
        CanonicalSource::Url(url) => {
            output.push(1);
            write_bytes(output, url.as_str().as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CompositionError, calculate_composition_hash};
    use crate::{
        CanonicalSource, CanonicalTemplatePath, HashInput, ManifestSchemaVersion,
        ResolvedIncludeEdge, ResolvedTemplateManifest, ResolvedTemplateNode, calculate_hash,
    };

    fn path(value: &str) -> CanonicalSource {
        CanonicalSource::LocalPath(
            CanonicalTemplatePath::try_from(value.to_owned()).expect("canonical path"),
        )
    }

    fn node(source: CanonicalSource, content: &[u8]) -> ResolvedTemplateNode {
        let content_hash = calculate_hash(HashInput::TextFileBytes {
            utf8_file_bytes: content,
        })
        .expect("content hash")
        .template()
        .to_owned();
        ResolvedTemplateNode {
            source,
            content_hash,
        }
    }

    #[test]
    fn rejects_duplicate_nodes_and_unknown_edges() {
        let source = path("root.md");
        let duplicate = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: vec![node(source.clone(), b"a"), node(source.clone(), b"b")],
            edges: Vec::new(),
        };
        assert_eq!(
            calculate_composition_hash(&duplicate),
            Err(CompositionError::DuplicateSource)
        );

        let unknown = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: vec![node(source.clone(), b"a")],
            edges: vec![ResolvedIncludeEdge {
                parent: source,
                child: path("missing.md"),
                occurrence: 0,
            }],
        };
        assert_eq!(
            calculate_composition_hash(&unknown),
            Err(CompositionError::UnknownEdgeEndpoint)
        );
    }

    #[test]
    fn node_and_edge_order_are_identity_inputs() {
        let root = path("root.md");
        let child = path("child.md");
        let first = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: vec![node(root.clone(), b"root"), node(child.clone(), b"child")],
            edges: vec![ResolvedIncludeEdge {
                parent: root.clone(),
                child: child.clone(),
                occurrence: 0,
            }],
        };
        let reordered = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: vec![node(child.clone(), b"child"), node(root.clone(), b"root")],
            edges: vec![ResolvedIncludeEdge {
                parent: child,
                child: root,
                occurrence: 0,
            }],
        };
        assert_ne!(
            calculate_composition_hash(&first).expect("first hash"),
            calculate_composition_hash(&reordered).expect("reordered hash")
        );
    }

    #[test]
    fn rejects_unsupported_schema_and_preserves_source_tags_and_occurrences() {
        let local = path("same");
        let unsupported = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::Unsupported(2),
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        assert_eq!(
            calculate_composition_hash(&unsupported),
            Err(CompositionError::UnsupportedManifestSchema)
        );

        let url = CanonicalSource::Url(
            crate::CanonicalSourceUrl::try_from("same".to_owned()).expect("canonical URL"),
        );
        let local_manifest = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: vec![node(local.clone(), b"same")],
            edges: Vec::new(),
        };
        let url_manifest = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: vec![node(url.clone(), b"same")],
            edges: Vec::new(),
        };
        assert_ne!(
            calculate_composition_hash(&local_manifest).expect("local hash"),
            calculate_composition_hash(&url_manifest).expect("URL hash")
        );

        let with_occurrence = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: vec![node(local.clone(), b"same")],
            edges: vec![ResolvedIncludeEdge {
                parent: local.clone(),
                child: local.clone(),
                occurrence: 0,
            }],
        };
        let with_second_occurrence = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: vec![node(local.clone(), b"same")],
            edges: vec![ResolvedIncludeEdge {
                parent: local.clone(),
                child: local,
                occurrence: 1,
            }],
        };
        assert_ne!(
            calculate_composition_hash(&with_occurrence).expect("first occurrence"),
            calculate_composition_hash(&with_second_occurrence).expect("second occurrence")
        );
    }
}
