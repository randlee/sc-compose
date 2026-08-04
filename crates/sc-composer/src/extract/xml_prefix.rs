//! Bounded normalization of the approved rendered-XML preamble.

use super::ExtractError;

pub(super) struct NormalizedXml {
    pub(super) source: String,
    pub(super) removed: Option<RemovedPrefix>,
}

pub(super) struct RemovedPrefix {
    pub(super) byte_end: usize,
    pub(super) line: usize,
    pub(super) column: usize,
}

pub(super) fn normalize_rendered(source: &str) -> Result<NormalizedXml, ExtractError> {
    let bytes = source.as_bytes();
    let mut cursor = 0;
    let mut retained_start = None;
    let mut saw_prolog = false;

    while cursor < bytes.len() {
        if bytes[cursor] != b'<' {
            cursor += 1;
            continue;
        }

        if source[cursor..].starts_with("<!--") {
            let Some(end) = source[cursor + 4..].find("-->") else {
                return Err(ExtractError::malformed(
                    "XML preamble contains an unterminated comment".to_owned(),
                ));
            };
            retained_start.get_or_insert(cursor);
            saw_prolog = true;
            cursor += 4 + end + 3;
            continue;
        }

        if source[cursor..].starts_with("<?") {
            let Some(end) = source[cursor + 2..].find("?>") else {
                return Err(ExtractError::malformed(
                    "XML preamble contains an unterminated processing instruction".to_owned(),
                ));
            };
            let declaration = source[cursor..].starts_with("<?xml")
                && bytes
                    .get(cursor + 5)
                    .is_none_or(|byte| byte.is_ascii_whitespace() || *byte == b'?');
            if declaration && saw_prolog {
                return Err(ExtractError::malformed(
                    "XML declaration must be first in the retained prolog".to_owned(),
                ));
            }
            retained_start.get_or_insert(cursor);
            saw_prolog = true;
            cursor += 2 + end + 2;
            continue;
        }

        if source[cursor..].starts_with("<!") {
            return Err(ExtractError::unsupported(
                "XML DTD or declaration markup is outside the reversible extraction subset",
            ));
        }

        // The first non-prolog '<' is the only root candidate. The existing
        // XML parser validates its complete element and all following input.
        if bytes
            .get(cursor + 1)
            .is_none_or(|byte| *byte == b'/' || *byte == b' ')
        {
            return Err(ExtractError::malformed(
                "XML preamble contains ambiguous or malformed markup".to_owned(),
            ));
        }
        let start = retained_start.unwrap_or(cursor);
        let (line, column) = line_column(source, start);
        return Ok(NormalizedXml {
            source: source[start..].to_owned(),
            removed: (start > 0).then_some(RemovedPrefix {
                byte_end: start,
                line,
                column,
            }),
        });
    }

    Err(ExtractError::malformed(
        "XML input has no root element".to_owned(),
    ))
}

fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let prefix = &source[..byte_offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = source[column_start..byte_offset].chars().count() + 1;
    (line, column)
}
