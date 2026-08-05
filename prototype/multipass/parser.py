"""Stacked YAML frontmatter parser.

Parses template files with one or more leading `---...---` YAML frontmatter
headers. Stacked headers must be contiguous at the start of the file; later
`---` lines in the body remain ordinary template content.
"""

from __future__ import annotations

import re
from typing import Any

import yaml

from .types import ParsedTemplate, PassHeader

_FRONTMATTER_RE = re.compile(r"^---\s*$", re.MULTILINE)
_DOTS_RE = re.compile(r"^\.\.\.\s*$", re.MULTILINE)


def parse_template(text: str) -> ParsedTemplate:
    """Parse a template string into stacked PassHeaders + body.

    Handles:
        - Multiple stacked ---...--- headers
        - No headers at all (passes=[], body=text)
        - Single header (backward compat, pass defaults to 1)
        - `...` as closing delimiter (YAML end-of-document marker)
        - Empty headers (--- / valid, just no fields)

    Args:
        text: Raw template file content.

    Returns:
        ParsedTemplate with extracted headers and body text.
    """
    text = text.replace("\r\n", "\n")

    # Find all leading stacked headers: (header_end_offset, raw_yaml_content)
    headers: list[tuple[int, str]] = []
    cursor = 0

    while text.startswith("---", cursor):
        match = _FRONTMATTER_RE.match(text, cursor)
        if match is None:
            break

        content_start = match.end()

        # Find closing delimiter: --- or ... on its own line
        closing = _FRONTMATTER_RE.search(text, content_start)
        dots_match = _DOTS_RE.search(text, content_start)

        # Pick the closer delimiter (--- or ...)
        if closing is not None and (dots_match is None or closing.start() < dots_match.start()):
            body_end = closing.start()
            end = closing.end()
        elif dots_match is not None:
            body_end = dots_match.start()
            end = dots_match.end()
        else:
            raise ValueError("frontmatter started with `---` but no closing delimiter was found")

        raw_yaml = text[content_start:body_end].strip()
        headers.append((end, raw_yaml))
        cursor = end

        if cursor < len(text) and text[cursor] == "\n":
            cursor += 1

    # Body is everything after the last stacked header.
    if headers:
        body = text[cursor:]
    else:
        body = text

    # Parse each header into a PassHeader
    passes: list[PassHeader] = []
    seen_explicit_pass_numbers: set[int] = set()
    for _, raw_yaml in headers:
        header, has_explicit_pass = _parse_header(raw_yaml)
        if has_explicit_pass and header.pass_number in seen_explicit_pass_numbers:
            raise ValueError(f"duplicate pass number in stacked headers: {header.pass_number}")
        if has_explicit_pass:
            seen_explicit_pass_numbers.add(header.pass_number)
        passes.append(header)

    return ParsedTemplate(passes=passes, body=body)


def _parse_header(raw_yaml: str) -> tuple[PassHeader, bool]:
    """Parse a single YAML frontmatter block into a PassHeader.

    If the YAML is empty or missing a `pass` field, defaults to pass 1.
    """
    if not raw_yaml.strip():
        return PassHeader(pass_number=1), False

    try:
        data = yaml.safe_load(raw_yaml)
    except yaml.YAMLError as error:
        raise ValueError(f"invalid YAML frontmatter: {error}") from error

    if not isinstance(data, dict):
        raise ValueError("frontmatter must parse to a YAML mapping")

    has_explicit_pass = "pass" in data
    pass_number = data.get("pass", 1)
    if not isinstance(pass_number, int) or pass_number < 1:
        pass_number = 1

    required_vars: list[str] = []
    raw_required = data.get("required_variables", [])
    if isinstance(raw_required, list):
        required_vars = [str(v) for v in raw_required if v is not None]

    defaults: dict[str, Any] = {}
    raw_defaults = data.get("defaults", {})
    if isinstance(raw_defaults, dict):
        defaults = {str(k): v for k, v in raw_defaults.items()}

    # input_defaults overrides defaults (sc-compose existing behavior)
    raw_input_defaults = data.get("input_defaults", {})
    if isinstance(raw_input_defaults, dict):
        for k, v in raw_input_defaults.items():
            defaults[str(k)] = v

    metadata: dict[str, Any] = {}
    raw_metadata = data.get("metadata", {})
    if isinstance(raw_metadata, dict):
        metadata = raw_metadata

    return (
        PassHeader(
            pass_number=pass_number,
            required_variables=required_vars,
            defaults=defaults,
            metadata=metadata,
        ),
        has_explicit_pass,
    )
