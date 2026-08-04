//! Deterministic structural matching for the supported XML extraction subset.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::mem;

use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

use crate::diagnostics::DiagnosticCode;
use crate::error::RecoveryHintKind;
use crate::frontmatter::parse_template_document;
use crate::types::VariableName;

use super::xml_prefix::{RemovedPrefix, normalize_rendered};
use super::{
    ExtractError, ExtractRequest, ExtractionDiagnostic, ExtractionDiagnosticKind,
    ExtractionOccurrence, ExtractionReport, raw_text,
};

#[path = "xml_match.rs"]
mod xml_match;
#[path = "xml_reject.rs"]
mod xml_reject;
#[path = "xml_serialize.rs"]
mod xml_serialize;

const MAX_XML_INPUT_BYTES: usize = 1_048_576;
const MAX_XML_NESTING_DEPTH: usize = 64;
const MAX_XML_OCCURRENCES: usize = 10_000;

/// XML element/attribute path evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmlPathSegment {
    /// An XML element and its zero-based ordinal among same-named siblings.
    Element {
        /// Element name.
        name: String,
        /// Zero-based sibling ordinal.
        ordinal: usize,
    },
    /// An XML attribute on the preceding element path.
    Attribute {
        /// Attribute name.
        name: String,
    },
}

/// XML source evidence for a recovered scalar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmlExtractionSource {
    /// The scalar was recovered from an XML attribute.
    Attribute {
        /// Attribute name.
        name: String,
    },
    /// The scalar was recovered from an XML text node.
    TextNode,
    /// The value occupies an element's complete content and may include
    /// canonicalized child markup.
    ElementContent,
}

/// XML occurrence report entry.
pub type XmlExtractionOccurrence = ExtractionOccurrence<XmlPathSegment, XmlExtractionSource>;

/// XML report over the generic extraction contract.
pub type XmlExtractionReport = ExtractionReport<XmlPathSegment, XmlExtractionSource>;

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

fn drop_xml_children(children: &mut Vec<XmlNode>) {
    let mut pending = mem::take(children);
    while let Some(mut node) = pending.pop() {
        if let XmlNode::Element(element) = &mut node {
            pending.append(&mut element.children);
        }
    }
}

impl Drop for XmlElement {
    fn drop(&mut self) {
        drop_xml_children(&mut self.children);
    }
}

impl Drop for XmlNode {
    fn drop(&mut self) {
        if let XmlNode::Element(element) = self {
            drop_xml_children(&mut element.children);
        }
    }
}

impl Drop for XmlDocument {
    fn drop(&mut self) {
        drop_xml_children(&mut self.root.children);
    }
}

#[derive(Clone, Debug)]
struct Capture {
    variable: VariableName,
    path: Vec<XmlPathSegment>,
    source: XmlExtractionSource,
    rendered_text: String,
}

#[derive(Default)]
struct Evidence {
    structural_matches: usize,
    expected_structural: usize,
    static_matches: usize,
    expected_static: usize,
}

/// Extract values from a known XML template and rendered XML document.
pub(crate) fn extract_xml(
    request: &ExtractRequest<'_>,
) -> Result<XmlExtractionReport, ExtractError> {
    validate_input_size(request.template, "template")?;
    validate_input_size(request.rendered, "rendered XML")?;
    let parsed_template = parse_template_document(request.template).map_err(|error| {
        ExtractError::unsupported_with_source(
            format!("template frontmatter is not supported: {error}"),
            error,
        )
    })?;
    let template_source = parsed_template.body();
    let normalized = normalize_rendered(request.rendered)?;
    xml_reject::reject_unsupported_template_syntax(template_source)?;
    xml_reject::reject_dynamic_element_syntax(template_source)?;
    let template = parse_xml(template_source)?;
    let rendered = parse_xml(&normalized.source)?;
    xml_reject::reject_dynamic_element_names(&template)?;
    xml_reject::reject_namespaces(&template)?;
    xml_reject::reject_namespaces(&rendered)?;

    if let Some(mut report) = missing_occurrence_report(&template, &rendered, request)? {
        if let Some(prefix) = normalized.removed.as_ref() {
            report
                .diagnostics
                .insert(0, dirty_prefix_diagnostic(prefix));
        }
        return Ok(report);
    }

    let mut captures = Vec::new();
    let mut evidence = Evidence::default();
    collect_expected_evidence(&template.root, &mut evidence)?;
    let root_path = vec![XmlPathSegment::Element {
        name: template.root.name.clone(),
        ordinal: 0,
    }];
    xml_match::match_element(
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
    let mut diagnostics = normalized
        .removed
        .as_ref()
        .map(dirty_prefix_diagnostic)
        .into_iter()
        .collect::<Vec<_>>();

    for capture in selected {
        values
            .entry(capture.variable.clone())
            .or_insert_with(|| capture.rendered_text.clone());
        occurrences.push(XmlExtractionOccurrence {
            variable: capture.variable,
            path: capture.path,
            source: capture.source,
            rendered_text: Some(capture.rendered_text),
        });
    }

    let evidence_total = evidence.expected_structural + evidence.expected_static;
    let matched_evidence = evidence.structural_matches + evidence.static_matches;
    let confidence = if evidence_total == 0 {
        0.0
    } else {
        evidence_confidence(matched_evidence, evidence_total)
    };
    if confidence < 0.75 {
        diagnostics.push(ExtractionDiagnostic::new(
            DiagnosticCode::WarnExtractLowConfidence,
            ExtractionDiagnosticKind::NotObserved,
            "insufficient structural or static evidence for a high-confidence extraction",
            None,
        ));
    }

    XmlExtractionReport::new(values, occurrences, confidence, diagnostics)
}

fn evidence_confidence(matched: usize, total: usize) -> f64 {
    let matched = u32::try_from(matched).unwrap_or(u32::MAX);
    let total = u32::try_from(total).unwrap_or(u32::MAX);
    f64::from(matched) / f64::from(total)
}

fn dirty_prefix_diagnostic(prefix: &RemovedPrefix) -> ExtractionDiagnostic {
    ExtractionDiagnostic::new(
        DiagnosticCode::WarnExtractDirtyPrefixStripped,
        ExtractionDiagnosticKind::NotObserved,
        format!(
            "stripped rendered XML preamble bytes 0..{} (line {}, column {})",
            prefix.byte_end, prefix.line, prefix.column
        ),
        None,
    )
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
    let mut diagnostics = missing_variables
        .into_iter()
        .map(|variable| {
            ExtractionDiagnostic::new(
                DiagnosticCode::WarnExtractNotObserved,
                ExtractionDiagnosticKind::NotObserved,
                format!("variable occurrence was not observed in rendered XML: {variable}"),
                None,
            )
        })
        .collect::<Vec<_>>();
    diagnostics.push(ExtractionDiagnostic::new(
        DiagnosticCode::WarnExtractLowConfidence,
        ExtractionDiagnosticKind::NotObserved,
        "no structural occurrence was observed for the selected variables",
        None,
    ));
    Ok(Some(XmlExtractionReport::new(
        BTreeMap::new(),
        Vec::new(),
        0.0,
        diagnostics,
    )?))
}

fn collect_expected_evidence(
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

fn parse_xml(source: &str) -> Result<XmlDocument, ExtractError> {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<(String, BTreeMap<String, String>, Vec<XmlNode>)> = Vec::new();
    let mut root = None;
    loop {
        let event = reader.read_event().map_err(|error| {
            malformed_with_source(format!("XML parser rejected input: {error}"), error)
        })?;
        match event {
            Event::Start(element) => {
                validate_parse_depth(stack.len())?;
                let name = decode_name(element.name().as_ref())?;
                let attributes = decode_attributes(&reader, &element)?;
                stack.push((name, attributes, Vec::new()));
            }
            Event::Empty(element) => {
                validate_parse_depth(stack.len())?;
                let name = decode_name(element.name().as_ref())?;
                let attributes = decode_attributes(&reader, &element)?;
                attach_element(
                    &mut stack,
                    &mut root,
                    XmlElement {
                        name,
                        attributes,
                        children: Vec::new(),
                    },
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
                attach_element(
                    &mut stack,
                    &mut root,
                    XmlElement {
                        name,
                        attributes,
                        children,
                    },
                )?;
            }
            Event::Text(text) => {
                attach_text(&mut stack, decode_xml_text(text.as_ref())?)?;
            }
            Event::CData(text) => {
                attach_text(&mut stack, decode_xml_cdata(text.as_ref())?)?;
            }
            Event::GeneralRef(reference) => {
                let name = reference
                    .decode()
                    .map_err(|error| {
                        malformed_with_source(format!("invalid XML entity: {error}"), error)
                    })?
                    .into_owned();
                let value = unescape(&format!("&{name};"))
                    .map_err(|error| {
                        malformed_with_source(format!("invalid XML entity: {error}"), error)
                    })?
                    .into_owned();
                attach_text(&mut stack, value)?;
            }
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) => {
                reject_post_root_content(root.as_ref(), &stack)?;
            }
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

fn decode_xml_text(bytes: &[u8]) -> Result<String, ExtractError> {
    let raw = std::str::from_utf8(bytes)
        .map_err(|error| malformed_with_source(format!("invalid XML text: {error}"), error))?;
    unescape(raw)
        .map(std::borrow::Cow::into_owned)
        .map_err(|error| malformed_with_source(format!("invalid XML text entity: {error}"), error))
}

fn decode_xml_cdata(bytes: &[u8]) -> Result<String, ExtractError> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| malformed_with_source(format!("invalid XML CDATA: {error}"), error))
}

fn reject_post_root_content(
    root: Option<&XmlElement>,
    stack: &[(String, BTreeMap<String, String>, Vec<XmlNode>)],
) -> Result<(), ExtractError> {
    if root.is_some() && stack.is_empty() {
        return Err(malformed(
            "XML content appeared after the root element".to_owned(),
        ));
    }
    Ok(())
}

fn decode_name(bytes: &[u8]) -> Result<String, ExtractError> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| {
            malformed_with_source(format!("XML name is not valid UTF-8: {error}"), error)
        })
}

fn decode_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
) -> Result<BTreeMap<String, String>, ExtractError> {
    let mut attributes = BTreeMap::new();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| {
            malformed_with_source(format!("invalid XML attribute: {error}"), error)
        })?;
        let name = decode_name(attribute.key.as_ref())?;
        if attributes.contains_key(&name) {
            return Err(malformed(format!("duplicate XML attribute: {name}")));
        }
        let value = attribute
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| {
                malformed_with_source(format!("invalid XML attribute value: {error}"), error)
            })?
            .into_owned();
        attributes.insert(name, value);
    }
    Ok(attributes)
}

fn attach_element(
    stack: &mut [(String, BTreeMap<String, String>, Vec<XmlNode>)],
    root: &mut Option<XmlElement>,
    element: XmlElement,
) -> Result<(), ExtractError> {
    if let Some((_, _, children)) = stack.last_mut() {
        children.push(XmlNode::Element(element));
    } else if root.is_some() {
        return Err(malformed(
            "XML input contains more than one root element".to_owned(),
        ));
    } else {
        *root = Some(element);
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

fn is_single_variable(value: &str) -> bool {
    value.trim().starts_with("{{") && value.trim().ends_with("}}")
}

fn parse_value_segments<'a>(
    value: &'a str,
    path: &[XmlPathSegment],
) -> Result<Vec<raw_text::RawTextSegment<'a>>, ExtractError> {
    raw_text::parse_raw_text_segments(value).map_err(|error| map_raw_text_error(error, path))
}

fn map_raw_text_error(error: raw_text::RawTextMatchError, path: &[XmlPathSegment]) -> ExtractError {
    match error.scope() {
        raw_text::RawTextErrorScope::Request => match error {
            raw_text::RawTextMatchError::InvalidTemplate { span, message } => {
                ExtractError::format_error(
                    DiagnosticCode::ErrExtractTemplateUnsupported,
                    ExtractionDiagnosticKind::Unsupported,
                    with_span(&message, span),
                    RecoveryHintKind::UnsupportedConstruct {
                        description: "use supported scalar XML placeholders".to_owned(),
                    },
                )
            }
            raw_text::RawTextMatchError::StaticMismatch { span, message } => {
                ExtractError::format_error(
                    DiagnosticCode::ErrExtractXmlStaticMismatch,
                    ExtractionDiagnosticKind::Unsupported,
                    with_span(&message, span),
                    RecoveryHintKind::InspectInput {
                        description: "align rendered XML static content with the known template"
                            .to_owned(),
                    },
                )
            }
            raw_text::RawTextMatchError::AmbiguousDelimiter { span, message } => {
                ExtractError::ambiguous(with_span(&message, span), None)
            }
        },
        raw_text::RawTextErrorScope::Occurrence => match error {
            raw_text::RawTextMatchError::InvalidTemplate { span, message } => {
                ExtractError::format_error(
                    DiagnosticCode::ErrExtractTemplateUnsupported,
                    ExtractionDiagnosticKind::Unsupported,
                    with_span(&message, span),
                    RecoveryHintKind::UnsupportedConstruct {
                        description: "use supported scalar XML placeholders".to_owned(),
                    },
                )
            }
            raw_text::RawTextMatchError::StaticMismatch { span, message } => {
                ExtractError::format_error(
                    DiagnosticCode::ErrExtractXmlStaticMismatch,
                    ExtractionDiagnosticKind::Unsupported,
                    format!("{} at {}", with_span(&message, span), format_path(path)),
                    RecoveryHintKind::InspectInput {
                        description: "align rendered XML static content with the known template"
                            .to_owned(),
                    },
                )
            }
            raw_text::RawTextMatchError::AmbiguousDelimiter { span, message } => {
                if message.contains("adjacent variable") {
                    ExtractError::ambiguous_delimiter(
                        "adjacent XML variable expressions have no structural delimiter",
                    )
                } else {
                    ExtractError::ambiguous(with_span(&message, span), None)
                }
            }
        },
    }
}

fn with_span(message: &str, span: Option<std::ops::Range<usize>>) -> String {
    span.map_or_else(
        || message.to_owned(),
        |span| format!("{message} (candidate bytes {}..{})", span.start, span.end),
    )
}

fn format_path(path: &[XmlPathSegment]) -> String {
    let mut result = String::from("$");
    for segment in path {
        match segment {
            XmlPathSegment::Element { name, ordinal } => {
                result.push('.');
                result.push_str(name);
                result.push('[');
                result.push_str(&ordinal.to_string());
                result.push(']');
            }
            XmlPathSegment::Attribute { name } => {
                result.push_str(".@");
                result.push_str(name);
            }
        }
    }
    result
}

fn validate_input_size(source: &str, label: &str) -> Result<(), ExtractError> {
    if source.len() > MAX_XML_INPUT_BYTES {
        return Err(input_limit_error(format!(
            "XML {label} input is {} bytes; maximum is {MAX_XML_INPUT_BYTES} bytes",
            source.len()
        )));
    }
    Ok(())
}

fn validate_parse_depth(depth: usize) -> Result<(), ExtractError> {
    if depth > MAX_XML_NESTING_DEPTH {
        return Err(input_limit_error(format!(
            "XML nesting depth exceeds the maximum of {MAX_XML_NESTING_DEPTH}"
        )));
    }
    Ok(())
}

fn input_limit_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractInputLimit,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "reduce XML input size, nesting depth, or occurrence count".to_owned(),
        },
    )
}

fn malformed(message: String) -> ExtractError {
    ExtractError::malformed(message)
}

fn malformed_with_source<E>(message: String, source: E) -> ExtractError
where
    E: StdError + Send + Sync + 'static,
{
    ExtractError::malformed_with_source(message, source)
}
