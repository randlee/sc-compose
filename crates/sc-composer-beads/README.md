# sc-composer-beads

`sc-composer-beads` is the host-neutral Rust library for rendering a Beads
formula template with `sc-composer`, validating it with the authoritative
`bd` executable, and returning a versioned receipt. It does not parse Beads
formula syntax or access a Beads database directly.

The public request and receipt contract is `sc-compose/beads/v1`; see
ADR-0021 in the repository documentation for the stable protocol and safety
requirements.
