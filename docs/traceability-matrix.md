# SC-Compose Traceability Matrix

This document maps requirements and non-functional requirements to the owning
architecture surfaces, implementation sprints, primary modules, and expected
verification layers.

## Functional Requirements

| Requirement | Primary architecture sections | Planned sprint(s) | Primary modules / APIs | Verification |
| --- | --- | --- | --- | --- |
| FR-1 Template Inputs | §6 Frontmatter Model | S2 | `frontmatter`, shared file and foundational type handling | unit tests, integration fixtures |
| FR-1a Frontmatter Schema | §6 Frontmatter Model | S2 | `frontmatter`, foundational type modules, `ComposeRequest` | unit tests |
| FR-1b Value Types | §6 Frontmatter Model, §11 Error Model | S2, S4 | foundational type modules, `frontmatter`, `validate` | unit tests, validation tests |
| FR-1c File Extension and Discovery | §5 Resolver Path Policy | S3 | `resolver` | resolver tests |
| FR-2 Variable Resolution and Precedence | §7 Variable and Token Semantics, §12 Request Lifecycle | S2, S4 | `context`, `tokens`, `validate` | unit tests, integration tests |
| FR-2a Undeclared Tokens | §7 Variable and Token Semantics | S4 | `tokens`, `validate`, `render`, `Renderer`, `compose()` | unit tests, golden tests |
| FR-2b Missing and Extra Variables | §7 Variable and Token Semantics, §11 Error Model | S4 | `validate`, `diagnostics`, `ComposeError` | unit tests, diagnostics tests |
| FR-3 Include Expansion | §9 Include and Frontmatter Merge Rules, §12 Request Lifecycle | S3 | `include`, `resolver` | unit tests, smoke tests |
| FR-3a Frontmatter Across Includes | §9 Include and Frontmatter Merge Rules | S4 | `include`, `frontmatter`, `context`, `validate` | unit tests, smoke tests |
| FR-4 Safety Constraints | §16 Safety Model | S3, S6 | `include`, confinement helpers, `ConfigError`, `IncludeError` | unit tests, cross-platform tests |
| FR-5 Prompt Resolution Conventions | §5 Resolver Path Policy | S3 | `resolver`, `ResolveResult`, `resolve_profile()` | resolver tests, JSON golden tests |
| FR-6 Composition Pipeline | §8 Public API Shape, §12 Request Lifecycle, §13 CLI Command Architecture | S4, S5 | `pipeline`, `Renderer`, `compose()`, CLI `render` | unit tests, integration tests |
| FR-7 CLI Surface | §13 CLI Command Architecture, §14 Output Path Policy, §15 `init` Command Behavior | S5 | `sc-compose` commands, `frontmatter_init()`, `init_workspace()` | CLI integration tests |
| FR-7a Variable File Rules | §15 `init` Command Behavior, command schema sections | S5 | var-file parser, CLI input wiring | integration tests |
| FR-7b Exit Codes | §17 Error and Exit Semantics | S5 | CLI command router, exit handling | integration tests |
| FR-7c Template Whitespace Control | §4 Module Architecture (`render`), §8 API Ownership Matrix | S4, S5 | `render`, `Renderer`, CLI `render` | golden tests |
| FR-8 Determinism and Diagnostics | §10 Diagnostics Model, §11 Error Model, §17 Error and Exit Semantics | S2, S4, S5, S6 | `diagnostics`, `error`, `validate()`, CLI JSON shaping | unit tests, golden tests, smoke tests |
| FR-8a Command JSON and Dry-Run Schemas | §13.1 Command Output Schemas | S5, S6 | CLI JSON serializers, `render`, `resolve`, `validate`, `init`, `frontmatter-init` | snapshot tests, integration tests |
| FR-9 Observability | §18 Observability Integration | S4, S5 | `observability`, open observer/sink traits, CLI binding | unit tests, integration tests |

## Non-Functional Requirements

| NFR | Planned sprint(s) | Primary enforcement point | Verification |
| --- | --- | --- | --- |
| Cross-platform correctness | S3, S6 | resolver/include/confinement behavior | cross-platform and path tests |
| Interactive performance | S5, S6 | `Renderer`, command hot paths, output shaping | smoke tests, benchmark sanity checks |
| Public API stability | S2, S6 | typed API surface and semver review | API review, final design review |
| Crate separability | S2 through S6 | dependency direction and boundary rules | CI, manifest review, QA |

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

- `qm-comp` owns QA review on sprint exit gates.
- `arch-ctm` owns the final design review at the phase exit gate.
- `arch-comp` owns branch-level coordination and handoff readiness.
