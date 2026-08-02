//! Deterministic structural matching for the supported XML extraction subset.

use std::collections::{BTreeMap, BTreeSet};

use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::Event;

use crate::diagnostics::DiagnosticCode;
use crate::error::{RecoveryHint, RecoveryHintKind};
use crate::frontmatter::parse_template_document;
use crate::types::VariableName;

use super::{
    ExtractError, ExtractRequest, ExtractionDiagnostic, ExtractionDiagnosticKind,
    ExtractionOccurrence, ExtractionReport, OccurrenceIndex,
};

/// XML-specific source evidence for a recovered scalar.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExtractionSource {
    /// The scalar was recovered from an XML attribute.
    Attribute {
        /// Attribute name as it appears in the XML document.
        name: String,
    },
    /// The scalar was recovered from an XML text node.
    TextNode,
}

/// XML element/attribute path segment used as occurrence provenance.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum XmlPathSegment {
    /// An element and its zero-based ordinal among same-named siblings.
    Element {
        /// Element name.
        name: String,
        /// Zero-based ordinal among same-named sibling elements.
        ordinal: usize,
    },
    /// An attribute on the preceding element path segment.
    Attribute {
        /// Attribute name.
        name: String,
    },
}

/// XML occurrence alias over the generic G.1 extraction contract.
pub type XmlExtractionOccurrence = ExtractionOccurrence<XmlPathSegment, ExtractionSource>;

/// XML report alias over the generic G.1 extraction contract.
pub type XmlExtractionReport = ExtractionReport<XmlPathSegment, ExtractionSource>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct XmlElement {
    name: String,
    attributes: BTreeMap<String, String>,
    children: Vec<XmlNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum XmlNode {
    Element(XmlElement),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct XmlDocument {
    root: XmlElement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TemplateSegment {
    Static(String),
    Variable(VariableName),
}

#[derive(Clone, Debug)]
struct Capture {
    variable: VariableName,
    path: Vec<XmlPathSegment>,
    source: ExtractionSource,
    rendered_text: String,
}

#[derive(Default)]
struct Evidence {
    structural_matches: usize,
    static_matches: usize,
}

/// Extract values from a known XML template and rendered XML document.
pub(crate) fn extract_xml(
    request: &ExtractRequest<'_>,
) -> Result<XmlExtractionReport, ExtractError> {
    let parsed_template = parse_template_document(request.template).map_err(|error| {
        ExtractError::unsupported(format!("template frontmatter is not supported: {error}"))
    })?;
    let template_source = parsed_template.body();
    reject_unsupported_template_syntax(template_source)?;
    let template = parse_xml(template_source)?;
    let rendered = parse_xml(request.rendered)?;
    reject_namespaces(&template)?;
    reject_namespaces(&rendered)?;

    if let Some(report) = missing_occurrence_report(&template, &rendered, request)? {
        return Ok(report);
    }

    let mut captures = Vec::new();
    let mut evidence = Evidence::default();
    let root_path = vec![XmlPathSegment::Element {
        name: template.root.name.clone(),
        ordinal: 0,
    }];
    match_element(
        &template.root,
        &rendered.root,
        &root_path,
        &mut captures,
        &mut evidence,
    )?;

    let selected = captures
        .into_iter()
        .filter(|capture| selected_variable(&capture.variable, request))
        .collect::<Vec<_>>();

    let mut values = BTreeMap::new();
    let mut occurrences = Vec::new();
    let mut diagnostics = Vec::new();
    let mut by_variable: BTreeMap<VariableName, Vec<(usize, Vec<XmlPathSegment>, String)>> =
        BTreeMap::new();

    for capture in selected {
        let index = occurrences.len();
        by_variable
            .entry(capture.variable.clone())
            .or_default()
            .push((index, capture.path.clone(), capture.rendered_text.clone()));
        occurrences.push(XmlExtractionOccurrence {
            variable: capture.variable,
            path: capture.path,
            source: capture.source,
            rendered_text: Some(capture.rendered_text),
        });
    }

    for (variable, entries) in by_variable {
        let first = &entries[0];
        let has_conflict = entries
            .iter()
            .skip(1)
            .any(|entry| entry.1 != first.1 || entry.2 != first.2);
        if has_conflict {
            diagnostics.push(ExtractionDiagnostic::new(
                DiagnosticCode::ErrExtractAmbiguous,
                ExtractionDiagnosticKind::Ambiguous,
                format!("variable has multiple conflicting XML occurrences: {variable}"),
                entries.get(1).map(|entry| OccurrenceIndex(entry.0)),
            ));
        } else {
            values.insert(variable, first.2.clone());
        }
    }

    let evidence_total = evidence.structural_matches + evidence.static_matches;
    let confidence = if evidence_total == 0 { 0.0 } else { 1.0 };
    if confidence < 0.75 {
        diagnostics.push(ExtractionDiagnostic::new(
            DiagnosticCode::WarnExtractLowConfidence,
            ExtractionDiagnosticKind::NotObserved,
            "insufficient structural or static evidence for a high-confidence extraction",
            None,
        ));
    }

    Ok(XmlExtractionReport {
        values,
        occurrences,
        confidence,
        diagnostics,
    })
}

fn selected_variable(variable: &VariableName, request: &ExtractRequest<'_>) -> bool {
    (request.include.is_empty() || request.include.contains(variable))
        && !request.exclude.contains(variable)
}

fn missing_occurrence_report(
    template: &XmlDocument,
    rendered: &XmlDocument,
    request: &ExtractRequest<'_>,
) -> Result<Option<XmlExtractionReport>, ExtractError> {
    let mut template_occurrences = Vec::new();
    collect_template_occurrences(
        &template.root,
        &[XmlPathSegment::Element {
            name: template.root.name.clone(),
            ordinal: 0,
        }],
        &mut template_occurrences,
    )?;
    let missing_variables = template_occurrences
        .iter()
        .filter(|occurrence| selected_variable(&occurrence.variable, request))
        .filter(|occurrence| !path_exists(&rendered.root, &occurrence.path))
        .map(|occurrence| occurrence.variable.clone())
        .collect::<BTreeSet<_>>();
    if missing_variables.is_empty() {
        return Ok(None);
    }
    Ok(Some(XmlExtractionReport {
        values: BTreeMap::new(),
        occurrences: Vec::new(),
        confidence: 0.0,
        diagnostics: missing_variables
            .into_iter()
            .map(|variable| {
                ExtractionDiagnostic::new(
                    DiagnosticCode::WarnExtractNotObserved,
                    ExtractionDiagnosticKind::NotObserved,
                    format!("variable occurrence was not observed in rendered XML: {variable}"),
                    None,
                )
            })
            .collect(),
    }))
}

struct TemplateOccurrence {
    variable: VariableName,
    path: Vec<XmlPathSegment>,
}

fn collect_template_occurrences(
    template: &XmlElement,
    path: &[XmlPathSegment],
    occurrences: &mut Vec<TemplateOccurrence>,
) -> Result<(), ExtractError> {
    for (name, value) in &template.attributes {
        let mut attribute_path = path.to_owned();
        attribute_path.push(XmlPathSegment::Attribute { name: name.clone() });
        for segment in parse_value_segments(value)? {
            if let TemplateSegment::Variable(variable) = segment {
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
                for segment in parse_value_segments(value)? {
                    if let TemplateSegment::Variable(variable) = segment {
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

fn path_exists(root: &XmlElement, path: &[XmlPathSegment]) -> bool {
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

fn reject_unsupported_template_syntax(template: &str) -> Result<(), ExtractError> {
    if template.contains("{%") || template.contains("{#") {
        return Err(ExtractError::unsupported(
            "XML extraction does not support Jinja statements or comments",
        ));
    }
    if template.contains("{{{") || template.contains("}}}") {
        return Err(ExtractError::unsupported(
            "XML extraction supports only double-brace scalar expressions",
        ));
    }
    Ok(())
}

fn parse_xml(source: &str) -> Result<XmlDocument, ExtractError> {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<(String, BTreeMap<String, String>, Vec<XmlNode>)> = Vec::new();
    let mut root = None;

    loop {
        let event = reader
            .read_event()
            .map_err(|error| malformed(format!("XML parser rejected input: {error}")))?;
        match event {
            Event::Start(element) => {
                let name = decode_name(element.name().as_ref())?;
                let attributes = decode_attributes(&reader, &element)?;
                stack.push((name, attributes, Vec::new()));
            }
            Event::Empty(element) => {
                let name = decode_name(element.name().as_ref())?;
                let attributes = decode_attributes(&reader, &element)?;
                attach_node(
                    &mut stack,
                    &mut root,
                    XmlNode::Element(XmlElement {
                        name,
                        attributes,
                        children: Vec::new(),
                    }),
                )?;
            }
            Event::End(end) => {
                let (name, attributes, children) = stack
                    .pop()
                    .ok_or_else(|| malformed("unexpected XML closing tag".to_owned()))?;
                let end_name = decode_name(end.name().as_ref())?;
                if name != end_name {
                    return Err(malformed(format!(
                        "XML closing tag does not match opening tag: {name} != {end_name}"
                    )));
                }
                attach_node(
                    &mut stack,
                    &mut root,
                    XmlNode::Element(XmlElement {
                        name,
                        attributes,
                        children,
                    }),
                )?;
            }
            Event::Text(text) => {
                let raw = std::str::from_utf8(text.as_ref())
                    .map_err(|error| malformed(format!("invalid XML text: {error}")))?;
                let value = unescape(raw)
                    .map_err(|error| malformed(format!("invalid XML text entity: {error}")))?
                    .into_owned();
                attach_text(&mut stack, value)?;
            }
            Event::CData(text) => {
                let value = std::str::from_utf8(text.as_ref())
                    .map_err(|error| malformed(format!("invalid XML CDATA: {error}")))?
                    .to_owned();
                attach_text(&mut stack, value)?;
            }
            Event::GeneralRef(reference) => {
                let name = reference
                    .decode()
                    .map_err(|error| malformed(format!("invalid XML entity: {error}")))?
                    .into_owned();
                let value = unescape(&format!("&{name};"))
                    .map_err(|error| malformed(format!("invalid XML entity: {error}")))?
                    .into_owned();
                attach_text(&mut stack, value)?;
            }
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) => {}
            Event::DocType(_) => {
                return Err(ExtractError::unsupported(
                    "XML DTD declarations are outside the reversible extraction subset",
                ));
            }
            Event::Eof => break,
        }
    }

    if !stack.is_empty() {
        return Err(malformed(
            "XML input ended before all elements closed".to_owned(),
        ));
    }
    let root = root.ok_or_else(|| malformed("XML input has no root element".to_owned()))?;
    Ok(XmlDocument { root })
}

fn decode_name(bytes: &[u8]) -> Result<String, ExtractError> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| malformed(format!("XML name is not valid UTF-8: {error}")))
}

fn decode_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
) -> Result<BTreeMap<String, String>, ExtractError> {
    let mut attributes = BTreeMap::new();
    for attribute in element.attributes() {
        let attribute =
            attribute.map_err(|error| malformed(format!("invalid XML attribute: {error}")))?;
        let name = decode_name(attribute.key.as_ref())?;
        if attributes.contains_key(&name) {
            return Err(malformed(format!("duplicate XML attribute: {name}")));
        }
        let value = attribute
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| malformed(format!("invalid XML attribute value: {error}")))?
            .into_owned();
        attributes.insert(name, value);
    }
    Ok(attributes)
}

fn attach_node(
    stack: &mut [(String, BTreeMap<String, String>, Vec<XmlNode>)],
    root: &mut Option<XmlElement>,
    node: XmlNode,
) -> Result<(), ExtractError> {
    if let Some((_, _, children)) = stack.last_mut() {
        children.push(node);
    } else if root.is_some() {
        return Err(malformed(
            "XML input contains more than one root element".to_owned(),
        ));
    } else if let XmlNode::Element(element) = node {
        *root = Some(element);
    } else {
        return Err(malformed(
            "XML text appeared outside the root element".to_owned(),
        ));
    }
    Ok(())
}

fn attach_text(
    stack: &mut [(String, BTreeMap<String, String>, Vec<XmlNode>)],
    value: String,
) -> Result<(), ExtractError> {
    if stack.is_empty() {
        if value.trim().is_empty() {
            return Ok(());
        }
        return Err(malformed(
            "XML text appeared outside the root element".to_owned(),
        ));
    }
    if value.is_empty() {
        return Ok(());
    }
    if let Some(XmlNode::Text(previous)) = stack.last_mut().and_then(|entry| entry.2.last_mut()) {
        previous.push_str(&value);
    } else if let Some((_, _, children)) = stack.last_mut() {
        children.push(XmlNode::Text(value));
    }
    Ok(())
}

fn reject_namespaces(document: &XmlDocument) -> Result<(), ExtractError> {
    fn visit(element: &XmlElement) -> bool {
        element.name.contains(':')
            || element
                .attributes
                .keys()
                .any(|name| name == "xmlns" || name.starts_with("xmlns:"))
            || element.children.iter().any(|child| match child {
                XmlNode::Element(child) => visit(child),
                XmlNode::Text(_) => false,
            })
    }

    if visit(&document.root) {
        Err(ExtractError::unsupported(
            "XML namespaces are outside the unambiguous extraction subset",
        ))
    } else {
        Ok(())
    }
}

fn match_element(
    template: &XmlElement,
    rendered: &XmlElement,
    path: &[XmlPathSegment],
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    if template.name != rendered.name {
        return Err(ExtractError::unsupported(format!(
            "rendered XML element does not match template structure: expected {}, found {}",
            template.name, rendered.name
        )));
    }
    evidence.structural_matches += 1;
    match_attributes(template, rendered, path, captures, evidence)?;
    match_children(template, rendered, path, captures, evidence)
}

fn match_attributes(
    template: &XmlElement,
    rendered: &XmlElement,
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
        return Err(ExtractError::unsupported(format!(
            "rendered XML attributes do not match template element {}",
            template.name
        )));
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
            &ExtractionSource::Attribute { name: name.clone() },
            captures,
            evidence,
        )?;
    }
    Ok(())
}

fn match_children(
    template: &XmlElement,
    rendered: &XmlElement,
    path: &[XmlPathSegment],
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    let template_children = &template.children;
    let rendered_children = &rendered.children;
    if template_children.len() == 1
        && rendered_children.is_empty()
        && matches!(&template_children[0], XmlNode::Text(value) if is_single_variable(value))
    {
        if let XmlNode::Text(value) = &template_children[0] {
            let segments = parse_value_segments(value)?;
            if let [TemplateSegment::Variable(variable)] = segments.as_slice() {
                captures.push(Capture {
                    variable: variable.clone(),
                    path: path.to_owned(),
                    source: ExtractionSource::TextNode,
                    rendered_text: String::new(),
                });
            }
        }
        return Ok(());
    }

    if template_children.len() != rendered_children.len() {
        return Err(ExtractError::unsupported(format!(
            "rendered XML child structure does not match template element {}",
            template.name
        )));
    }

    let mut element_ordinals = BTreeMap::<String, usize>::new();
    for (template_child, rendered_child) in template_children.iter().zip(rendered_children) {
        match (template_child, rendered_child) {
            (XmlNode::Text(template_text), XmlNode::Text(rendered_text)) => {
                match_value(
                    template_text,
                    rendered_text,
                    path,
                    &ExtractionSource::TextNode,
                    captures,
                    evidence,
                )?;
            }
            (XmlNode::Element(template_element), XmlNode::Element(rendered_element)) => {
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
                    template_element,
                    rendered_element,
                    &child_path,
                    captures,
                    evidence,
                )?;
            }
            _ => {
                return Err(ExtractError::unsupported(format!(
                    "rendered XML node structure does not match template element {}",
                    template.name
                )));
            }
        }
    }
    Ok(())
}

fn is_single_variable(value: &str) -> bool {
    value.trim().starts_with("{{") && value.trim().ends_with("}}")
}

fn match_value(
    template: &str,
    rendered: &str,
    path: &[XmlPathSegment],
    source: &ExtractionSource,
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    let segments = parse_value_segments(template)?;
    let variables = segments
        .iter()
        .filter_map(|segment| match segment {
            TemplateSegment::Variable(variable) => Some(variable),
            TemplateSegment::Static(_) => None,
        })
        .cloned()
        .collect::<Vec<_>>();
    if variables.is_empty() {
        if template != rendered {
            return Err(ExtractError::unsupported(
                "rendered XML static content does not match the known template",
            ));
        }
        evidence.static_matches += usize::from(!template.is_empty());
        return Ok(());
    }

    let mut cursor = 0;
    let mut captures_for_value = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        match segment {
            TemplateSegment::Static(static_text) => {
                if !rendered[cursor..].starts_with(static_text) {
                    return Err(ExtractError::unsupported(
                        "rendered XML static content does not match the known template",
                    ));
                }
                cursor += static_text.len();
                evidence.static_matches += usize::from(!static_text.is_empty());
            }
            TemplateSegment::Variable(variable) => {
                let next_static = segments.iter().skip(index + 1).find_map(|next| match next {
                    TemplateSegment::Static(value) if !value.is_empty() => Some(value),
                    _ => None,
                });
                let end = if let Some(next_static) = next_static {
                    let remainder = &rendered[cursor..];
                    let Some(offset) = remainder.find(next_static) else {
                        return Err(ExtractError::unsupported(
                            "rendered XML is missing static suffix around a variable",
                        ));
                    };
                    if remainder[offset + next_static.len()..].contains(next_static) {
                        return Err(ExtractError::ambiguous(
                            "static XML suffix occurs multiple times around a variable",
                            None,
                        ));
                    }
                    cursor + offset
                } else {
                    rendered.len()
                };
                captures_for_value.push((variable.clone(), rendered[cursor..end].to_owned()));
                cursor = end;
            }
        }
    }
    if cursor != rendered.len() {
        return Err(ExtractError::unsupported(
            "rendered XML has trailing content outside the known template",
        ));
    }
    for (variable, rendered_text) in captures_for_value {
        captures.push(Capture {
            variable,
            path: path.to_owned(),
            source: source.clone(),
            rendered_text,
        });
    }
    Ok(())
}

fn parse_value_segments(value: &str) -> Result<Vec<TemplateSegment>, ExtractError> {
    let mut segments = Vec::new();
    let mut cursor = 0;
    while let Some(relative_open) = value[cursor..].find("{{") {
        let open = cursor + relative_open;
        if value[open..].starts_with("{{{") {
            return Err(ExtractError::unsupported(
                "XML extraction supports only double-brace scalar expressions",
            ));
        }
        if open > cursor {
            segments.push(TemplateSegment::Static(value[cursor..open].to_owned()));
        }
        let expression_start = open + 2;
        let Some(relative_close) = value[expression_start..].find("}}") else {
            return Err(ExtractError::unsupported(
                "XML template contains an unterminated expression",
            ));
        };
        let close = expression_start + relative_close;
        if value[close..].starts_with("}}}") {
            return Err(ExtractError::unsupported(
                "XML extraction supports only double-brace scalar expressions",
            ));
        }
        let expression = value[expression_start..close].trim();
        let variable = VariableName::new(expression).map_err(|error| {
            ExtractError::unsupported(format!(
                "unsupported XML template expression: {{{{ {expression} }}}}: {error}"
            ))
        })?;
        segments.push(TemplateSegment::Variable(variable));
        cursor = close + 2;
    }
    if cursor < value.len() {
        segments.push(TemplateSegment::Static(value[cursor..].to_owned()));
    }
    if segments.is_empty() {
        segments.push(TemplateSegment::Static(value.to_owned()));
    }
    if segments.windows(2).any(|pair| {
        matches!(
            (&pair[0], &pair[1]),
            (TemplateSegment::Variable(_), TemplateSegment::Variable(_))
        )
    }) {
        return Err(ExtractError::ambiguous(
            "adjacent XML variable expressions have no structural delimiter",
            None,
        ));
    }
    Ok(segments)
}

fn malformed(message: String) -> ExtractError {
    ExtractError::MalformedXml {
        diagnostic: ExtractionDiagnostic::new(
            DiagnosticCode::ErrExtractMalformed,
            ExtractionDiagnosticKind::Malformed,
            message,
            None,
        ),
        recovery_hints: vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "rendered XML input".to_owned(),
        })],
    }
}
