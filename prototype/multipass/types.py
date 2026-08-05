"""Shared types for multi-pass template system."""

from dataclasses import dataclass, field
from typing import Any


@dataclass
class PassHeader:
    """A single YAML frontmatter header for one render pass."""

    pass_number: int = 1  # Which pass this header belongs to (1-based)
    required_variables: list[str] = field(default_factory=list)
    defaults: dict[str, Any] = field(default_factory=dict)
    metadata: dict[str, Any] = field(default_factory=dict)

    @property
    def brace_count(self) -> int:
        """Variable delimiter brace count for this pass.
        
        Pass 1 → {{ }}, Pass 2 → {{{ }}}, Pass 3 → {{{{ }}}}, etc.
        """
        return self.pass_number + 1


@dataclass
class ParsedTemplate:
    """A template file parsed into stacked headers + body."""

    passes: list[PassHeader]
    body: str

    @property
    def pass_count(self) -> int:
        """Number of render passes remaining."""
        return len(self.passes)

    @property
    def is_fully_rendered(self) -> bool:
        """True if no passes remain (ready for deployment)."""
        return len(self.passes) == 0


@dataclass
class RenderContext:
    """Variables provided for a single render pass."""

    pass_number: int
    variables: dict[str, Any] = field(default_factory=dict)
