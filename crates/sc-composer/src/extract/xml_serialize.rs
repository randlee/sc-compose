//! Canonical XML serialization for block-content matching.

use quick_xml::escape::escape;

use super::{XmlDocument, XmlElement, XmlNode};

pub(super) fn canonical_inner_content(document: &XmlDocument, children: &[XmlNode]) -> String {
    let mut output = String::new();
    for child in children {
        append_canonical_node(document, child, &mut output);
    }
    output
}

fn append_canonical_node(document: &XmlDocument, node: &XmlNode, output: &mut String) {
    match node {
        XmlNode::Text(value) => output.push_str(&escape(value)),
        XmlNode::Element(element_id) => {
            let element = document.element(*element_id);
            output.push('<');
            output.push_str(&element.name);
            append_attributes(element, output);
            if element.children.is_empty() {
                output.push_str("/>");
                return;
            }
            output.push('>');
            for child in &element.children {
                append_canonical_node(document, child, output);
            }
            output.push_str("</");
            output.push_str(&element.name);
            output.push('>');
        }
    }
}

fn append_attributes(element: &XmlElement, output: &mut String) {
    for (name, value) in &element.attributes {
        output.push(' ');
        output.push_str(name);
        output.push_str("=\"");
        output.push_str(&escape(value));
        output.push('"');
    }
}
