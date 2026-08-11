//! Shared, deterministic SHA-256 identities for text files and resolved
//! template compositions.
//!
//! `sc-sha` deliberately does not read files, discover includes, canonicalize
//! paths, or apply resolver policy. Callers provide UTF-8 file bytes or a
//! fully resolved, ordered manifest. The crate exposes exactly two hashing
//! operations: [`calculate_hash`] and [`calculate_composition_hash`].

mod composition;
mod file;
mod manifest;

pub use composition::{CompositionError, CompositionSha256, calculate_composition_hash};
pub use file::{HashInput, HashResult, ShaError, TemplateSha256, calculate_hash};
pub use manifest::{
    CanonicalSource, CanonicalSourceError, CanonicalSourceUrl, CanonicalTemplatePath,
    ManifestSchemaVersion, ResolvedIncludeEdge, ResolvedTemplateManifest, ResolvedTemplateNode,
};
