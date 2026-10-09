"""Decode only the quoted literal fragments emitted by recovery-command quoting.

This is not a shell and never evaluates commands or substitutions. Hex escapes
produce bytes directly, including non-UTF-8 bytes in reference vectors. Production
Rust strings are UTF-8; NUL cannot be passed as a real subprocess argument.
"""
import json
import sys


def decode(text):
    arguments = []
    i = 0
    while i < len(text):
        if text[i] in " \t":
            i += 1
            continue
        word = bytearray()
        while i < len(text) and text[i] not in " \t":
            if text.startswith('"\'"', i):
                word.append(39)
                i += 3
                continue
            ansi = text.startswith("$'", i)
            if not ansi and text[i] != "'":
                raise ValueError("expected a quoted literal; unquoted syntax rejected")
            i += 2 if ansi else 1
            while i < len(text) and text[i] != "'":
                if ansi and text[i] == "\\":
                    i += 1
                    if i == len(text):
                        raise ValueError("unfinished escape")
                    escape = text[i]
                    i += 1
                    if escape in "\\'":
                        word.extend(escape.encode("utf-8"))
                    elif escape in ("x", "u", "U"):
                        width = {"x": 2, "u": 4, "U": 8}[escape]
                        digits = text[i:i + width]
                        if len(digits) != width or any(c not in "0123456789abcdefABCDEF" for c in digits):
                            raise ValueError("invalid fixed-width hex escape")
                        value = int(digits, 16)
                        if escape == "x":
                            word.append(value)
                        else:
                            if value > 0x10FFFF or 0xD800 <= value <= 0xDFFF:
                                raise ValueError("invalid Unicode scalar")
                            word.extend(chr(value).encode("utf-8"))
                        i += width
                    else:
                        raise ValueError("unsupported escape")
                else:
                    word.extend(text[i].encode("utf-8"))
                    i += 1
            if i == len(text):
                raise ValueError("unterminated quote")
            i += 1
        arguments.append(list(word))
    return arguments


try:
    text = json.loads(sys.stdin.buffer.read().decode("utf-8"))
    result = decode(text)
    # ASCII JSON byte arrays avoid console encoding and UTF-8 decoding of hex bytes.
    sys.stdout.buffer.write(json.dumps(result, ensure_ascii=True).encode("ascii"))
except (ValueError, TypeError) as error:
    sys.stderr.buffer.write(str(error).encode("utf-8"))
    sys.exit(1)
