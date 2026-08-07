//! Private XML occurrence and evidence collection ownership.

use std::collections::BTreeMap;

use crate::types::VariableName;

use super::{ExtractError, XmlElement, XmlNode, XmlPathSegment, parse_value_segments, raw_text};

pub(super) struct Capture {
    pub(super) variable: VariableName,
    pub(super) path: Vec<XmlPathSegment>,
    pub(super) source: super::XmlExtractionSource,
    pub(super) rendered_text: String,
}

#[derive(Default)]
pub(super) struct Evidence {
    pub(super) structural_matches: usize,
    pub(super) expected_structural: usize,
    pub(super) static_matches: usize,
    pub(super) expected_static: usize,
}

pub(super) fn collect_expected_evidence(
    element: &XmlElement,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    evidence.expected_structural += 1;
    for value in element.attributes.values() {
        collect_expected_value_evidence(value, &[], evidence)?;
    }
    for child in &element.children {
        match child {
            XmlNode::Element(child) => collect_expected_evidence(child, evidence)?,
            XmlNode::Text(value) => collect_expected_value_evidence(value, &[], evidence)?,
        }
    }
    Ok(())
}

fn collect_expected_value_evidence(
    value: &str,
    path: &[XmlPathSegment],
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    for segment in parse_value_segments(value, path)? {
        match segment {
            raw_text::RawTextSegment::Static(static_text) => {
                evidence.expected_static += usize::from(!static_text.is_empty());
            }
            raw_text::RawTextSegment::Variable(_) => evidence.expected_structural += 1,
        }
    }
    Ok(())
}

pub(super) struct TemplateOccurrence {
    pub(super) variable: VariableName,
    pub(super) path: Vec<XmlPathSegment>,
}

pub(super) fn collect_template_occurrences(
    template: &XmlElement,
    path: &[XmlPathSegment],
    occurrences: &mut Vec<TemplateOccurrence>,
) -> Result<(), ExtractError> {
    for (name, value) in &template.attributes {
        let mut attribute_path = path.to_owned();
        attribute_path.push(XmlPathSegment::Attribute { name: name.clone() });
        for segment in parse_value_segments(value, &attribute_path)? {
            if let raw_text::RawTextSegment::Variable(variable) = segment {
                occurrences.push(TemplateOccurrence {
                    variable,
                    path: attribute_path.clone(),
                });
            }
        }
    }
    let mut element_ordinals = BTreeMap::<String, usize>::new();
    for child in &template.children {
        match child {
            XmlNode::Text(value) => {
                for segment in parse_value_segments(value, path)? {
                    if let raw_text::RawTextSegment::Variable(variable) = segment {
                        occurrences.push(TemplateOccurrence {
                            variable,
                            path: path.to_owned(),
                        });
                    }
                }
            }
            XmlNode::Element(element) => {
                let ordinal = element_ordinals.entry(element.name.clone()).or_default();
                let child_path = path
                    .iter()
                    .cloned()
                    .chain([XmlPathSegment::Element {
                        name: element.name.clone(),
                        ordinal: *ordinal,
                    }])
                    .collect::<Vec<_>>();
                *ordinal += 1;
                collect_template_occurrences(element, &child_path, occurrences)?;
            }
        }
    }
    Ok(())
}

pub(super) fn path_exists(root: &XmlElement, path: &[XmlPathSegment]) -> bool {
    let Some(XmlPathSegment::Element { name, ordinal }) = path.first() else {
        return false;
    };
    if root.name != *name || *ordinal != 0 {
        return false;
    }
    let mut current = root;
    for segment in path.iter().skip(1) {
        match segment {
            XmlPathSegment::Element { name, ordinal } => {
                let mut seen = 0;
                let Some(element) = current.children.iter().find_map(|child| match child {
                    XmlNode::Element(element) if element.name == *name => {
                        let found = (seen == *ordinal).then_some(element);
                        seen += 1;
                        found
                    }
                    _ => None,
                }) else {
                    return false;
                };
                current = element;
            }
            XmlPathSegment::Attribute { name } => return current.attributes.contains_key(name),
        }
    }
    true
}
