//! Pipeline typestates and output assembly helpers.

use std::marker::PhantomData;

/// Zero-cost typestate wrapper for pipeline documents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document<State> {
    body: String,
    state: PhantomData<State>,
}

impl<State> Document<State> {
    /// Create a typestated document body.
    #[must_use]
    pub fn new(body: String) -> Self {
        Self {
            body,
            state: PhantomData,
        }
    }

    /// Borrow the document body.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Parsed document marker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parsed;

/// Expanded document marker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Expanded;

/// Validated document marker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Validated;

/// Rendered document marker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rendered;

impl Document<Parsed> {
    /// Transition a parsed document into the expanded state.
    #[must_use]
    pub fn into_expanded(self) -> Document<Expanded> {
        Document::new(self.body)
    }
}

impl Document<Expanded> {
    /// Transition an expanded document into the validated state.
    #[must_use]
    pub fn into_validated(self) -> Document<Validated> {
        Document::new(self.body)
    }
}

impl Document<Validated> {
    /// Transition a validated document into the rendered state.
    #[must_use]
    pub fn into_rendered(self, body: String) -> Document<Rendered> {
        Document::new(body)
    }
}

/// Assemble the fixed output block order: rendered profile body, optional
/// guidance, then optional user prompt.
#[must_use]
pub fn assemble_output_blocks(
    _document: Document<Validated>,
    profile_body: &str,
    guidance_block: Option<&str>,
    user_prompt: Option<&str>,
) -> String {
    let mut blocks = vec![profile_body.trim_end().to_owned()];
    if let Some(guidance) = guidance_block.filter(|value| !value.is_empty()) {
        blocks.push(guidance.to_owned());
    }
    if let Some(prompt) = user_prompt.filter(|value| !value.is_empty()) {
        blocks.push(prompt.to_owned());
    }
    blocks.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::{Document, Parsed, assemble_output_blocks};

    #[test]
    fn output_blocks_follow_profile_guidance_prompt_order() {
        let validated = Document::<Parsed>::new("profile".to_owned())
            .into_expanded()
            .into_validated();

        let output = assemble_output_blocks(validated, "profile", Some("guidance"), Some("prompt"));

        assert_eq!(output, "profile\n\nguidance\n\nprompt");
    }
}
