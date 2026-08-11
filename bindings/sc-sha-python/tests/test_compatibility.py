import pytest

import sc_sha


def test_file_vector_and_result_shape() -> None:
    result = sc_sha.calculate_hash({"utf8_file_bytes": "hello\n".encode("utf-8")})
    assert result == {
        "kind": "template",
        "sha256": "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03",
    }


def test_unicode_and_newline_policy_is_explicit() -> None:
    lf = sc_sha.calculate_hash({"utf8_file_bytes": "世界\n".encode("utf-8")})
    crlf = sc_sha.calculate_hash({"utf8_file_bytes": "世界\r\n".encode("utf-8")})
    assert lf == crlf

    with pytest.raises(sc_sha.ScShaError) as error:
        sc_sha.calculate_hash({"utf8_file_bytes": "世界"})
    assert error.value.code == "SC_SHA_INVALID_INPUT"


def test_composition_preserves_nodes_and_edges_shape() -> None:
    child = sc_sha.calculate_hash({"utf8_file_bytes": b"child\n"})["sha256"]
    root = sc_sha.calculate_hash({"utf8_file_bytes": b"root\n"})["sha256"]
    manifest = {
        "schema": "sc-sha/manifest/v1",
        "nodes": [
            {"source": {"kind": "local_path", "value": "root.md"}, "sha256": root},
            {"source": {"kind": "local_path", "value": "child.md"}, "sha256": child},
        ],
        "edges": [
            {
                "parent": {"kind": "local_path", "value": "root.md"},
                "child": {"kind": "local_path", "value": "child.md"},
                "occurrence": 0,
            }
        ],
    }
    result = sc_sha.calculate_composition_hash(manifest)
    assert result["kind"] == "composition"
    assert len(result["sha256"]) == 64
    assert result["sha256"] == result["sha256"].lower()


def test_recursive_composition_preserves_caller_order_and_repeated_edges() -> None:
    def digest(value: bytes) -> str:
        return sc_sha.calculate_hash({"utf8_file_bytes": value})["sha256"]

    root = {"kind": "local_path", "value": "root.md"}
    child = {"kind": "local_path", "value": "nested/child.md"}
    leaf = {"kind": "local_path", "value": "nested/leaf.md"}
    manifest = {
        "schema": "sc-sha/manifest/v1",
        "nodes": [
            {"source": root, "sha256": digest(b"root\n")},
            {"source": child, "sha256": digest(b"child\n")},
            {"source": leaf, "sha256": digest(b"leaf\n")},
        ],
        "edges": [
            {"parent": root, "child": child, "occurrence": 0},
            {"parent": child, "child": leaf, "occurrence": 0},
            {"parent": root, "child": child, "occurrence": 1},
        ],
    }

    result = sc_sha.calculate_composition_hash(manifest)
    assert result["kind"] == "composition"
    assert result["sha256"] == "b0188088d70b61539a37ea70e06313cc852d5fedb685c3abdb657c6b3365857e"


def test_composition_hash_changes_when_one_nested_child_changes() -> None:
    source = {"kind": "local_path", "value": "root.md"}
    child = {"kind": "local_path", "value": "child.md"}

    def compose(child_bytes: bytes) -> str:
        child_sha = sc_sha.calculate_hash({"utf8_file_bytes": child_bytes})["sha256"]
        return sc_sha.calculate_composition_hash(
            {
                "schema": "sc-sha/manifest/v1",
                "nodes": [
                    {"source": source, "sha256": sc_sha.calculate_hash({"utf8_file_bytes": b"root"})["sha256"]},
                    {"source": child, "sha256": child_sha},
                ],
                "edges": [{"parent": source, "child": child, "occurrence": 0}],
            }
        )["sha256"]

    assert compose(b"v1") != compose(b"v2")


def test_local_and_url_sources_are_tagged_distinctly() -> None:
    digest = sc_sha.calculate_hash({"utf8_file_bytes": b"same"})["sha256"]

    def compose(source: dict[str, str]) -> str:
        return sc_sha.calculate_composition_hash(
            {
                "schema": "sc-sha/manifest/v1",
                "nodes": [{"source": source, "sha256": digest}],
                "edges": [],
            }
        )["sha256"]

    assert compose({"kind": "local_path", "value": "same.md"}) != compose(
        {"kind": "url", "value": "https://example.test/same.md"}
    )


def test_malformed_manifest_and_invalid_utf8_have_stable_codes() -> None:
    with pytest.raises(sc_sha.ScShaError) as manifest_error:
        sc_sha.calculate_composition_hash({"schema": "v1", "nodes": [], "edges": [1]})
    assert manifest_error.value.code == "SC_SHA_INVALID_MANIFEST"

    with pytest.raises(sc_sha.ScShaError) as utf8_error:
        sc_sha.calculate_hash({"utf8_file_bytes": b"\xff"})
    assert utf8_error.value.code == "SC_SHA_INVALID_UTF8"

    with pytest.raises(sc_sha.ScShaError) as schema_error:
        sc_sha.calculate_composition_hash(
            {"schema": "sc-sha/manifest/v2", "nodes": [], "edges": []}
        )
    assert schema_error.value.code == "SC_SHA_UNSUPPORTED_MANIFEST_SCHEMA"

    with pytest.raises(sc_sha.ScShaError) as str_error:
        sc_sha.calculate_hash({"utf8_file_bytes": "hello"})
    assert str_error.value.code == "SC_SHA_INVALID_INPUT"
