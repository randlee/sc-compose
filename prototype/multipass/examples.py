"""Examples and fixtures for the multi-pass prototype."""

import tempfile
from pathlib import Path

# ── Example 1: 2-pass rust-team-deployment template ──────────────────────

TWO_PASS_TEMPLATE = """---
pass: 2
required_variables:
- team_name
- repo_name
defaults:
  codex_agent: cwy
---
---
pass: 1
required_variables:
- task.title
- variant
defaults:
  variant: claude
---
# Agent: arch-{{{ codex_agent }}}

**Team:** {{{ team_name }}}
**Repo:** {{{ repo_name }}}

## Task

{{ task.title }}

**Variant:** {{ variant }}
"""

TWO_PASS_DEPLOY_VARS = {
    2: {"team_name": "wyvern", "repo_name": "atm-core"},
    1: {"task": {"title": "Fix UDP transport"}, "variant": "opencode"},
}

TWO_PASS_EXPECTED_OUTPUT = """# Agent: arch-cwy

**Team:** wyvern
**Repo:** atm-core

## Task

Fix UDP transport

**Variant:** opencode"""

# ── Example 2: Single-pass backward compat ───────────────────────────────

SINGLE_PASS_TEMPLATE = """---
required_variables:
- name
defaults:
  name: world
---
hello {{ name }}
"""

SINGLE_PASS_EXPECTED = "hello world"

# ── Example 3: 3-pass template ───────────────────────────────────────────

THREE_PASS_TEMPLATE = """---
pass: 3
required_variables:
- cluster
defaults:
  cluster: production
---
---
pass: 2
required_variables:
- namespace
defaults:
  namespace: default
---
---
pass: 1
required_variables:
- service.name
---
Cluster: {{{{ cluster }}}}
Namespace: {{{ namespace }}}
Service: {{ service.name }}
"""

THREE_PASS_VARS = {
    3: {"cluster": "staging"},
    2: {"namespace": "wyvern-api"},
    1: {"service": {"name": "gateway"}},
}

THREE_PASS_EXPECTED = """Cluster: staging
Namespace: wyvern-api
Service: gateway"""


def write_temp(path: str, content: str) -> Path:
    """Write content to a temp file, return Path."""
    import tempfile
    tmp = Path(tempfile.mktemp(suffix=path))
    tmp.parent.mkdir(parents=True, exist_ok=True)
    tmp.write_text(content)
    return tmp
