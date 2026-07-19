"""template-init: Convert concrete files into stacked multi-pass templates.

Scans a file for literal values and replaces them with {N}-brace variables,
generating stacked YAML headers.

DD-005 compliant: longest-match-first replacement order.
"""

from __future__ import annotations

import re
import yaml
from collections import OrderedDict
from pathlib import Path
from typing import Any


class InitPass:
    """Configuration for one pass in template-init."""

    def __init__(self, pass_number: int, variables: dict[str, str]):
        """
        Args:
            pass_number: Which pass this belongs to.
            variables: Mapping of variable_name → concrete_value_to_replace.
        """
        self.pass_number = pass_number
        self.variables = variables


def template_init(
    file_path: str | Path,
    passes: list[InitPass],
    *,
    dry_run: bool = False,
    force: bool = False,
) -> TemplateInitResult:
    """Convert a concrete file into a stacked multi-pass template.

    Args:
        file_path: Path to the concrete file to convert.
        passes: Ordered list of InitPass configs (outermost first).
        dry_run: If True, return result without writing to disk.
        force: If True, overwrite existing file.

    Returns:
        TemplateInitResult with the generated template and metadata.

    Raises:
        FileNotFoundError: If the file doesn't exist.
        FileExistsError: If dry_run=False, force=False, and file would change.
        ValueError: If a variable value is not found in the file.
    """
    path = Path(file_path)
    if not path.exists():
        raise FileNotFoundError(f"file not found: {path}")

    original = path.read_text()
    content = original

    # Collect ALL replacements across all passes
    # Format: (pass_number, variable_name, value, brace_count)
    all_replacements: list[tuple[int, str, str, int]] = []
    for init_pass in passes:
        brace_count = init_pass.pass_number + 1
        for var_name, value in init_pass.variables.items():
            all_replacements.append(
                (init_pass.pass_number, var_name, value, brace_count)
            )

    # Sort by value length descending → longest-match-first (DD-006)
    all_replacements.sort(key=lambda x: len(x[2]), reverse=True)

    # Validate all values found in file
    not_found: list[tuple[str, str]] = []
    for _, var_name, value, _ in all_replacements:
        if value not in content:
            not_found.append((var_name, value))

    if not_found:
        names = ", ".join(f"{var}={val}" for var, val in not_found)
        raise ValueError(
            f"values not found in file: {names}. "
            f"Check for typos or differences in whitespace/encoding."
        )

    # Replace values with brace-count variables (longest-first)
    for pass_number, var_name, value, brace_count in all_replacements:
        open_delim = "{" * brace_count
        close_delim = "}" * brace_count
        replacement = f"{open_delim} {var_name} {close_delim}"
        content = content.replace(value, replacement)

    # Build stacked headers (outermost pass first)
    header_lines: list[str] = []
    for init_pass in passes:
        header = {
            "required_variables": list(init_pass.variables.keys()),
            "defaults": init_pass.variables,
        }
        if init_pass.pass_number > 1 or len(passes) > 1:
            header["pass"] = init_pass.pass_number
        header_lines.append("---")
        header_lines.append(
            yaml.dump(header, default_flow_style=False, sort_keys=False).rstrip()
        )
        header_lines.append("---")

    template_text = "\n".join(header_lines) + "\n" + content.lstrip("\n")

    # Detect changes
    changed = template_text != original
    would_change = changed

    result = TemplateInitResult(
        target_path=path,
        original=original,
        template_text=template_text,
        discovered_variables=[
            name for _, name, _, _ in all_replacements
        ],
        changed=changed,
        would_change=would_change,
    )

    if not dry_run and changed:
        if not force and path.exists():
            raise FileExistsError(
                f"target file {path} already exists and would change. "
                f"Use force=True to overwrite."
            )
        path.write_text(template_text)
        result.changed = True
    elif dry_run:
        result.changed = False  # dry run never writes

    return result


class TemplateInitResult:
    """Result of a template-init operation."""

    def __init__(
        self,
        target_path: Path,
        original: str,
        template_text: str,
        discovered_variables: list[str],
        changed: bool,
        would_change: bool,
    ):
        self.target_path = target_path
        self.original = original
        self.template_text = template_text
        self.discovered_variables = discovered_variables
        self.changed = changed
        self.would_change = would_change

    def __repr__(self) -> str:
        return (
            f"TemplateInitResult(path={self.target_path}, "
            f"vars={self.discovered_variables}, "
            f"changed={self.changed})"
        )


def _reconstruct(header: dict, body: str) -> str:
    """Reconstruct template text from header + body (for testing)."""
    lines = ["---"]
    if header:
        lines.append(yaml.dump(header, default_flow_style=False).rstrip())
    lines.append("---")
    lines.append(body)
    return "\n".join(lines) + "\n"
