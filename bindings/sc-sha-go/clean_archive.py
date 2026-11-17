#!/usr/bin/env python3
"""Remove Rust archive metadata rejected by Go's cgo linker checks."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import struct
import subprocess
import tempfile


EXCLUDED = re.compile(rb'(?<!\S)[-/]exclude-symbols:[^\s\x00]+', re.IGNORECASE)


def clean_windows_archive(data: bytes) -> tuple[bytes, list[str]]:
    """Erase only COFF exclusion directives, preserving all other bytes and offsets."""
    if not data.startswith(b"!<arch>\n"):
        raise ValueError("expected a regular ar archive")
    result = bytearray(data)
    offset = 8
    long_names = b""
    names: list[str] = []
    empty: list[str] = []
    while offset < len(result):
        if offset + 60 > len(result) or result[offset + 58 : offset + 60] != b"`\n":
            raise ValueError("malformed ar member header")
        name = bytes(result[offset : offset + 16]).strip()
        try:
            size = int(result[offset + 48 : offset + 58])
        except ValueError as error:
            raise ValueError("malformed ar member size") from error
        start = offset + 60
        end = start + size
        if size < 0 or end > len(result):
            raise ValueError("ar member extends beyond archive")
        if name == b"//":
            long_names = bytes(result[start:end])
        elif name not in (b"/", b"/SYM64/"):
            if name.startswith(b"#1/"):
                name_len = int(name[3:])
                actual_name = bytes(result[start : start + name_len]).rstrip(b"\0").decode("utf-8")
                start += name_len
                if start > end:
                    raise ValueError("malformed extended ar name")
            elif name.startswith(b"/") and name[1:].isdigit():
                name_start = int(name[1:])
                name_end = long_names.find(b"/\n", name_start)
                if name_end < 0:
                    raise ValueError("malformed ar long name")
                actual_name = long_names[name_start:name_end].decode("utf-8")
            else:
                actual_name = name.rstrip(b"/").decode("utf-8")
            names.append(actual_name)
            if end - start < 20:
                raise ValueError("truncated archive object")
            machine, count = struct.unpack_from("<HH", result, start)
            if machine in (0xAA64, 0x8664):
                symbol_count = struct.unpack_from("<I", result, start + 12)[0]
                if count == 0:
                    if symbol_count:
                        raise ValueError("cannot remove a sectionless object with symbols")
                    empty.append(actual_name)
                    offset = end + (size & 1)
                    continue
                optional = struct.unpack_from("<H", result, start + 16)[0]
                sections = start + 20 + optional
                if sections + count * 40 > end:
                    raise ValueError("malformed COFF section table")
                for index in range(count):
                    section = sections + index * 40
                    section_name = bytes(result[section : section + 8]).rstrip(b"\0")
                    if section_name not in (b".drectve", b".llvmbc", b".llvmcmd"):
                        continue
                    length, pointer = struct.unpack_from("<II", result, section + 16)
                    position = start + pointer
                    if position + length > end:
                        raise ValueError("malformed COFF section data")
                    original = bytes(result[position : position + length])
                    if section_name == b".drectve":
                        result[position : position + length] = EXCLUDED.sub(lambda match: b" " * len(match.group()), original)
                    else:
                        # Destroy unused embedded IR without shifting archive offsets.
                        result[position : position + length] = bytes(length)
                        result[section : section + 8] = b".unused\0"
                        struct.pack_into("<II", result, section + 16, 0, 0)
        offset = end + (size & 1)
    if offset != len(result):
        raise ValueError("malformed ar padding")
    if any(names.count(name) != 1 for name in empty):
        raise ValueError("cannot safely remove duplicate archive member names")
    return bytes(result), empty


def verify_archive(path: Path) -> None:
    """Fail closed if any object still carries forbidden linker metadata."""
    result = subprocess.run(
        ["llvm-readobj", "--sections", "--coff-directives", str(path)],
        check=True, stdout=subprocess.PIPE, text=True,
    )
    for line in result.stdout.splitlines():
        if (re.match(r"\s*Name: (?:\.llvmbc|\.llvmcmd|\.llvm\.lto|\.gnu\.(?:debug)?lto_)", line)
                or re.match(r"\s*Segment: __LLVM(?:\s|$)", line)
                or ("Directive(s):" in line and EXCLUDED.search(line.encode()))):
            raise ValueError(f"unsupported metadata remains in archive: {line.strip()}")


def clean_archive(path: Path, target: str) -> None:
    if target in ("aarch64-pc-windows-gnullvm", "x86_64-pc-windows-gnu"):
        data, empty = clean_windows_archive(path.read_bytes())
        with tempfile.TemporaryDirectory(prefix="sc-sha-go-clean-") as directory:
            output = Path(directory) / "archive.a"
            output.write_bytes(data)
            if empty:
                subprocess.run(["llvm-ar", "d", str(output), *empty], check=True)
            path.write_bytes(output.read_bytes())
    elif "linux" in target or "apple-darwin" in target:
        sections = [".llvmbc", ".llvmcmd"] if "linux" in target else ["__LLVM,__bitcode", "__LLVM,__cmdline"]
        with tempfile.TemporaryDirectory(prefix="sc-sha-go-clean-") as directory:
            output = Path(directory) / "archive.a"
            subprocess.run(["llvm-objcopy", *(f"--remove-section={section}" for section in sections), str(path), str(output)], check=True)
            path.write_bytes(output.read_bytes())
    else:
        raise ValueError(f"unsupported native archive target: {target}")
    verify_archive(path)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("target")
    args = parser.parse_args()
    clean_archive(args.archive, args.target)


if __name__ == "__main__":
    main()
