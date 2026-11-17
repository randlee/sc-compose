"""Tests for preserving COFF content while removing forbidden metadata."""

import importlib.util
from pathlib import Path
import struct
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "clean_archive.py"
spec = importlib.util.spec_from_file_location("clean_archive", SCRIPT)
clean = importlib.util.module_from_spec(spec)
spec.loader.exec_module(clean)


def member(name: str, data: bytes) -> bytes:
    header = f"{name + '/':<16}{0:<12}{0:<6}{0:<6}{0:<8}{len(data):<10}`\n".encode()
    return header + data + (b"\n" if len(data) & 1 else b"")


def coff(sections: list[tuple[bytes, bytes]], symbols: int = 0) -> bytes:
    offset = 20 + 40 * len(sections)
    headers = bytearray()
    bodies = bytearray()
    for name, body in sections:
        header = bytearray(40)
        header[:8] = name.ljust(8, b"\0")
        struct.pack_into("<II", header, 16, len(body), offset + len(bodies))
        headers += header
        bodies += body
    return struct.pack("<HHIIIHH", 0xAA64, len(sections), 0, 0, symbols, 0, 0) + headers + bodies


class ArchiveTests(unittest.TestCase):
    def test_erases_only_excluded_directives_and_bitcode(self):
        directive = b" -defaultlib:kernel32 -exclude-symbols:secret -include:visible"
        obj = coff([(b".text", b"machine-code"), (b".drectve", directive), (b".llvmbc", b"LLVM-IR")])
        archive = b"!<arch>\n" + member("test.o", obj)
        result, empty = clean.clean_windows_archive(archive)
        self.assertEqual(empty, [])
        self.assertEqual(len(result), len(archive))
        self.assertIn(b"machine-code", result)
        self.assertIn(b"-defaultlib:kernel32", result)
        self.assertIn(b"-include:visible", result)
        self.assertNotIn(b"-exclude-symbols:secret", result)
        self.assertNotIn(b"LLVM-IR", result)
        self.assertNotIn(b".llvmbc", result)

    def test_empty_sectionless_objects_are_identified(self):
        result, empty = clean.clean_windows_archive(b"!<arch>\n" + member("empty.o", coff([])))
        self.assertEqual(empty, ["empty.o"])
        self.assertTrue(result.startswith(b"!<arch>\n"))
        with self.assertRaisesRegex(ValueError, "with symbols"):
            clean.clean_windows_archive(b"!<arch>\n" + member("bad.o", coff([], symbols=1)))

    def test_truncated_archive_is_refused(self):
        with self.assertRaises(ValueError):
            clean.clean_windows_archive(b"!<arch>\nbroken")


if __name__ == "__main__":
    unittest.main()
