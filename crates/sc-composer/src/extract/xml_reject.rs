//! XML template and document rejection rules.

use super::{ExtractError, XmlDocument, XmlElement, XmlNode};

pub(super) fn reject_unsupported_template_syntax(template: &str) -> Result<(), ExtractError> {
    if template.contains("{%") || template.contains("{#") {
        return Err(ExtractError::format_error(
            super::DiagnosticCode::ErrExtractXmlControlFlowUnsupported,
            super::ExtractionDiagnosticKind::Unsupported,
            "XML block extraction does not support Jinja statements or comments",
            super::RecoveryHintKind::UnsupportedConstruct {
                description: "use a known XML template with scalar placeholders".to_owned(),
            },
        ));
    }
    if template.contains("{{{") || template.contains("}}}") {
        return Err(ExtractError::format_error(
            super::DiagnosticCode::ErrExtractTemplateUnsupported,
            super::ExtractionDiagnosticKind::Unsupported,
            "XML extraction supports only double-brace scalar expressions",
            super::RecoveryHintKind::UnsupportedConstruct {
                description: "use double-brace scalar placeholders".to_owned(),
            },
        ));
    }
    Ok(())
}

pub(super) fn reject_dynamic_element_syntax(template: &str) -> Result<(), ExtractError> {
    if template.contains("<{{") || template.contains("</{{") {
        return Err(ExtractError::format_error(
            super::DiagnosticCode::ErrExtractXmlDynamicElementName,
            super::ExtractionDiagnosticKind::Unsupported,
            "dynamic XML element names are outside the reversible extraction subset",
            super::RecoveryHintKind::UnsupportedConstruct {
                description: "use static XML element names".to_owned(),
            },
        ));
    }
    Ok(())
}

pub(super) fn reject_dynamic_element_names(document: &XmlDocument) -> Result<(), ExtractError> {
    fn visit(element: &XmlElement) -> bool {
        element.name.contains('{')
            || element.name.contains('}')
            || element.children.iter().any(|child| match child {
                XmlNode::Element(child) => visit(child),
                XmlNode::Text(_) => false,
            })
    }

    if visit(&document.root) {
        Err(ExtractError::format_error(
            super::DiagnosticCode::ErrExtractXmlDynamicElementName,
            super::ExtractionDiagnosticKind::Unsupported,
            "dynamic XML element names are outside the reversible extraction subset",
            super::RecoveryHintKind::UnsupportedConstruct {
                description: "use static XML element names".to_owned(),
            },
        ))
    } else {
        Ok(())
    }
}

pub(super) fn reject_namespaces(document: &XmlDocument) -> Result<(), ExtractError> {
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
        Err(ExtractError::format_error(
            super::DiagnosticCode::ErrExtractXmlNamespaceUnsupported,
            super::ExtractionDiagnosticKind::Unsupported,
            "XML namespaces are outside the unambiguous extraction subset",
            super::RecoveryHintKind::UnsupportedConstruct {
                description: "use unqualified XML element and attribute names".to_owned(),
            },
        ))
    } else {
        Ok(())
    }
}
