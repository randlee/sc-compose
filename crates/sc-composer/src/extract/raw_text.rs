//! Format-neutral matching for known-template scalar values.

use std::ops::Range;

use crate::types::VariableName;

/// A static template segment or a scalar variable expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RawTextSegment<'a> {
    /// Literal text that must occur exactly in the candidate value.
    Static(&'a str),
    /// A scalar variable whose rendered spelling is captured.
    Variable(VariableName),
}

/// Input to the shared raw-text matcher.
pub(crate) struct RawTextMatchInput<'a> {
    /// Template-side segments produced by [`parse_raw_text_segments`].
    pub(crate) segments: &'a [RawTextSegment<'a>],
    /// Candidate value identified by the format adapter.
    pub(crate) rendered_candidate: &'a str,
}

/// One captured scalar variable and its candidate-relative byte span.
pub(crate) struct RawTextCapture {
    /// Captured variable name.
    pub(crate) variable: VariableName,
    /// Byte span relative to the candidate value.
    pub(crate) span: Range<usize>,
    /// Rendered string at the capture span.
    pub(crate) rendered_text: String,
}

/// A candidate ambiguity that can be localized to a byte span.
pub(crate) struct RawTextAmbiguity {
    /// Candidate-relative ambiguity span, when known.
    pub(crate) span: Option<Range<usize>>,
    /// Human-readable ambiguity explanation.
    pub(crate) message: String,
}

/// Result of shared raw-text matching.
pub(crate) struct RawTextMatch {
    /// Captured variables in template order.
    pub(crate) captures: Vec<RawTextCapture>,
    /// Non-fatal ambiguity, if an adapter elects to preserve one.
    pub(crate) ambiguity: Option<RawTextAmbiguity>,
    /// Number of non-empty static segments matched.
    pub(crate) static_matches: usize,
}

/// A format-neutral matching failure.
pub(crate) enum RawTextMatchError {
    /// The template cannot be interpreted as the supported scalar subset.
    InvalidTemplate {
        /// Candidate-relative location, when available.
        span: Option<Range<usize>>,
        /// Human-readable explanation.
        message: String,
    },
    /// Static text does not match the candidate.
    StaticMismatch {
        /// Candidate-relative location, when available.
        span: Option<Range<usize>>,
        /// Human-readable explanation.
        message: String,
    },
    /// Adjacent variables or repeated delimiters make capture boundaries ambiguous.
    AmbiguousDelimiter {
        /// Candidate-relative location, when available.
        span: Option<Range<usize>>,
        /// Human-readable explanation.
        message: String,
    },
}

/// Scope of a raw-text failure for adapter propagation.
pub(crate) enum RawTextErrorScope {
    /// The template request itself is invalid.
    Request,
    /// One candidate occurrence is invalid while others may continue.
    Occurrence,
}

impl RawTextMatchError {
    /// Return the programmatic request/occurrence scope marker.
    pub(crate) const fn scope(&self) -> RawTextErrorScope {
        match self {
            Self::InvalidTemplate { .. } => RawTextErrorScope::Request,
            Self::StaticMismatch { .. } | Self::AmbiguousDelimiter { .. } => {
                RawTextErrorScope::Occurrence
            }
        }
    }
}

/// Format a raw-text diagnostic with its candidate-relative byte span.
pub(crate) fn format_diagnostic_message(message: &str, span: Option<Range<usize>>) -> String {
    span.map_or_else(
        || message.to_owned(),
        |span| format!("{message} (candidate bytes {}..{})", span.start, span.end),
    )
}

/// Parse the supported double-brace scalar expression subset.
pub(crate) fn parse_raw_text_segments(
    value: &str,
) -> Result<Vec<RawTextSegment<'_>>, RawTextMatchError> {
    let mut segments = Vec::new();
    let mut cursor = 0;
    while let Some(relative_open) = value[cursor..].find("{{") {
        let open = cursor + relative_open;
        if value[open..].starts_with("{{{") {
            return Err(invalid_template(
                "extraction supports only double-brace scalar expressions",
            ));
        }
        if open > cursor {
            segments.push(RawTextSegment::Static(&value[cursor..open]));
        }
        let expression_start = open + 2;
        let Some(relative_close) = value[expression_start..].find("}}") else {
            return Err(invalid_template(
                "template contains an unterminated expression",
            ));
        };
        let close = expression_start + relative_close;
        if value[close..].starts_with("}}}") {
            return Err(invalid_template(
                "extraction supports only double-brace scalar expressions",
            ));
        }
        let expression = value[expression_start..close].trim();
        let variable = VariableName::new(expression).map_err(|error| {
            invalid_template(format!(
                "unsupported template expression: {{{{ {expression} }}}}: {error}"
            ))
        })?;
        if variable.as_str().contains('.') {
            return Err(invalid_template(format!(
                "dotted extraction expression is unsupported: {{{{ {expression} }}}}"
            )));
        }
        segments.push(RawTextSegment::Variable(variable));
        cursor = close + 2;
    }
    if cursor < value.len() {
        segments.push(RawTextSegment::Static(&value[cursor..]));
    }
    if segments.is_empty() {
        segments.push(RawTextSegment::Static(value));
    }
    if segments.windows(2).any(|pair| {
        matches!(
            (&pair[0], &pair[1]),
            (RawTextSegment::Variable(_), RawTextSegment::Variable(_))
        )
    }) {
        return Err(RawTextMatchError::AmbiguousDelimiter {
            span: None,
            message: "adjacent variable expressions have no structural delimiter".to_owned(),
        });
    }
    Ok(segments)
}

/// Match parsed template segments against one candidate value.
pub(crate) fn match_raw_text(
    input: &RawTextMatchInput<'_>,
) -> Result<RawTextMatch, RawTextMatchError> {
    let variables = input
        .segments
        .iter()
        .filter_map(|segment| match segment {
            RawTextSegment::Variable(variable) => Some(variable),
            RawTextSegment::Static(_) => None,
        })
        .count();
    if variables == 0 {
        let expected = input
            .segments
            .iter()
            .find_map(|segment| match segment {
                RawTextSegment::Static(value) if !value.is_empty() => Some(*value),
                _ => None,
            })
            .unwrap_or_default();
        if input.rendered_candidate != expected {
            return Err(static_mismatch(
                "rendered static content does not match the known template",
                None,
            ));
        }
        return Ok(RawTextMatch {
            captures: Vec::new(),
            ambiguity: None,
            static_matches: usize::from(!expected.is_empty()),
        });
    }

    let mut cursor = 0;
    let mut captures = Vec::new();
    let mut static_matches = 0;
    for (index, segment) in input.segments.iter().enumerate() {
        match segment {
            RawTextSegment::Static(static_text) => {
                if !input.rendered_candidate[cursor..].starts_with(static_text) {
                    return Err(static_mismatch(
                        "rendered static content does not match the known template",
                        Some(cursor..cursor.saturating_add(static_text.len())),
                    ));
                }
                cursor += static_text.len();
                static_matches += usize::from(!static_text.is_empty());
            }
            RawTextSegment::Variable(variable) => {
                let next_static =
                    input
                        .segments
                        .iter()
                        .skip(index + 1)
                        .find_map(|next| match next {
                            RawTextSegment::Static(value) if !value.is_empty() => Some(*value),
                            _ => None,
                        });
                let end = if let Some(next_static) = next_static {
                    let remainder = &input.rendered_candidate[cursor..];
                    let Some(offset) = remainder.find(next_static) else {
                        return Err(static_mismatch(
                            "rendered input is missing static suffix around a variable",
                            Some(cursor..input.rendered_candidate.len()),
                        ));
                    };
                    if remainder[offset + next_static.len()..].contains(next_static) {
                        return Err(RawTextMatchError::AmbiguousDelimiter {
                            span: Some(cursor..cursor + offset),
                            message: "static suffix occurs multiple times around a variable"
                                .to_owned(),
                        });
                    }
                    cursor + offset
                } else {
                    input.rendered_candidate.len()
                };
                captures.push(RawTextCapture {
                    variable: variable.clone(),
                    span: cursor..end,
                    rendered_text: input.rendered_candidate[cursor..end].to_owned(),
                });
                cursor = end;
            }
        }
    }
    if cursor != input.rendered_candidate.len() {
        return Err(static_mismatch(
            "rendered input has trailing content outside the known template",
            Some(cursor..input.rendered_candidate.len()),
        ));
    }
    Ok(RawTextMatch {
        captures,
        ambiguity: None,
        static_matches,
    })
}

fn invalid_template(message: impl Into<String>) -> RawTextMatchError {
    RawTextMatchError::InvalidTemplate {
        span: None,
        message: message.into(),
    }
}

fn static_mismatch(message: impl Into<String>, span: Option<Range<usize>>) -> RawTextMatchError {
    RawTextMatchError::StaticMismatch {
        span,
        message: message.into(),
    }
}
