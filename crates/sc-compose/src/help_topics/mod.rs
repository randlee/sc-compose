//! Bundled feature manuals exposed through the `sc-compose help` command.

/// The single-touch-point registry for manuals shipped with the CLI.
///
/// Follow-on feature manuals should add one ordered entry here and a linked
/// document under `docs/manual/`; the command and list output then stay
/// deterministic without duplicating topic metadata elsewhere.
pub(crate) const TOPICS: &[(&str, &str)] = &[
    (
        "exit-codes",
        include_str!("../../../../docs/manual/exit-codes.md"),
    ),
    ("render", include_str!("../../../../docs/manual/render.md")),
    (
        "resolve",
        include_str!("../../../../docs/manual/resolve.md"),
    ),
    (
        "validate",
        include_str!("../../../../docs/manual/validate.md"),
    ),
    ("verify", include_str!("../../../../docs/manual/verify.md")),
    (
        "extract",
        include_str!("../../../../docs/manual/extract.md"),
    ),
    (
        "template-init",
        include_str!("../../../../docs/manual/template-init.md"),
    ),
    (
        "frontmatter-init",
        include_str!("../../../../docs/manual/frontmatter-init.md"),
    ),
    ("init", include_str!("../../../../docs/manual/init.md")),
    (
        "examples",
        include_str!("../../../../docs/manual/examples.md"),
    ),
    (
        "templates",
        include_str!("../../../../docs/manual/templates.md"),
    ),
    (
        "reports",
        include_str!("../../../../docs/manual/reports.md"),
    ),
];

pub(crate) fn topic_names() -> Vec<&'static str> {
    TOPICS.iter().map(|(name, _)| *name).collect()
}

pub(crate) fn find(topic: &str) -> Option<&'static str> {
    TOPICS
        .iter()
        .find(|(name, _)| *name == topic)
        .map(|(_, manual)| *manual)
}

pub(crate) fn index() -> String {
    let mut output = String::from("Available manual topics:\n");
    for (topic, _) in TOPICS {
        output.push_str("  ");
        output.push_str(topic);
        output.push('\n');
    }
    output
}
