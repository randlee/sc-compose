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


def test_malformed_manifest_and_invalid_utf8_have_stable_codes() -> None:
    with pytest.raises(sc_sha.ScShaError) as manifest_error:
        sc_sha.calculate_composition_hash({"schema": "v1", "nodes": [], "edges": [1]})
    assert manifest_error.value.code == "SC_SHA_INVALID_MANIFEST"

    with pytest.raises(sc_sha.ScShaError) as utf8_error:
        sc_sha.calculate_hash({"utf8_file_bytes": b"\xff"})
    assert utf8_error.value.code == "SC_SHA_INVALID_UTF8"
