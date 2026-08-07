pub(super) fn parse_include_directive(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    (trimmed.starts_with("@<") && trimmed.ends_with('>') && trimmed.len() > 3)
        .then(|| &trimmed[2..trimmed.len() - 1])
}
