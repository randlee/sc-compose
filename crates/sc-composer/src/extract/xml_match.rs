//! XML structural and block-content matching.

use std::collections::BTreeMap;

use crate::error::RecoveryHintKind;

use super::{
    Capture, DiagnosticCode, Evidence, ExtractError, ExtractionDiagnosticKind, MAX_XML_OCCURRENCES,
    XmlDocument, XmlElementId, XmlExtractionSource, XmlNode, XmlPathSegment, input_limit_error,
    is_single_variable, map_raw_text_error, parse_value_segments, with_span,
};
use super::{raw_text, xml_serialize};

pub(super) fn match_element(
    template_document: &XmlDocument,
    template_id: XmlElementId,
    rendered_document: &XmlDocument,
    rendered_id: XmlElementId,
    path: &[XmlPathSegment],
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    let template = template_document.element(template_id);
    let rendered = rendered_document.element(rendered_id);
    if template.name != rendered.name {
        return Err(ExtractError::format_error(
            DiagnosticCode::ErrExtractXmlElementMismatch,
            ExtractionDiagnosticKind::Unsupported,
            format!(
                "rendered XML element does not match template structure: expected {}, found {}",
                template.name, rendered.name
            ),
            RecoveryHintKind::InspectInput {
                description: "align rendered element names with the known XML template".to_owned(),
            },
        ));
    }
    evidence.structural_matches += 1;
    match_attributes(template, rendered, path, captures, evidence)?;
    match_children(
        template_document,
        template,
        rendered_document,
        rendered,
        path,
        captures,
        evidence,
    )
}

fn match_attributes(
    template: &super::XmlElement,
    rendered: &super::XmlElement,
    path: &[XmlPathSegment],
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    if template.attributes.len() != rendered.attributes.len()
        || template
            .attributes
            .keys()
            .any(|name| !rendered.attributes.contains_key(name))
    {
        return Err(ExtractError::format_error(
            DiagnosticCode::ErrExtractXmlAttributeMismatch,
            ExtractionDiagnosticKind::Unsupported,
            format!(
                "rendered XML attributes do not match template element {}",
                template.name
            ),
            RecoveryHintKind::InspectInput {
                description: "align rendered XML attributes with the known template".to_owned(),
            },
        ));
    }
    for (name, template_value) in &template.attributes {
        let rendered_value = rendered
            .attributes
            .get(name)
            .expect("attribute presence checked above");
        let mut attribute_path = path.to_owned();
        attribute_path.push(XmlPathSegment::Attribute { name: name.clone() });
        match_value(
            template_value,
            rendered_value,
            &attribute_path,
            &XmlExtractionSource::Attribute { name: name.clone() },
            captures,
            evidence,
        )?;
    }
    Ok(())
}

fn match_children(
    template_document: &XmlDocument,
    template: &super::XmlElement,
    rendered_document: &XmlDocument,
    rendered: &super::XmlElement,
    path: &[XmlPathSegment],
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    let template_children = &template.children;
    let rendered_children = &rendered.children;

    // A full-content placeholder is the one approved shape whose rendered
    // child list is intentionally allowed to differ from the template. The
    // candidate is still matched through the shared raw-text matcher; XML's
    // format-specific responsibility here is only to provide a deterministic
    // representation of the parsed child nodes.
    if rendered_children
        .iter()
        .any(|child| matches!(child, XmlNode::Element(_)))
        && match_full_content(
            rendered_document,
            template_children,
            rendered_children,
            path,
            captures,
            evidence,
        )?
    {
        return Ok(());
    }

    if template_children.len() == 1
        && rendered_children.is_empty()
        && matches!(&template_children[0], XmlNode::Text(value) if is_single_variable(value))
    {
        if let XmlNode::Text(value) = &template_children[0] {
            let segments = parse_value_segments(value.trim(), path)?;
            if let [raw_text::RawTextSegment::Variable(variable)] = segments.as_slice() {
                if captures.len() >= MAX_XML_OCCURRENCES {
                    return Err(input_limit_error(format!(
                        "XML extraction exceeded the maximum of {MAX_XML_OCCURRENCES} occurrences"
                    )));
                }
                captures.push(Capture {
                    variable: variable.clone(),
                    path: path.to_owned(),
                    source: XmlExtractionSource::TextNode,
                    rendered_text: String::new(),
                });
            }
        }
        return Ok(());
    }

    if template_children.len() != rendered_children.len() {
        return Err(ExtractError::format_error(
            DiagnosticCode::ErrExtractXmlChildStructureMismatch,
            ExtractionDiagnosticKind::Unsupported,
            format!(
                "rendered XML child structure does not match template element {}",
                template.name
            ),
            RecoveryHintKind::InspectInput {
                description: "align rendered XML child structure with the known template"
                    .to_owned(),
            },
        ));
    }

    match_child_sequence(
        template_document,
        rendered_document,
        template,
        template_children,
        rendered_children,
        path,
        captures,
        evidence,
    )
}

fn match_child_sequence(
    template_document: &XmlDocument,
    rendered_document: &XmlDocument,
    template: &super::XmlElement,
    template_children: &[XmlNode],
    rendered_children: &[XmlNode],
    path: &[XmlPathSegment],
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    let mut element_ordinals = BTreeMap::<String, usize>::new();
    for (template_child, rendered_child) in template_children.iter().zip(rendered_children) {
        match (template_child, rendered_child) {
            (XmlNode::Text(template_text), XmlNode::Text(rendered_text)) => {
                match_value(
                    template_text,
                    rendered_text,
                    path,
                    &XmlExtractionSource::TextNode,
                    captures,
                    evidence,
                )?;
            }
            (XmlNode::Element(template_id), XmlNode::Element(rendered_id)) => {
                let template_element = template_document.element(*template_id);
                let ordinal = element_ordinals
                    .entry(template_element.name.clone())
                    .or_default();
                let child_path = path
                    .iter()
                    .cloned()
                    .chain([XmlPathSegment::Element {
                        name: template_element.name.clone(),
                        ordinal: *ordinal,
                    }])
                    .collect::<Vec<_>>();
                *ordinal += 1;
                match_element(
                    template_document,
                    *template_id,
                    rendered_document,
                    *rendered_id,
                    &child_path,
                    captures,
                    evidence,
                )?;
            }
            _ => {
                return Err(ExtractError::format_error(
                    DiagnosticCode::ErrExtractXmlChildStructureMismatch,
                    ExtractionDiagnosticKind::Unsupported,
                    format!(
                        "rendered XML node structure does not match template element {}",
                        template.name
                    ),
                    RecoveryHintKind::InspectInput {
                        description: "align rendered XML child structure with the known template"
                            .to_owned(),
                    },
                ));
            }
        }
    }
    Ok(())
}

fn match_full_content(
    rendered_document: &XmlDocument,
    template_children: &[XmlNode],
    rendered_children: &[XmlNode],
    path: &[XmlPathSegment],
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<bool, ExtractError> {
    let [XmlNode::Text(value)] = template_children else {
        return Ok(false);
    };
    let segments = parse_value_segments(value.trim(), path)?;
    let [raw_text::RawTextSegment::Variable(variable)] = segments.as_slice() else {
        return Ok(false);
    };
    let rendered_content =
        xml_serialize::canonical_inner_content(rendered_document, rendered_children);
    let matched = raw_text::match_raw_text(&raw_text::RawTextMatchInput {
        segments: &[raw_text::RawTextSegment::Variable(variable.clone())],
        rendered_candidate: &rendered_content,
    })
    .map_err(|error| map_raw_text_error(error, path))?;
    evidence.structural_matches += 1;
    if let Some(ambiguity) = matched.ambiguity {
        return Err(ExtractError::ambiguous(
            with_span(&ambiguity.message, ambiguity.span),
            None,
        ));
    }
    for capture in matched.captures {
        if captures.len() >= MAX_XML_OCCURRENCES {
            return Err(input_limit_error(format!(
                "XML extraction exceeded the maximum of {MAX_XML_OCCURRENCES} occurrences"
            )));
        }
        captures.push(Capture {
            variable: capture.variable,
            path: path.to_owned(),
            source: XmlExtractionSource::ElementContent,
            rendered_text: capture.rendered_text,
        });
    }
    Ok(true)
}

fn match_value(
    template: &str,
    rendered: &str,
    path: &[XmlPathSegment],
    source: &XmlExtractionSource,
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    let segments = parse_value_segments(template, path)?;
    let variables = segments
        .iter()
        .filter_map(|segment| match segment {
            raw_text::RawTextSegment::Variable(variable) => Some(variable),
            raw_text::RawTextSegment::Static(_) => None,
        })
        .cloned()
        .collect::<Vec<_>>();
    let structurally_anchored = matches!(source, XmlExtractionSource::Attribute { .. })
        || segments.iter().any(|segment| {
            matches!(segment, raw_text::RawTextSegment::Static(static_text) if !static_text.is_empty())
        });
    if structurally_anchored && !variables.is_empty() {
        evidence.structural_matches += variables.len();
    }
    let matched = raw_text::match_raw_text(&raw_text::RawTextMatchInput {
        segments: &segments,
        rendered_candidate: rendered,
    })
    .map_err(|error| map_raw_text_error(error, path))?;
    evidence.static_matches += matched.static_matches;
    if let Some(ambiguity) = matched.ambiguity {
        return Err(ExtractError::ambiguous(
            with_span(&ambiguity.message, ambiguity.span),
            None,
        ));
    }
    for capture in matched.captures {
        if captures.len() >= MAX_XML_OCCURRENCES {
            return Err(input_limit_error(format!(
                "XML extraction exceeded the maximum of {MAX_XML_OCCURRENCES} occurrences"
            )));
        }
        debug_assert_eq!(&rendered[capture.span.clone()], capture.rendered_text);
        captures.push(Capture {
            variable: capture.variable,
            path: path.to_owned(),
            source: source.clone(),
            rendered_text: capture.rendered_text,
        });
    }
    Ok(())
}
