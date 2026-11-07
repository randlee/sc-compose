# ADR-0026 D8: empty included content

`template.j2` includes `e.md`, which is empty (zero bytes).

Empty content has no final line break, so under the D8 trailing-terminator
rule the directive's own line break follows it, and the directive line becomes
an empty line.

- v1.7.0 output: `A\nB\n`
- candidate output: `A\n\nB\n`

This is the zero-length interpretation of the approved D8 trailing rule, not a
separate user approval. See the `ADR-0026-D8-EMPTY-CHILD` entry in
`../../approved-exceptions.json`.
