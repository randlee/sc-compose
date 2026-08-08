//! Private XML parser and tree-model ownership.

use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use std::collections::BTreeMap;

use super::{ExtractError, input_limit_error};

const MAX_XML_NESTING_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct XmlElement {
    pub(super) name: String,
    pub(super) attributes: BTreeMap<String, String>,
    pub(super) children: Vec<XmlNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct XmlElementId(usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum XmlNode {
    Element(XmlElementId),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct XmlDocument {
    pub(super) root: XmlElementId,
    elements: Vec<XmlElement>,
}

impl XmlDocument {
    pub(super) fn element(&self, id: XmlElementId) -> &XmlElement {
        &self.elements[id.0]
    }
}

pub(super) fn parse_xml(source: &str) -> Result<XmlDocument, ExtractError> {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<(String, BTreeMap<String, String>, Vec<XmlNode>)> = Vec::new();
    let mut root = None;
    let mut elements = Vec::new();
    loop {
        let event = reader.read_event().map_err(|error| {
            super::malformed_with_source(format!("XML parser rejected input: {error}"), error)
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
                    &mut elements,
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
                    .ok_or_else(|| super::malformed("unexpected XML closing tag".to_owned()))?;
                let end_name = decode_name(end.name().as_ref())?;
                if name != end_name {
                    return Err(super::malformed(format!(
                        "XML closing tag does not match opening tag: {name} != {end_name}"
                    )));
                }
                attach_element(
                    &mut stack,
                    &mut root,
                    &mut elements,
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
                        super::malformed_with_source(format!("invalid XML entity: {error}"), error)
                    })?
                    .into_owned();
                let value = unescape(&format!("&{name};"))
                    .map_err(|error| {
                        super::malformed_with_source(format!("invalid XML entity: {error}"), error)
                    })?
                    .into_owned();
                attach_text(&mut stack, value)?;
            }
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) => {
                reject_post_root_content(root, &stack)?;
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
        return Err(super::malformed(
            "XML input ended before all elements closed".to_owned(),
        ));
    }
    let root = root.ok_or_else(|| super::malformed("XML input has no root element".to_owned()))?;
    Ok(XmlDocument { root, elements })
}

fn decode_xml_text(bytes: &[u8]) -> Result<String, ExtractError> {
    let raw = std::str::from_utf8(bytes).map_err(|error| {
        super::malformed_with_source(format!("invalid XML text: {error}"), error)
    })?;
    unescape(raw)
        .map(std::borrow::Cow::into_owned)
        .map_err(|error| {
            super::malformed_with_source(format!("invalid XML text entity: {error}"), error)
        })
}

fn decode_xml_cdata(bytes: &[u8]) -> Result<String, ExtractError> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| super::malformed_with_source(format!("invalid XML CDATA: {error}"), error))
}

fn reject_post_root_content(
    root: Option<XmlElementId>,
    stack: &[(String, BTreeMap<String, String>, Vec<XmlNode>)],
) -> Result<(), ExtractError> {
    if root.is_some() && stack.is_empty() {
        return Err(super::malformed(
            "XML content appeared after the root element".to_owned(),
        ));
    }
    Ok(())
}

fn decode_name(bytes: &[u8]) -> Result<String, ExtractError> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| {
            super::malformed_with_source(format!("XML name is not valid UTF-8: {error}"), error)
        })
}

fn decode_attributes(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
) -> Result<BTreeMap<String, String>, ExtractError> {
    let mut attributes = BTreeMap::new();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| {
            super::malformed_with_source(format!("invalid XML attribute: {error}"), error)
        })?;
        let name = decode_name(attribute.key.as_ref())?;
        if attributes.contains_key(&name) {
            return Err(super::malformed(format!("duplicate XML attribute: {name}")));
        }
        let value = attribute
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| {
                super::malformed_with_source(format!("invalid XML attribute value: {error}"), error)
            })?
            .into_owned();
        attributes.insert(name, value);
    }
    Ok(attributes)
}

fn attach_element(
    stack: &mut [(String, BTreeMap<String, String>, Vec<XmlNode>)],
    root: &mut Option<XmlElementId>,
    elements: &mut Vec<XmlElement>,
    element: XmlElement,
) -> Result<(), ExtractError> {
    let id = XmlElementId(elements.len());
    elements.push(element);
    if let Some((_, _, children)) = stack.last_mut() {
        children.push(XmlNode::Element(id));
    } else if root.is_some() {
        return Err(super::malformed(
            "XML input contains more than one root element".to_owned(),
        ));
    } else {
        *root = Some(id);
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
        return Err(super::malformed(
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

fn validate_parse_depth(depth: usize) -> Result<(), ExtractError> {
    if depth > MAX_XML_NESTING_DEPTH {
        return Err(input_limit_error(format!(
            "XML nesting depth exceeds the maximum of {MAX_XML_NESTING_DEPTH}"
        )));
    }
    Ok(())
}
