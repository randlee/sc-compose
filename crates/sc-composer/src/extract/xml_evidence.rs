//! Private XML occurrence and evidence collection ownership.

use std::collections::BTreeMap;

use crate::types::VariableName;

use super::{
    ExtractError, XmlDocument, XmlElementId, XmlNode, XmlPathSegment, parse_value_segments,
    raw_text,
};

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
    document: &XmlDocument,
    element_id: XmlElementId,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    let element = document.element(element_id);
    evidence.expected_structural += 1;
    for value in element.attributes.values() {
        collect_expected_value_evidence(value, &[], evidence)?;
    }
    for child in &element.children {
        match child {
            XmlNode::Element(child) => collect_expected_evidence(document, *child, evidence)?,
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
    document: &XmlDocument,
    template_id: XmlElementId,
    path: &[XmlPathSegment],
    occurrences: &mut Vec<TemplateOccurrence>,
) -> Result<(), ExtractError> {
    let template = document.element(template_id);
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
            XmlNode::Element(element_id) => {
                let element = document.element(*element_id);
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
                collect_template_occurrences(document, *element_id, &child_path, occurrences)?;
            }
        }
    }
    Ok(())
}

pub(super) fn path_exists(
    document: &XmlDocument,
    root_id: XmlElementId,
    path: &[XmlPathSegment],
) -> bool {
    let Some(XmlPathSegment::Element { name, ordinal }) = path.first() else {
        return false;
    };
    if document.element(root_id).name != *name || *ordinal != 0 {
        return false;
    }
    let mut current_id = root_id;
    for segment in path.iter().skip(1) {
        match segment {
            XmlPathSegment::Element { name, ordinal } => {
                let mut seen = 0;
                let Some(element_id) =
                    document
                        .element(current_id)
                        .children
                        .iter()
                        .find_map(|child| match child {
                            XmlNode::Element(element_id)
                                if document.element(*element_id).name == *name =>
                            {
                                let found = (seen == *ordinal).then_some(*element_id);
                                seen += 1;
                                found
                            }
                            _ => None,
                        })
                else {
                    return false;
                };
                current_id = element_id;
            }
            XmlPathSegment::Attribute { name } => {
                return document.element(current_id).attributes.contains_key(name);
            }
        }
    }
    true
}
