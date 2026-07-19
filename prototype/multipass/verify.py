"""verify: Drift check for multi-pass templates.

Renders a template with all passes and diffs against a deployed file.
DD-006 compliant: exit 0 = no drift, exit 1 = drift (unified diff).
"""

from __future__ import annotations

import difflib
from pathlib import Path

from .types import ParsedTemplate, RenderContext
from .parser import parse_template
from .renderer import render_all


class VerifyResult:
    """Result of a template verify operation."""

    def __init__(
        self,
        clean: bool,
        template_path: Path,
        deployed_path: Path,
        rendered: str,
        deployed: str,
        diff: str = "",
    ):
        self.clean = clean
        self.template_path = template_path
        self.deployed_path = deployed_path
        self.rendered = rendered
        self.deployed = deployed
        self.diff = diff

    @property
    def exit_code(self) -> int:
        """0 = clean (no drift), 1 = drift detected."""
        return 0 if self.clean else 1

    def __repr__(self) -> str:
        status = "clean" if self.clean else "DRIFT"
        return f"VerifyResult({status}, template={self.template_path})"


def verify(
    deployed_path: str | Path,
    template_path: str | Path,
    contexts: list[RenderContext],
    *,
    quiet: bool = False,
) -> VerifyResult:
    """Verify that a deployed file matches its multi-pass template source.

    Renders the template through all passes with provided contexts,
    then diffs against the deployed file.

    Args:
        deployed_path: Path to the deployed (concrete) file.
        template_path: Path to the multi-pass template.
        contexts: One RenderContext per pass (outermost first).
        quiet: If True, suppress diff output (for CI use).

    Returns:
        VerifyResult with clean/drift status, rendered output, and diff.

    Raises:
        FileNotFoundError: If either file doesn't exist.
    """
    deployed_p = Path(deployed_path)
    template_p = Path(template_path)

    if not deployed_p.exists():
        raise FileNotFoundError(f"deployed file not found: {deployed_p}")
    if not template_p.exists():
        raise FileNotFoundError(f"template file not found: {template_p}")

    # Parse and render template
    template_text = template_p.read_text()
    parsed = parse_template(template_text)
    rendered = render_all(parsed, contexts)

    # Read deployed file
    deployed = deployed_p.read_text()

    # Diff
    clean = rendered == deployed
    diff = ""
    if not clean:
        diff_lines = difflib.unified_diff(
            deployed.splitlines(keepends=True),
            rendered.splitlines(keepends=True),
            fromfile=str(deployed_p),
            tofile=f"rendered({template_p.name})",
            lineterm="",
        )
        diff = "\n".join(diff_lines)

    return VerifyResult(
        clean=clean,
        template_path=template_p,
        deployed_path=deployed_p,
        rendered=rendered,
        deployed=deployed,
        diff=diff,
    )


def verify_and_report(
    deployed_path: str | Path,
    template_path: str | Path,
    contexts: list[RenderContext],
    *,
    quiet: bool = False,
) -> int:
    """Run verify and print/report results. Returns exit code (0=clean, 1=drift)."""
    result = verify(deployed_path, template_path, contexts, quiet=quiet)

    if result.clean:
        if not quiet:
            print(f"OK  {result.template_path.name} → {result.deployed_path}")
        return 0

    print(f"DRIFT detected: {result.template_path.name} ≠ {result.deployed_path}")
    if result.diff and not quiet:
        print(result.diff)
    return 1
