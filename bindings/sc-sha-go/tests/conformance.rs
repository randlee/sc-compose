use std::path::PathBuf;

use sc_sha_go::{
    CanonicalSource, ResolvedIncludeEdge, ResolvedTemplateManifest, ResolvedTemplateNode,
    ScShaError, calculate_composition_hash, calculate_hash,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Vectors {
    hash_cases: Vec<HashCase>,
    composition_cases: Vec<CompositionCase>,
}

#[derive(Deserialize)]
struct HashCase {
    name: String,
    utf8_file_bytes_hex: String,
    sha256: Option<String>,
    error_code: Option<String>,
}

#[derive(Deserialize)]
struct CompositionCase {
    name: String,
    manifest: ManifestCase,
    sha256: Option<String>,
    error_code: Option<String>,
}

#[derive(Deserialize)]
struct ManifestCase {
    schema: String,
    nodes: Vec<NodeCase>,
    edges: Vec<EdgeCase>,
}

#[derive(Deserialize)]
struct NodeCase {
    source: SourceCase,
    sha256: String,
}

#[derive(Deserialize)]
struct EdgeCase {
    parent: SourceCase,
    child: SourceCase,
    occurrence: u32,
}

#[derive(Deserialize)]
struct SourceCase {
    kind: String,
    value: String,
}

fn vectors() -> Vectors {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/conformance-v1.json");
    let text = std::fs::read_to_string(path).expect("read shared conformance vectors");
    serde_json::from_str(&text).expect("parse shared conformance vectors")
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "hex has an even length");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex is ASCII"), 16)
                .expect("valid hex byte")
        })
        .collect()
}

fn source(source: SourceCase) -> CanonicalSource {
    match source.kind.as_str() {
        "local_path" => CanonicalSource::LocalPath {
            value: source.value,
        },
        "url" => CanonicalSource::Url {
            value: source.value,
        },
        other => panic!("unexpected test source kind: {other}"),
    }
}

fn manifest(manifest: ManifestCase) -> ResolvedTemplateManifest {
    ResolvedTemplateManifest {
        schema: manifest.schema,
        nodes: manifest
            .nodes
            .into_iter()
            .map(|node| ResolvedTemplateNode {
                source: source(node.source),
                sha256: node.sha256,
            })
            .collect(),
        edges: manifest
            .edges
            .into_iter()
            .map(|edge| ResolvedIncludeEdge {
                parent: source(edge.parent),
                child: source(edge.child),
                occurrence: edge.occurrence,
            })
            .collect(),
    }
}

fn error_code(error: &ScShaError) -> &str {
    match error {
        ScShaError::InvalidUtf8 { code, .. }
        | ScShaError::InvalidDigest { code, .. }
        | ScShaError::InvalidCanonicalSource { code, .. }
        | ScShaError::InvalidManifest { code, .. }
        | ScShaError::UnsupportedManifestSchema { code, .. }
        | ScShaError::DuplicateSource { code, .. }
        | ScShaError::UnknownEdgeEndpoint { code, .. } => code,
    }
}

#[test]
fn adapter_matches_shared_hash_vectors() {
    for case in vectors().hash_cases {
        match (case.sha256, case.error_code) {
            (Some(expected), None) => assert_eq!(
                calculate_hash(decode_hex(&case.utf8_file_bytes_hex))
                    .expect("valid vector")
                    .sha256,
                expected,
                "{}",
                case.name
            ),
            (None, Some(expected)) => assert_eq!(
                error_code(
                    &calculate_hash(decode_hex(&case.utf8_file_bytes_hex))
                        .expect_err("invalid vector")
                ),
                expected,
                "{}",
                case.name
            ),
            _ => panic!("{} must define exactly one expected result", case.name),
        }
    }
}

#[test]
fn adapter_matches_shared_composition_vectors() {
    for case in vectors().composition_cases {
        match (case.sha256, case.error_code) {
            (Some(expected), None) => assert_eq!(
                calculate_composition_hash(manifest(case.manifest))
                    .expect("valid vector")
                    .sha256,
                expected,
                "{}",
                case.name
            ),
            (None, Some(expected)) => assert_eq!(
                error_code(
                    &calculate_composition_hash(manifest(case.manifest))
                        .expect_err("invalid vector")
                ),
                expected,
                "{}",
                case.name
            ),
            _ => panic!("{} must define exactly one expected result", case.name),
        }
    }
}
