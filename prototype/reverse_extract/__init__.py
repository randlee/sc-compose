"""Reverse template variable extractor.

Given a j2 template and its rendered output, extract the JSON
variable bindings that produced it — the reverse of rendering.

Supports XML, JSON, and Markdown rendered outputs.
"""

from .extractor import extract_variables, VariableExtraction

__all__ = ["extract_variables", "VariableExtraction"]
