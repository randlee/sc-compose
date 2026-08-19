//! Thin generated-binding adapter for the two public `sc-sha` operations.
//!
//! The UDL defines the foreign contract. This crate validates typed UDL values
//! and delegates hashing to `sc-sha`; it owns no hash implementation.

use sc_sha::{
    CanonicalSource as CoreCanonicalSource, CanonicalSourceError, CanonicalSourceUrl,
    CanonicalTemplatePath, CompositionError, HashInput, ManifestSchemaError, ManifestSchemaVersion,
    ResolvedIncludeEdge as CoreResolvedIncludeEdge,
    ResolvedTemplateManifest as CoreResolvedTemplateManifest,
    ResolvedTemplateNode as CoreResolvedTemplateNode, ShaError, TemplateSha256,
    calculate_composition_hash as calculate_core_composition_hash,
    calculate_hash as calculate_core_hash,
};
// The included UniFFI scaffold uses this runtime through generated code under
// `OUT_DIR`; retain an explicit source-level reference for dependency analysis.
use uniffi as _;

include!(concat!(env!("OUT_DIR"), "/sc_sha_go.uniffi.rs"));

/// Tagged canonical source identity accepted by the generated Go package.
#[derive(Debug)]
pub enum CanonicalSource {
    /// A canonical local template path.
    LocalPath { value: String },
    /// A canonical source URL.
    Url { value: String },
}

/// Per-template normalized text identity.
#[derive(Debug)]
pub struct TemplateHash {
    /// Lowercase SHA-256 hexadecimal digest.
    pub sha256: String,
}

/// Resolved composition identity.
#[derive(Debug)]
pub struct CompositionHash {
    /// Lowercase SHA-256 hexadecimal digest.
    pub sha256: String,
}

/// A resolved template source and its per-file digest.
#[derive(Debug)]
pub struct ResolvedTemplateNode {
    /// Tagged source identity.
    pub source: CanonicalSource,
    /// Lowercase SHA-256 hexadecimal digest.
    pub sha256: String,
}

/// An ordered include occurrence supplied by the caller's resolver.
#[derive(Debug)]
pub struct ResolvedIncludeEdge {
    /// Including source.
    pub parent: CanonicalSource,
    /// Included source.
    pub child: CanonicalSource,
    /// Resolver-defined occurrence ordinal.
    pub occurrence: u32,
}

/// A fully resolved, ordered, de-duplicated template manifest.
#[derive(Debug)]
pub struct ResolvedTemplateManifest {
    /// Supported value is `sc-sha/manifest/v1` (or the equivalent `v1`).
    pub schema: String,
    /// Resolver-defined node order.
    pub nodes: Vec<ResolvedTemplateNode>,
    /// Resolver-defined include occurrence order.
    pub edges: Vec<ResolvedIncludeEdge>,
}

/// Stable typed errors for the generated Go binding.
#[derive(Debug, thiserror::Error)]
pub enum ScShaError {
    /// Input bytes were not strict UTF-8.
    #[error("{message}")]
    InvalidUtf8 { code: String, message: String },
    /// A supplied digest is not canonical hexadecimal SHA-256.
    #[error("{message}")]
    InvalidDigest { code: String, message: String },
    /// A path or URL is not canonical.
    #[error("{message}")]
    InvalidCanonicalSource { code: String, message: String },
    /// A manifest field or source tag is invalid.
    #[error("{message}")]
    InvalidManifest { code: String, message: String },
    /// The manifest schema is unsupported.
    #[error("{message}")]
    UnsupportedManifestSchema { code: String, message: String },
    /// A source appears more than once in the node list.
    #[error("{message}")]
    DuplicateSource { code: String, message: String },
    /// An edge endpoint is not present in the node list.
    #[error("{message}")]
    UnknownEdgeEndpoint { code: String, message: String },
}

fn core_source(source: CanonicalSource) -> Result<CoreCanonicalSource, ScShaError> {
    match source {
        CanonicalSource::LocalPath { value } => CanonicalTemplatePath::try_from(value)
            .map(CoreCanonicalSource::LocalPath)
            .map_err(canonical_source_error),
        CanonicalSource::Url { value } => CanonicalSourceUrl::try_from(value)
            .map(CoreCanonicalSource::Url)
            .map_err(canonical_source_error),
    }
}

fn canonical_source_error(error: CanonicalSourceError) -> ScShaError {
    ScShaError::InvalidCanonicalSource {
        code: error.code().to_owned(),
        message: error.to_string(),
    }
}

fn parse_schema(schema: &str) -> Result<ManifestSchemaVersion, ScShaError> {
    ManifestSchemaVersion::try_from(schema).map_err(|error| match error {
        ManifestSchemaError::Empty => ScShaError::InvalidManifest {
            code: "SC_SHA_INVALID_MANIFEST".to_owned(),
            message: "manifest schema must not be empty".to_owned(),
        },
        ManifestSchemaError::Unsupported => ScShaError::UnsupportedManifestSchema {
            code: "SC_SHA_UNSUPPORTED_MANIFEST_SCHEMA".to_owned(),
            message: "unsupported manifest schema; use sc-sha/manifest/v1".to_owned(),
        },
    })
}

fn core_node(node: ResolvedTemplateNode) -> Result<CoreResolvedTemplateNode, ScShaError> {
    let content_hash = TemplateSha256::from_hex(&node.sha256).map_err(sha_error)?;
    Ok(CoreResolvedTemplateNode {
        source: core_source(node.source)?,
        content_hash,
    })
}

fn core_edge(edge: ResolvedIncludeEdge) -> Result<CoreResolvedIncludeEdge, ScShaError> {
    Ok(CoreResolvedIncludeEdge {
        parent: core_source(edge.parent)?,
        child: core_source(edge.child)?,
        occurrence: edge.occurrence,
    })
}

fn core_manifest(
    manifest: ResolvedTemplateManifest,
) -> Result<CoreResolvedTemplateManifest, ScShaError> {
    let schema = parse_schema(&manifest.schema)?;
    let nodes = manifest
        .nodes
        .into_iter()
        .map(core_node)
        .collect::<Result<Vec<_>, _>>()?;
    let edges = manifest
        .edges
        .into_iter()
        .map(core_edge)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CoreResolvedTemplateManifest {
        schema,
        nodes,
        edges,
    })
}

fn sha_error(error: ShaError) -> ScShaError {
    let (code, message) = (error.code().to_owned(), error.to_string());
    match error {
        ShaError::InvalidUtf8 => ScShaError::InvalidUtf8 { code, message },
        ShaError::InvalidDigestHex => ScShaError::InvalidDigest { code, message },
    }
}

fn composition_error(error: CompositionError) -> ScShaError {
    let (code, message) = (error.code().to_owned(), error.to_string());
    match error {
        CompositionError::UnsupportedManifestSchema => {
            ScShaError::UnsupportedManifestSchema { code, message }
        }
        CompositionError::DuplicateSource => ScShaError::DuplicateSource { code, message },
        CompositionError::UnknownEdgeEndpoint => ScShaError::UnknownEdgeEndpoint { code, message },
    }
}

/// Calculate a normalized text-file identity through `sc-sha`.
///
/// # Errors
///
/// Returns a typed [`ScShaError`] when the provided bytes are not UTF-8.
#[allow(
    clippy::needless_pass_by_value,
    reason = "the UniFFI UDL `bytes` parameter generates an owned Vec<u8> ABI contract"
)]
pub fn calculate_hash(utf8_file_bytes: Vec<u8>) -> Result<TemplateHash, ScShaError> {
    let result = calculate_core_hash(HashInput::TextFileBytes {
        utf8_file_bytes: &utf8_file_bytes,
    })
    .map_err(sha_error)?;
    Ok(TemplateHash {
        sha256: result.template().to_hex(),
    })
}

/// Calculate an ordered resolved-template composition identity through `sc-sha`.
///
/// # Errors
///
/// Returns a typed [`ScShaError`] when the manifest schema, source, digest, or
/// resolved graph violates the published `sc-sha` contract.
pub fn calculate_composition_hash(
    manifest: ResolvedTemplateManifest,
) -> Result<CompositionHash, ScShaError> {
    let manifest = core_manifest(manifest)?;
    let result = calculate_core_composition_hash(&manifest).map_err(composition_error)?;
    Ok(CompositionHash {
        sha256: result.to_hex(),
    })
}

#[cfg(test)]
mod tests {
    use super::{ScShaError, calculate_hash, parse_schema};

    #[test]
    fn calls_sc_sha_for_normalized_text() {
        let hash = calculate_hash(b"hello\r\n".to_vec()).expect("hash valid UTF-8");
        assert_eq!(
            hash.sha256,
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
        );
    }

    #[test]
    fn errors_are_typed_and_keep_stable_codes() {
        let error = calculate_hash(vec![0xff]).expect_err("invalid UTF-8 rejected");
        match error {
            ScShaError::InvalidUtf8 { code, message } => {
                assert_eq!(code, "SC_SHA_INVALID_UTF8");
                assert!(message.contains("UTF-8"));
            }
            _ => panic!("unexpected error variant"),
        }
        assert!(matches!(
            parse_schema("v2"),
            Err(ScShaError::UnsupportedManifestSchema { .. })
        ));
        assert!(matches!(
            parse_schema(""),
            Err(ScShaError::InvalidManifest { .. })
        ));
    }
}
