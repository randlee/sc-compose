"""Multi-pass template prototype — package init."""

from .types import ParsedTemplate, PassHeader, RenderContext
from .parser import parse_template
from .discover import discover_tokens, discover_all_tokens
from .renderer import render_pass, render_all

__all__ = [
    "ParsedTemplate",
    "PassHeader",
    "RenderContext",
    "parse_template",
    "discover_tokens",
    "discover_all_tokens",
    "render_pass",
    "render_all",
]
