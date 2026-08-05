#!/usr/bin/env python3
"""End-to-end multi-pass demo using a real sc-compose template.

Pipeline: template_init → parse → validate → render → verify
"""

import sys
import tempfile
from pathlib import Path

# Add parent to path so we can import multipass
sys.path.insert(0, str(Path(__file__).parent))

from multipass.parser import parse_template
from multipass.renderer import render_all
from multipass.types import RenderContext
from multipass.template_init import template_init, InitPass
from multipass.validate_passes import validate_passes, print_report
from multipass.verify import verify


def main():
    print("=" * 70)
    print("E2E Multi-Pass Demo: sc-compose frontmatter-demo.md.j2")
    print("=" * 70)

    # ── Step 0: Write a concrete file to convert ──────────────────────────
    print("\n── Step 0: Concrete file (simulating deploy-time + install-time) ──\n")

    concrete = tempfile.NamedTemporaryFile(
        mode="w", suffix=".md", delete=False, prefix="frontmatter-demo-"
    )
    concrete.write("""# api-gateway

- Owner: platform-team
- Environment: production

Review notes:
- Omit `service_name` to see validation fail.
""")
    concrete.close()
    print(f"Written: {concrete.name}")
    print(Path(concrete.name).read_text())

    # ── Step 1: template-init — convert to 2-pass stacked template ────────
    print("── Step 1: template-init (deploy-time pass 2 + invocation-time pass 1) ──\n")

    passes = [
        InitPass(pass_number=2, variables={
            "owner": "platform-team",
            "environment": "production",
        }),
        InitPass(pass_number=1, variables={
            "service_name": "api-gateway",
        }),
    ]

    result = template_init(concrete.name, passes, dry_run=True)
    print("Generated template:")
    print(result.template_text)

    # ── Step 2: Parse ──────────────────────────────────────────────────────
    print("── Step 2: Parse stacked headers ──\n")
    parsed = parse_template(result.template_text)
    for i, h in enumerate(parsed.passes):
        print(f"  Pass {h.pass_number} (brace_count={h.brace_count}): "
              f"required={h.required_variables}, defaults={list(h.defaults.keys())}")
    print(f"  Body ({len(parsed.body)} chars): {parsed.body[:60].strip()}...")

    # ── Step 3: Validate ───────────────────────────────────────────────────
    print("\n── Step 3: Validate per-pass ──\n")

    ctx2 = RenderContext(pass_number=2, variables={
        "owner": "platform-team",
        "environment": "production",
    })
    ctx1 = RenderContext(pass_number=1, variables={
        "service_name": "api-gateway",
    })

    report = validate_passes(parsed, [ctx2, ctx1])
    print_report(report)

    # ── Step 4: Render all passes ──────────────────────────────────────────
    print("\n── Step 4: Render all passes ──\n")
    rendered = render_all(parsed, [ctx2, ctx1])
    print("Rendered output:")
    print(rendered)

    # ── Step 5: Verify: check what happens with drift ─────────────────────
    print("\n── Step 5: Verify (drift check) ──\n")

    # Write deployed file matching expected output
    deployed_path = Path(concrete.name).with_suffix(".deployed.md")
    deployed_path.write_text(rendered)

    # Write template to a file too
    template_path = Path(concrete.name).with_suffix(".md.2.j2")
    template_path.write_text(result.template_text)

    vr = verify(str(deployed_path), str(template_path), [ctx2, ctx1])
    print(f"  Clean: {vr.clean} (exit code {vr.exit_code})")

    # Now introduce drift
    deployed_path.write_text("# Manual edit: someone changed this by hand\n")
    vr_drift = verify(str(deployed_path), str(template_path), [ctx2, ctx1])
    print(f"  After manual edit — Clean: {vr_drift.clean}")
    if vr_drift.diff:
        print(f"  Diff:\n{vr_drift.diff}")

    # ── Cleanup ───────────────────────────────────────────────────────────
    Path(concrete.name).unlink(missing_ok=True)
    deployed_path.unlink(missing_ok=True)
    template_path.unlink(missing_ok=True)

    print("\n" + "=" * 70)
    print("Pipeline complete: template_init → parse → validate → render → verify")
    print("=" * 70)


if __name__ == "__main__":
    main()
