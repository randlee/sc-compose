# SC-Compose Traceability Matrix

This document maps requirements and non-functional requirements to the owning
architecture surfaces, release-plan sprints, primary modules, and expected
verification layers.

## Functional Requirements

| Requirement | Primary architecture sections | Planned sprint(s) | Primary modules / APIs | Verification |
| --- | --- | --- | --- | --- |
| FR-1 Template Inputs | §6 Frontmatter Model | Sprint 4 | `frontmatter`, shared file and foundational type handling | unit tests, integration fixtures |
| FR-1a Frontmatter Schema | §6 Frontmatter Model | Sprint 4 | `frontmatter`, foundational type modules, `ComposeRequest` | unit tests |
| FR-1b Value Types | §6 Frontmatter Model, §11 Error Model | Sprint 4, Sprint S7 | foundational type modules, `frontmatter`, `validate` | unit tests, validation tests, CLI var-file tests |
| FR-1d Template Pack Layout | §13 CLI Command Architecture, §15 Template Pack Architecture | Sprint S7 | `TemplateStore`, examples/templates command wiring | CLI integration tests |
| FR-1c File Extension and Discovery | §5 Resolver Path Policy | Sprint 4 | `resolver` | resolver tests |
| FR-2 Variable Resolution and Precedence | §7 Variable and Token Semantics, §12 Request Lifecycle | Sprint 4, Sprint S7 | `validation`, `validate`, request input merge logic | unit tests, integration tests |
| FR-2a Undeclared Tokens | §7 Variable and Token Semantics | Sprint 4 | `validation`, `validate`, `render`, `Renderer`, `compose()` | unit tests, golden tests |
| FR-2b Missing and Extra Variables | §7 Variable and Token Semantics, §11 Error Model | Sprint 4 | `validate`, `diagnostics`, `ComposeError` | unit tests, diagnostics tests |
| FR-3 Include Expansion | §9 Include and Frontmatter Merge Rules, §12 Request Lifecycle | Sprint 4 | `include`, `resolver` | unit tests, smoke tests |
| FR-3a Frontmatter Across Includes | §9 Include and Frontmatter Merge Rules | Sprint 4, Sprint S7 | `include`, `frontmatter`, `validation` | unit tests, smoke tests |
| FR-4 Safety Constraints | §17 Safety Model | Sprint 3, Sprint 4 | `include`, confinement helpers, `ConfigError`, `IncludeError` | unit tests, cross-platform tests |
| FR-5 Prompt Resolution Conventions | §5 Resolver Path Policy | Sprint 4 | `resolver`, `ResolveResult`, `resolve_profile()` | resolver tests, JSON golden tests |
| FR-6 Composition Pipeline | §8 Public API Shape, §12 Request Lifecycle, §13 CLI Command Architecture | Sprint 4 | `composer`, `Renderer`, `compose()`, CLI `render` | unit tests, integration tests |
| FR-7 CLI Surface | §13 CLI Command Architecture, §14 Output Path Policy, §15 Template Pack Architecture, §16 `init` Command Behavior | Sprint 1, Sprint 4, Sprint S7 | `sc-compose` commands, `TemplateStore`, `frontmatter_init()`, `init_workspace()` | CLI integration tests |
| FR-7a Variable File Rules | §16 `init` Command Behavior, command schema sections | Sprint 4, Sprint S7 | var-file parser, CLI input wiring | integration tests |
| FR-7b Exit Codes | §18 Error and Exit Semantics | Sprint 4 | CLI command router, exit handling | integration tests |
| FR-7c Template Whitespace Control | §4 Module Architecture (`render`), §8 API Ownership Matrix | Sprint 4 | `render`, `Renderer`, CLI `render` | golden tests |
| FR-8 Determinism and Diagnostics | §10 Diagnostics Model, §11 Error Model, §18 Error and Exit Semantics | Sprint 4 | `diagnostics`, `error`, `validate()`, CLI JSON shaping | unit tests, golden tests, smoke tests |
| FR-8a Command JSON and Dry-Run Schemas | §13.1 Command Output Schemas | Sprint 1, Sprint 4, Sprint S7 | CLI JSON serializers, `render`, `resolve`, `validate`, `init`, `frontmatter-init`, `examples`, `templates` | snapshot tests, integration tests |
| FR-9 Observability No-Op And Host Hooks | §19 Observability Integration | Sprint 1, Sprint 2, Sprint 3 | `observer`, `NoopObserver`, observer injection paths | unit tests, integration tests |
| FR-10 Downstream Logging Extension Model | §19 Observability Integration | Sprint 1, Sprint 2, Sprint 3 | open observer traits, `ObservationEvent`, adapter mapping surface | unit tests, integration tests |
| FR-11 CLI Logging And Health Wiring | §19 Observability Integration | Sprint 1, Sprint 2, Sprint 3, Sprint 4 | `observability`, `observer_impl`, `observability-health`, CLI shutdown path | unit tests, integration tests, smoke tests |

## Non-Functional Requirements

| NFR | Planned sprint(s) | Primary enforcement point | Verification |
| --- | --- | --- | --- |
| Cross-platform correctness | Sprint 3, Sprint 4 | resolver/include/confinement behavior | cross-platform and path tests |
| Interactive performance | Sprint 4 | `Renderer`, command hot paths, output shaping | smoke tests, benchmark sanity checks |
| Public API stability | Sprint 1, Sprint 4 | typed API surface and semver review | API review, final design review |
| Crate separability | Sprint 1 through Sprint 4 | dependency direction and boundary rules | CI, repo-boundary tests, manifest review, QA |

## API Surface Ownership

| Surface | Semantic owner | Notes |
| --- | --- | --- |
| `Renderer` | `sc-composer` | Primary long-lived rendering API |
| `compose()` | `sc-composer` | End-to-end composition convenience surface |
| `render_template()` | `sc-composer` | Lower-level one-shot helper |
| `validate()` | `sc-composer` | Validation-only pipeline surface |
| `resolve_profile()` | `sc-composer` | Resolver API |
| `frontmatter_init()` | `sc-composer` | Administrative helper |
| `init_workspace()` | `sc-composer` | Administrative helper |
| `render` / `resolve` / `validate` CLI commands | `sc-compose` | Transport and UX over `sc-composer` |

## Review Gates

- `quality-mgr` owns QA review on sprint exit gates.
- `team-lead` owns the final design review at the phase exit gate.
- `team-lead` owns branch-level coordination and handoff readiness.
