# Phase U intent — expanded draft for Rand approval

## Purpose (verbatim)

> the intent of this phase is to create the much needed beads extensions so consumers don't need to write/maintain scripts to pour beads, avoid render bugs in bd.

Source: Rand via lead, ATM01M6JA14QKWCFW1QV4F6SH43NH.

> intent (part 2) make sure sc-compose does not limit consumers due to python version or platform

Source: Rand via lead, ATM01M6JA233ZPP2RCZ2JWFDCFEYZ.

## Consumers and the problem today

- Consumers currently write and maintain pour scripts. Source: purpose quote1.
- bd rendering is unreliable across fields: probed bd1.3.2-dev407fe27 leaves labels unexpanded and rewrites triple braces under runtime vars. Source: purpose quote1; `bd-132-probe/evidence.md` BD-V1/V4 (observations, not additional user requirements).
- ABI3 wheels now target CPython3.11+ (abi3-py311);3.12+ reuse them. Python<=3.10 is deliberately excluded; macOS CLT /usr/bin/python3 at3.9 needs an upgrade. Source: Rand via lead01M6JAJWE373EJC3EV62AZHE14. cp311-only wheels and missing platform artifacts limit installations. Source: purpose quote2; user platform quote, transcript8470; v1.7.0 publish manifest (baseline observations).

## Success for a consumer

- Pour beads without maintaining a consumer pour script. Source: purpose quote1.
- Render tight, padded and loop-local variable expressions before bd. Source: transcript8223, “Variables rendered before bd. Tight {{key}} and padded {{ key }}, including loop-local values.”
- Keep brace-bearing values as data; filters see the actual value. Source: transcript8223, “Values containing {{ }}, {{{ }}} or {% %} stay data and are never re-parsed.” and “Filters act on the real value, so A{BC lowercases to a{bc and has length 4.”
- Use include defaults and bead inputs with graph/attach pouring. Source: transcript8223, “Include defaults.” and “Bead inputs on graph and attach pour”.
- Obtain Rust/wheel/Go artifacts for macOS, Windows and Linux on x64/arm64 through applicable channels. Source: transcript8470, “support of mac x64/arm, win x64/arm, linux x64/arm for rust/wheel/go distributed through all applicable channels”.

## Boundaries and non-goals

- sc-compose constructs structure; bd validates and creates. Source: lead01M6JA2YXXZD98FVG6BS39WGRD identifies this as Rand's stated boundary; FR24/ADR0024 records it. Original transcript quote not recovered yet.
- Schema-less rendering stays byte-identical to v1.7.0 except genuine bugs. Source: Rand via lead01M6JAKMVZ5RXHGS45A59P45G6, “schema-less templates render byte-identical to v1.7.0 except genuine bugs”; exception clarified by transcript6183.
- Publishing is after Phase U, with1.8.0 assumed. Source: transcript5233, “we will publish AFTER phase-u, do not include publishing as part of phase-u.”
- No new consumer-specific features; sc-compose does not guarantee bd concurrency. Source: lead01M6JA2YXXZD98FVG6BS39WGRD identifies these as Rand-stated boundaries; original quotes/tests still to locate, not new requirements invented here.

## Standing constraints

- Lint passes. Source: memory/lint-must-pass-allowlist-words.md, “you need to attribute or add the words so lint passes”.
- Tests use controlled inputs, avoiding flaky race/timing/sleep/load outcomes. Source: ~/.claude/CLAUDE.md standing prohibition.
- Windows changes receive an xwin cross-check. Source: memory/windows-cross-check-before-ci.md, “use xwin”.
- docli runs every publish; stale docs block preflight. Source: transcript8326, “docli must be run every publish (stale documentation blocks preflight)”.
- Full install matrix runs only in the preflight-required integration test. Source: transcript8644, “I want ci running full matrix ONLY for integration test (required for preflight) NOT every sprint”.

## Questions for Rand

- The existing tests settle defaults and blocked-path timing. Generic JSON-depth safety needs a separate source quote; the unsafe-graph quote does not define generic render policy. New safety drafts and fixtures remain unapproved; Option A wording was approved via lead01M6JAPEKYDRBSBMQMD2PRCG0G, without locking.
