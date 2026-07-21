"""Reverse template variable extractor.

Given a Jinja2 template and its rendered output (XML/JSON/MD),
extract the variable bindings that produced it.

Algorithm:
  1. Parse template frontmatter → required_variables + optional_variables
  2. Find all {{ var }} in template body, classify each by structural context
  3. Parse rendered XML with ElementTree
  4. Extract values using XPath derived from template structure
  5. Return structured JSON
"""

from __future__ import annotations

import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml


# ── Types ────────────────────────────────────────────────────────────────────


@dataclass
class VariableExtraction:
    """Result of extracting a single variable from rendered output."""

    name: str
    value: str
    source: str  # "attribute", "element_text", "block_text", "not_found", "skipped"
    xpath: str = ""


@dataclass
class VariableBinding:
    """Describes where a variable appears in the template."""

    name: str
    kind: str  # "attribute", "element_text", "block_text"
    tag: str  # enclosing XML tag
    attribute: str | None = None  # set when kind == "attribute"
    line: int = 0


# ── Regex patterns ───────────────────────────────────────────────────────────

# XML tag name: letters, digits, hyphens, underscores, colons
_TAG = r"[\w:-]+"

# Find all {{ var }} occurrences (captures variable name)
VAR_RE = re.compile(r"\{\{\s*(\w+(?:\.\w+)*)\s*\}\}")

# Line-based element text: <tag>{{ var }}</tag>
ELEM_TEXT_RE = re.compile(
    r'<(' + _TAG + r')>\{\{\s*(\w+(?:\.\w+)*)\s*\}\}</\1>'
)

# Standalone {{ var }} on its own line (block text)
BLOCK_RE = re.compile(r"^\s*\{\{\s*(\w+(?:\.\w+)*)\s*\}\}\s*$")


# ── Frontmatter parsing ──────────────────────────────────────────────────────


def parse_frontmatter(text: str) -> dict[str, Any]:
    """Parse YAML frontmatter from template text (between --- delimiters)."""
    parts = text.split("---", 2)
    if len(parts) < 3:
        return {}
    return yaml.safe_load(parts[1]) or {}


def parse_body(text: str) -> str:
    """Extract template body (everything after the second ---)."""
    parts = text.split("---", 2)
    if len(parts) < 3:
        return text
    return parts[2]


# ── Variable binding discovery ───────────────────────────────────────────────


# Match attr="{{ var }}" within a tag — captures attribute name + variable name
_ATTR_IN_TAG_RE = re.compile(r'(\w+)="\{\{\s*(\w+(?:\.\w+)*)\s*\}\}"')


def _find_enclosing_tag(lines: list[str], line_num: int) -> str:
    """Find the XML tag enclosing line line_num (1-indexed).

    Scans upward for <tag> or <tag ...> that isn't closed on the same line.
    """
    depth = 1
    for i in range(line_num - 2, -1, -1):
        line = lines[i]
        open_tags = re.findall(r"<([\w:-]+)[>\s]", line)
        close_tags = re.findall(r"</([\w:-]+)>", line)
        depth += len(close_tags) - len(open_tags)
        if depth <= 0:
            for tag in reversed(open_tags):
                return tag
    return "unknown"


def _find_parent_tag(lines: list[str], line_num: int) -> str:
    """Find the enclosing XML tag name for line line_num.

    Strategy: scan upward for the most recent opening tag.
    """
    for i in range(line_num - 1, -1, -1):
        line = lines[i]
        m = re.search(r"<([\w:-]+)[>\s]", line)
        if m:
            return m.group(1)
    return "unknown"


def discover_bindings(body: str) -> list[VariableBinding]:
    """Scan template body to find all {{ var }} positions and classify each.

    Classification priority:
      1. attribute — <tag attr="{{ var }}">
      2. element_text — <tag>{{ var }}</tag> on single line
      3. block_text — standalone {{ var }} between enclosing tags

    Uses VAR_RE to find ALL {{ var }} positions first, then classifies
    by looking at surrounding context — avoids greedy regex issues with
    multi-attribute elements.
    """
    lines = body.split("\n")
    bindings: list[VariableBinding] = []
    seen: set[str] = set()

    # Phase 1: Check each line for attribute bindings
    for i, line in enumerate(lines, 1):
        # Find opening tag on this line
        tag_m = re.search(r"<([\w:-]+)[>\s]", line)
        tag = tag_m.group(1) if tag_m else "unknown"

        for m in _ATTR_IN_TAG_RE.finditer(line):
            attr_name = m.group(1)
            var_name = m.group(2)
            if var_name not in seen:
                seen.add(var_name)
                bindings.append(
                    VariableBinding(
                        name=var_name,
                        kind="attribute",
                        tag=tag,
                        attribute=attr_name,
                        line=i,
                    )
                )

    # Phase 2: Element-text bindings — <tag>{{ var }}</tag>
    for i, line in enumerate(lines, 1):
        for m in ELEM_TEXT_RE.finditer(line):
            var_name = m.group(2)
            if var_name not in seen:
                seen.add(var_name)
                bindings.append(
                    VariableBinding(
                        name=var_name,
                        kind="element_text",
                        tag=m.group(1),
                        line=i,
                    )
                )

    # Phase 3: Block-text bindings — standalone {{ var }}
    for i, line in enumerate(lines, 1):
        m = BLOCK_RE.match(line)
        if m:
            var_name = m.group(1)
            if var_name not in seen:
                seen.add(var_name)
                tag = _find_parent_tag(lines, i)
                bindings.append(
                    VariableBinding(
                        name=var_name,
                        kind="block_text",
                        tag=tag,
                        line=i,
                    )
                )

    return bindings


# ── Value extraction from rendered XML ───────────────────────────────────────


def _extract_attribute(root: ET.Element, tag: str, attr: str) -> str | None:
    """Extract attribute value from the first matching element."""
    for el in root.iter(tag):
        return el.get(attr)
    return None


def _extract_element_text(root: ET.Element, tag: str) -> str | None:
    """Extract text content from the first matching element."""
    for el in root.iter(tag):
        text = (el.text or "").strip()
        if text:
            return text
    return None


def _extract_block_text(root: ET.Element, tag: str) -> str | None:
    """Extract full text content (including children) from a multi-line block."""
    for el in root.iter(tag):
        text = (el.text or "") + "".join(
            ET.tostring(child, encoding="unicode") for child in el
        )
        text = text.strip()
        if text:
            return text
    return None


EXTRACTION_STRATEGIES = {
    "attribute": _extract_attribute,
    "element_text": _extract_element_text,
    "block_text": _extract_block_text,
}


# ── Filter helpers ───────────────────────────────────────────────────────────


def _should_extract(
    name: str,
    include_vars: list[str] | None,
    exclude_vars: list[str] | None,
) -> bool:
    """Determine if a variable should be extracted based on filters."""
    if exclude_vars and name in exclude_vars:
        return False
    if include_vars and name not in include_vars:
        return False
    return True


# ── Main API ─────────────────────────────────────────────────────────────────


def _extract_xml_body(text: str) -> str:
    """Strip leading non-XML content (ATM headers, log lines) from rendered output.

    Finds the first '<' that starts an XML element and returns from there.
    """
    for m in re.finditer(r"<([\w:-]+)[>\s]", text):
        pos = m.start()
        if "<!--" not in text[max(0, pos - 10) : pos]:
            return text[pos:]
    return text


def compute_confidence(template_body: str, rendered_text: str) -> float:
    """Compute confidence that rendered_text was produced from this template.

    Splits template on {{ var }} placeholders to get static text segments,
    then measures what fraction of static characters appear in-order in
    the rendered output. Returns 0.0–1.0.

    A mismatch (wrong template) typically scores ~0.0 because the static
    text won't appear in the rendered output at all.
    """
    # Split template body on {{ var }} → list of static text segments
    # Strip leading whitespace from body to match _extract_xml_body behavior
    # VAR_RE.split with capture group returns: [static0, var0, static1, var1, ...]
    all_parts = VAR_RE.split(template_body.lstrip())
    # Only even-indexed parts are static text; odd are variable names
    static_segments = [all_parts[i] for i in range(0, len(all_parts), 2)]

    # Total static characters in the template
    total_static = sum(len(s) for s in static_segments)
    if total_static == 0:
        return 1.0  # template with no static text (degenerate)

    # Walk rendered text, find each segment in order
    matched_chars = 0
    search_pos = 0
    for segment in static_segments:
        if not segment:
            continue
        # Find this segment in the rendered text at or after search_pos
        idx = rendered_text.find(segment, search_pos)
        if idx >= 0:
            matched_chars += len(segment)
            search_pos = idx + len(segment)
        # else: segment not found — contributes 0 matched chars

    return matched_chars / total_static


def extract_variables(
    template_path: str | Path,
    rendered_path: str | Path,
    *,
    include_vars: list[str] | None = None,
    exclude_vars: list[str] | None = None,
    include_metadata: bool = True,
) -> dict[str, Any]:
    """Extract variable bindings from a rendered template.

    Args:
        template_path: Path to the .j2 template file.
        rendered_path: Path to the rendered output file.
        include_vars: If set, ONLY extract these variables.
        exclude_vars: If set, skip these variables.
        include_metadata: Include _extractions metadata in output.

    Returns:
        Dictionary of variable_name → extracted_value.
    """
    template_text = Path(template_path).read_text()
    rendered_text = Path(rendered_path).read_text()

    # Strip any leading non-XML content (ATM message headers, log lines, etc.)
    rendered_text = _extract_xml_body(rendered_text).rstrip()

    # Parse template
    fm = parse_frontmatter(template_text)
    body = parse_body(template_text)
    bindings = discover_bindings(body)

    required = fm.get("required_variables", [])
    optional = fm.get("optional_variables", [])
    defaults = fm.get("defaults", {})

    # Parse rendered XML
    root = ET.fromstring(rendered_text)

    # Extract each variable
    result: dict[str, Any] = {}
    extractions: list[VariableExtraction] = []

    for binding in bindings:
        # Check filters
        if not _should_extract(binding.name, include_vars, exclude_vars):
            extractions.append(
                VariableExtraction(
                    name=binding.name,
                    value="",
                    source="skipped",
                )
            )
            continue

        strategy = EXTRACTION_STRATEGIES[binding.kind]

        if binding.kind == "attribute":
            value = strategy(root, binding.tag, binding.attribute)
            xpath = f"//{binding.tag}/@{binding.attribute}"
        elif binding.kind == "element_text":
            value = strategy(root, binding.tag)
            xpath = f"//{binding.tag}/text()"
        else:  # block_text
            value = strategy(root, binding.tag)
            xpath = f"//{binding.tag}/text()"

        extraction = VariableExtraction(
            name=binding.name,
            value=value or "",
            source=binding.kind if value else "not_found",
            xpath=xpath,
        )
        extractions.append(extraction)

        if value:
            result[binding.name] = value

    # Apply defaults for missing variables
    for var in required + optional:
        if var not in result and var in defaults:
            result[var] = defaults[var]

    # Compute confidence: % of static template text matching rendered output
    confidence = compute_confidence(body, rendered_text)

    if include_metadata:
        result["_extractions"] = [
            {
                "name": e.name,
                "value": e.value[:80] + "..." if len(e.value) > 80 else e.value,
                "source": e.source,
                "xpath": e.xpath,
            }
            for e in extractions
        ]
        result["_confidence"] = round(confidence, 4)

    return result
