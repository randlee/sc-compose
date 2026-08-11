//! Compatibility vectors for synaptic-canvas-dolt's verified text-file
//! contract at commit `787f0507fa99be999c0a40d21b273cee05093a1f`.
//!
//! That source reads files with Python `Path.read_text(encoding="utf-8")`,
//! which strictly decodes UTF-8 and applies universal newline translation,
//! then hashes `content.encode("utf-8")` into the persisted
//! `package_files.sha256` field. These vectors intentionally cover text that
//! is common in Markdown, logs, and other Unicode-bearing template files.

use sc_sha::{
    CanonicalSource, CanonicalTemplatePath, HashInput, ManifestSchemaVersion, ResolvedIncludeEdge,
    ResolvedTemplateManifest, ResolvedTemplateNode, ShaError, calculate_composition_hash,
    calculate_hash,
};

fn hash_hex(bytes: &[u8]) -> String {
    calculate_hash(HashInput::TextFileBytes {
        utf8_file_bytes: bytes,
    })
    .expect("valid UTF-8 vector")
    .template()
    .to_hex()
}

#[test]
fn matches_synaptic_canvas_dolt_normalized_text_vectors() {
    let vectors = [
        (
            "empty",
            b"".as_slice(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "hello-lf",
            b"hello\n".as_slice(),
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03",
        ),
        (
            "hello-crlf",
            b"hello\r\n".as_slice(),
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03",
        ),
        (
            "hello-bare-cr",
            b"hello\r".as_slice(),
            "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03",
        ),
        (
            "bom-preserved-as-unicode",
            b"\xEF\xBB\xBFhello".as_slice(),
            "7489ebbcc2a00056ddaaaac190bce473e5c03696ea1bd8ed83cf59a174283862",
        ),
        (
            "no-final-newline",
            b"hello".as_slice(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        ),
        (
            "unicode-markdown-text",
            "café — 你好 🌍\n".as_bytes(),
            "a7cebdd8e9f02d462c15803864648c5b880f7a66b00c5c88c65c51bfd497befb",
        ),
        (
            "non-bmp-text",
            "🙂\r\n".as_bytes(),
            "8bfc725da0be07262af827dce94b8209cc262d634b21c65dc19dcd92943254f0",
        ),
        (
            "combining-mark-text",
            "e\u{301}\n".as_bytes(),
            "f979a211b00b61497349a7c753652a3d173550a368711a9f9f9845e6383db7cb",
        ),
    ];

    for (name, bytes, expected) in vectors {
        assert_eq!(hash_hex(bytes), expected, "vector {name}");
    }
}

#[test]
fn invalid_utf8_has_no_digest() {
    let error = calculate_hash(HashInput::TextFileBytes {
        utf8_file_bytes: &[0xf0, 0x28, 0x8c, 0x28],
    })
    .expect_err("invalid UTF-8 must fail");
    assert_eq!(error, ShaError::InvalidUtf8);
    assert_eq!(error.code(), "SC_SHA_INVALID_UTF8");
}

#[test]
fn composition_vector_is_stable_and_tagged() {
    let root = CanonicalSource::LocalPath(
        CanonicalTemplatePath::try_from("root.md".to_owned()).expect("root path"),
    );
    let child = CanonicalSource::LocalPath(
        CanonicalTemplatePath::try_from("child.md".to_owned()).expect("child path"),
    );
    let root_hash = calculate_hash(HashInput::TextFileBytes {
        utf8_file_bytes: b"root\n",
    })
    .expect("root hash")
    .template()
    .to_owned();
    let child_hash = calculate_hash(HashInput::TextFileBytes {
        utf8_file_bytes: b"child\n",
    })
    .expect("child hash")
    .template()
    .to_owned();
    let manifest = ResolvedTemplateManifest {
        schema: ManifestSchemaVersion::V1,
        nodes: vec![
            ResolvedTemplateNode {
                source: root.clone(),
                content_hash: root_hash,
            },
            ResolvedTemplateNode {
                source: child.clone(),
                content_hash: child_hash,
            },
        ],
        edges: vec![ResolvedIncludeEdge {
            parent: root,
            child,
            occurrence: 0,
        }],
    };

    let composition = calculate_composition_hash(&manifest).expect("composition hash");
    assert_eq!(
        composition.to_hex(),
        "80c55ea43eaa4c0453fe189c5aa0bbc1f523b8c66cc23ab990ec0356acd737ac"
    );
    assert_eq!(composition.as_bytes().len(), 32);
}
